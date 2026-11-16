//! A finite, offline three-model bake; it does not implement infinite learned streaming.
use crate::{
    assets::{ModelPaths, BASE, DECODER},
    detail_math, Progress, MEANS, MODEL_REVISION, MODEL_SHA256, PIXELS, SIDE, STDS,
};
use ort::{
    session::{Session, SessionInputValue},
    value::Tensor,
};
use std::{path::Path, sync::atomic::Ordering};
use worldcraft_core::{
    regional::{LearnedDetail, LearnedRegion},
    GeneratorSettings,
};
const STEPS: usize = 27;
fn open_model(
    path: &Path,
    runtime: &Path,
    names: &[&str],
    progress: &Progress,
) -> Result<Session, String> {
    let env = crate::shared_environment(runtime)?;
    let threads = progress.threads();
    let mut builder = Session::builder(&env)
        .map_err(|e| e.to_string())?
        .with_intra_threads(threads)
        .map_err(|e| e.to_string())?
        .with_config_entry("session.intra_op.allow_spinning", "0")
        .map_err(|e| e.to_string())?
        .with_config_entry("session.inter_op.allow_spinning", "0")
        .map_err(|e| e.to_string())?
        .with_inter_threads(1)
        .map_err(|e| e.to_string())?
        .with_parallel_execution(false)
        .map_err(|e| e.to_string())?
        .with_deterministic_compute(true)
        .map_err(|e| e.to_string())?;
    let session = builder
        .commit_from_file(path)
        .map_err(|e| format!("Cannot load detailed terrain model: {e}"))?;
    if session.inputs().len() != names.len()
        || names
            .iter()
            .any(|n| !session.inputs().iter().any(|i| i.name() == *n))
    {
        return Err("Pinned detailed-model input contract changed".into());
    }
    Ok(session)
}
fn coarse_condition(settings: &GeneratorSettings) -> Vec<f32> {
    let mut out = vec![0.0; 5 * PIXELS];
    let noise = detail_math::noise_patch(settings.seed, -32, -32, SIDE, SIDE, 5, 64);
    let t = 0.5_f32.atan();
    let spacing = 7680.0 * settings.terrain_scale;
    for y in 0..SIDE {
        for x in 0..SIDE {
            let h = worldcraft_core::geology::raw_height(
                settings,
                (x as f64 - 31.5) * spacing,
                (y as f64 - 31.5) * spacing,
            );
            let values = [
                (h.signum() * h.abs().sqrt()) as f32,
                (settings.temperature * 45.0 - 10.0 - h.max(0.0) * 0.0065) as f32,
                300.0,
                (settings.rainfall * 1800.0) as f32,
                55.0,
            ];
            for (ch, index) in [0, 2, 3, 4, 5].into_iter().enumerate() {
                let i = ch * PIXELS + y * SIDE + x;
                out[i] = t.cos() * (values[ch] - MEANS[index]) / STDS[index] + t.sin() * noise[i];
            }
        }
    }
    out
}
fn latent_condition(coarse: &[f32]) -> Vec<f32> {
    let means = [14.99, 11.65, 15.87, 619.26, 833.12, 69.40, 0.66];
    let stds = [21.72, 21.78, 10.40, 452.29, 738.09, 34.59, 0.47];
    let mut normalized = vec![0.0_f32; 6 * 16];
    for ch in 0..6 {
        for y in 0..4 {
            for x in 0..4 {
                let p = (30 + y) * SIDE + 30 + x;
                let mut v = coarse[ch * PIXELS + p] / 0.5 * STDS[ch] + MEANS[ch];
                if ch == 1 {
                    v = coarse[p] / 0.5 * STDS[0] + MEANS[0] - v;
                }
                normalized[ch * 16 + y * 4 + x] = (v - means[ch]) / stds[ch];
            }
        }
    }
    let mut groups = vec![normalized[..16].to_vec(), normalized[16..32].to_vec()];
    groups.push(
        (2..6)
            .map(|ch| {
                [5, 6, 9, 10]
                    .iter()
                    .map(|&p| normalized[ch * 16 + p])
                    .sum::<f32>()
                    / 4.0
            })
            .collect(),
    );
    groups.push(vec![(1.0 - means[6]) / stds[6]; 16]);
    groups.push(vec![0.0; 5]);
    groups.push(vec![-0.5 * 12.0_f32.sqrt()]);
    let factor = 348.0_f32.sqrt() / 6.0;
    groups
        .into_iter()
        .flat_map(|g| {
            let scale = factor / (g.len() as f32).sqrt();
            g.into_iter().map(move |v| v * scale)
        })
        .collect()
}
fn refine(
    settings: &GeneratorSettings,
    condition: &[f32],
    path: &Path,
    runtime: &Path,
    progress: &Progress,
) -> Result<Vec<f32>, String> {
    progress.check()?;
    progress.stage.store(5, Ordering::Relaxed);
    let mut session = open_model(path, runtime, &["x", "noise_labels", "cond_0"], progress)?;
    let mut sample = vec![0.0_f32; 5 * PIXELS];
    for (step, t) in [160.0_f32.atan(), 0.7_f32.atan()].into_iter().enumerate() {
        progress.check()?;
        progress.stage.store(6, Ordering::Relaxed);
        let noise = detail_math::noise_patch(
            settings.seed.wrapping_add(5819 + step as u64),
            -32,
            -32,
            SIDE,
            SIDE,
            5,
            64,
        );
        let xt: Vec<f32> = sample
            .iter()
            .zip(noise)
            .map(|(v, n)| t.cos() * v + t.sin() * n)
            .collect();
        let inputs: Vec<(String, SessionInputValue<'_>)> = vec![
            (
                "x".into(),
                Tensor::from_array(([1usize, 5, SIDE, SIDE], xt.clone()))
                    .map_err(|e| e.to_string())?
                    .into(),
            ),
            (
                "noise_labels".into(),
                Tensor::from_array(([1usize], vec![t]))
                    .map_err(|e| e.to_string())?
                    .into(),
            ),
            (
                "cond_0".into(),
                Tensor::from_array(([1usize, 58], condition.to_vec()))
                    .map_err(|e| e.to_string())?
                    .into(),
            ),
        ];
        let output = session
            .run(inputs)
            .map_err(|e| format!("Detailed terrain refinement failed: {e}"))?;
        let (shape, data) = output[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| e.to_string())?;
        if **shape != [1, 5, 64, 64]
            || data.len() != 5 * PIXELS
            || !data.iter().all(|v| v.is_finite())
        {
            return Err("Invalid base-model output".into());
        }
        sample = xt
            .iter()
            .zip(data)
            .map(|(x, p)| t.cos() * x + t.sin() * p)
            .collect();
        progress.step.store(21 + step, Ordering::Relaxed);
        progress.pause()?;
    }
    progress.check()?;
    Ok(sample)
}
fn decode(
    settings: &GeneratorSettings,
    latent: &[f32],
    path: &Path,
    runtime: &Path,
    progress: &Progress,
) -> Result<Vec<f32>, String> {
    progress.check()?;
    progress.stage.store(7, Ordering::Relaxed);
    let mut session = open_model(path, runtime, &["x", "noise_labels"], progress)?;
    let mut residual = vec![0.0_f32; 448 * 448];
    let mut weights = residual.clone();
    let t = 160.0_f32.atan();
    let mut tile_index = 0;
    for oy in [4_usize, 28] {
        for ox in [4_usize, 28] {
            progress.check()?;
            progress.stage.store(8, Ordering::Relaxed);
            let noise = detail_math::noise_patch(
                settings.seed.wrapping_add(5819),
                (oy as i32 - 32) * 8,
                (ox as i32 - 32) * 8,
                256,
                256,
                1,
                256,
            );
            let mut input = Vec::with_capacity(5 * 256 * 256);
            input.extend(noise.iter().map(|n| t.sin() * n));
            for ch in 0..4 {
                for y in 0..256 {
                    for x in 0..256 {
                        input.push(latent[(ch * 64 + oy + y / 8) * 64 + ox + x / 8]);
                    }
                }
            }
            let inputs: Vec<(String, SessionInputValue<'_>)> = vec![
                (
                    "x".into(),
                    Tensor::from_array(([1usize, 5, 256, 256], input))
                        .map_err(|e| e.to_string())?
                        .into(),
                ),
                (
                    "noise_labels".into(),
                    Tensor::from_array(([1usize], vec![t]))
                        .map_err(|e| e.to_string())?
                        .into(),
                ),
            ];
            let output = session
                .run(inputs)
                .map_err(|e| format!("Detailed terrain decoding failed: {e}"))?;
            let (shape, data) = output[0]
                .try_extract_tensor::<f32>()
                .map_err(|e| e.to_string())?;
            if **shape != [1, 1, 256, 256]
                || data.len() != 256 * 256
                || !data.iter().all(|v| v.is_finite())
            {
                return Err("Invalid decoder output".into());
            }
            for y in 0..256 {
                for x in 0..256 {
                    let p = y * 256 + x;
                    let q = ((oy - 4) * 8 + y) * 448 + (ox - 4) * 8 + x;
                    let w = detail_math::blend_weight(x, y, 256);
                    residual[q] += (t.cos() * t.sin() * noise[p] + t.sin() * data[p]) * 0.7 * w;
                    weights[q] += w;
                }
            }
            progress.pause()?;
            tile_index += 1;
            progress.step.store(22 + tile_index, Ordering::Relaxed);
        }
    }
    for (v, w) in residual.iter_mut().zip(weights) {
        *v /= w;
    }
    progress.check()?;
    Ok(residual)
}
pub fn generate_detailed(
    settings: &GeneratorSettings,
    paths: &ModelPaths,
    runtime: &Path,
    progress: &Progress,
) -> Result<LearnedRegion, String> {
    settings.validate()?;
    progress.check()?;
    if settings.preset == worldcraft_core::Preset::Empty {
        return Err("Empty creates a flat canvas; choose a landscape preset for AI terrain".into());
    }
    progress.total_steps.store(STEPS, Ordering::Relaxed);
    progress.step.store(0, Ordering::Relaxed);
    paths.verify(progress)?;
    // Each function drops its session before the next model loads.
    let coarse = crate::infer_coarse(
        settings,
        &paths.coarse,
        runtime,
        progress,
        Some(coarse_condition(settings)),
        Some(detail_math::noise_patch(
            settings.seed.wrapping_add(1),
            -32,
            -32,
            SIDE,
            SIDE,
            6,
            64,
        )),
    )?;
    let latent = refine(
        settings,
        &latent_condition(&coarse),
        &paths.base,
        runtime,
        progress,
    )?;
    let residual = decode(settings, &latent, &paths.decoder, runtime, progress)?;
    progress.stage.store(9, Ordering::Relaxed);
    progress.check()?;
    let low: Vec<f32> = (4..60)
        .flat_map(|y| (4..60).map(move |x| (y, x)))
        .map(|(y, x)| latent[4 * PIXELS + y * 64 + x] * 38.6 - 31.4)
        .collect();
    let full = detail_math::reconstruct(&residual, &low, 56);
    if full.iter().any(|v| !v.is_finite() || v.abs() > 20000.0) {
        return Err("Detailed terrain exceeded the supported physical height range".into());
    }
    let height: Vec<f32> = (64..384)
        .flat_map(|y| full[y * 448 + 64..y * 448 + 384].iter().copied())
        .collect();
    let spacing = 30.0 * settings.terrain_scale;
    let detail = LearnedDetail {
        base_model_sha256: BASE.sha256.into(),
        decoder_model_sha256: DECODER.sha256.into(),
        side: 320,
        origin: [-159.5 * spacing; 2],
        spacing,
        blend: 32.0 * spacing,
        height,
    };
    let spacing = 7680.0 * settings.terrain_scale;
    let mut region = LearnedRegion {
        detail: Some(detail),
        recipe: "terrain-diffusion-detailed-region-v1".into(),
        model_revision: MODEL_REVISION.into(),
        model_sha256: MODEL_SHA256.into(),
        side: 48,
        origin: [-23.5 * spacing; 2],
        spacing,
        blend: 47.0 * spacing * 0.15,
        height: Vec::with_capacity(48 * 48),
        temperature: Vec::with_capacity(48 * 48),
        precipitation: Vec::with_capacity(48 * 48),
    };
    for y in 8..56 {
        for x in 8..56 {
            let p = y * 64 + x;
            let value = |ch: usize| coarse[ch * PIXELS + p] / 0.5 * STDS[ch] + MEANS[ch];
            let root = value(0);
            region
                .height
                .push((root.signum() * root * root).clamp(-12000.0, 9000.0));
            region.temperature.push(value(2).clamp(-80.0, 60.0));
            region.precipitation.push(value(4).clamp(0.0, 20000.0));
        }
    }
    region.validate()?;
    progress.check()?;
    progress.step.store(STEPS, Ordering::Relaxed);
    progress.stage.store(10, Ordering::Relaxed);
    Ok(region)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn latent_condition_has_the_reference_group_normalization() {
        let coarse = vec![0.0_f32; 6 * PIXELS];
        let c = latent_condition(&coarse);
        assert_eq!(c.len(), 58);
        assert!(c.iter().all(|v| v.is_finite()));
        assert!((c[0] - ((MEANS[0] - 14.99) / 21.72) * (348.0_f32.sqrt() / 24.0)).abs() < 1e-5);
        assert!(
            (c[16] - ((MEANS[0] - MEANS[1] - 11.65) / 21.78) * (348.0_f32.sqrt() / 24.0)).abs()
                < 1e-5
        );
        assert_eq!(&c[52..57], &[0.0; 5]);
        assert!((c[57] + 0.5 * 12.0_f32.sqrt() * 348.0_f32.sqrt() / 6.0).abs() < 1e-5);
    }
}
