//! Native, pinned Terrain Diffusion coarse-model hybrid.
//! Learned relief is baked once into the world; exploration has no inference cost.
//! The 2 GB base model and residual decoder are not used by this hybrid recipe.
mod scheduler;
use ort::{
    session::{Session, SessionInputValue},
    value::Tensor,
};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
};
use worldcraft_core::{regional::LearnedRegion, GeneratorSettings};

pub const MODEL_REVISION: &str = "ad2df557eca5645f588766101cf3bc3682455c3e";
pub const MODEL_SHA256: &str = "d6ca15b21b2e35d5e594a9ac7a4249a2376590c0ad2b5b49a1e6e2d033450008";
pub const MODEL_BYTES: u64 = 22_497_125;
const SIDE: usize = 64;
const PIXELS: usize = SIDE * SIDE;
const MEANS: [f32; 6] = [
    -37.700_01,
    1.140_306_5,
    18.102_486,
    332.834_26,
    1332.207_9,
    52.660_088,
];
const STDS: [f32; 6] = [
    39.742,
    1.768_184_4,
    8.921_469,
    321.766_02,
    842.929_4,
    31.079_985,
];
#[derive(Default)]
pub struct Progress {
    pub bytes: AtomicU64,
    pub step: AtomicUsize,
    pub cancel: AtomicBool,
}
impl Progress {
    pub fn check(&self) -> Result<(), String> {
        if self.cancel.load(Ordering::Relaxed) {
            Err("Cancelled".into())
        } else {
            Ok(())
        }
    }
}
pub fn digest_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(hash.finalize().iter().map(|b| format!("{b:02x}")).collect())
}
pub fn verify_model(path: &Path) -> Result<(), String> {
    if path.metadata().map_err(|e| e.to_string())?.len() != MODEL_BYTES
        || digest_file(path)? != MODEL_SHA256
    {
        return Err("Terrain Diffusion model size/checksum mismatch; download a clean copy".into());
    }
    Ok(())
}
pub fn model_path(folder: &Path) -> PathBuf {
    folder.join(MODEL_REVISION).join("coarse_model.onnx")
}
pub fn download(folder: &Path, progress: &Progress) -> Result<PathBuf, String> {
    let path = model_path(folder);
    if path.exists() && verify_model(&path).is_ok() {
        return Ok(path);
    }
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temporary = path.with_extension(format!("onnx.{}.{unique}.part", std::process::id()));
    let result = (|| {
        progress.check()?;
        let url=format!("https://huggingface.co/xandergos/terrain-diffusion-30m-onnx/resolve/{MODEL_REVISION}/coarse_model.onnx");
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(std::time::Duration::from_secs(600)))
            .timeout_recv_body(Some(std::time::Duration::from_secs(30)))
            .build();
        let agent: ureq::Agent = config.into();
        let mut response = agent
            .get(&url)
            .call()
            .map_err(|e| format!("Model download failed: {e}"))?;
        let mut input = response.body_mut().as_reader();
        let mut output = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|e| format!("Cannot create model download: {e}"))?;
        let mut buffer = [0u8; 65536];
        let mut count = 0u64;
        loop {
            progress.check()?;
            let n = input
                .read(&mut buffer)
                .map_err(|e| format!("Model download interrupted: {e}"))?;
            if n == 0 {
                break;
            }
            count += n as u64;
            if count > MODEL_BYTES {
                return Err("Model download exceeded the pinned size".into());
            }
            output.write_all(&buffer[..n]).map_err(|e| e.to_string())?;
            progress.bytes.store(count, Ordering::Relaxed);
        }
        output.sync_all().map_err(|e| e.to_string())?;
        drop(output);
        verify_model(&temporary)?;
        if path.exists() {
            if verify_model(&path).is_ok() {
                fs::remove_file(&temporary).map_err(|e| e.to_string())?;
                return Ok(path.clone());
            }
            fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
        fs::rename(&temporary, &path).map_err(|e| e.to_string())?;
        Ok(path.clone())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}
/// PCG XSH-RR and Marsaglia polar, matching the upstream portable stream.
pub fn normal_noise(seed: u64, length: usize) -> Vec<f32> {
    fn next(state: &mut u64) -> u32 {
        *state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let x = (((*state >> 18) ^ *state) >> 27) as u32;
        x.rotate_right((*state >> 59) as u32)
    }
    let mut state = seed;
    let mut values = Vec::with_capacity(length);
    while values.len() < length {
        let a = 2.0 * (f64::from(next(&mut state)) + 1.0) / 4294967296.0 - 1.0;
        let b = 2.0 * (f64::from(next(&mut state)) + 1.0) / 4294967296.0 - 1.0;
        let s = a * a + b * b;
        if s > 0.0 && s < 1.0 {
            let f = (-2.0 * s.ln() / s).sqrt();
            values.push((a * f) as f32);
            if values.len() < length {
                values.push((b * f) as f32);
            }
        }
    }
    values
}
fn tile_seed(seed: u64, y: i32, x: i32) -> u64 {
    seed.wrapping_mul(0x9e3779b9)
        .wrapping_add(y as u32 as u64)
        .wrapping_mul(0x9e3779b9)
        .wrapping_add(x as u32 as u64)
}
fn conditioning(settings: &GeneratorSettings) -> Vec<f32> {
    let mut out = vec![0.0; 5 * PIXELS];
    let noise = normal_noise(tile_seed(settings.seed, 0, 0), 5 * PIXELS);
    let (cos, sin) = (0.5_f32.atan().cos(), 0.5_f32.atan().sin());
    // This is explicit Worldcraft conditioning, not the reference WorldClim
    // quantile factory. It gives the chosen seed/preset and climate to the model.
    let extent = 65536.0 * settings.terrain_scale;
    for z in 0..SIDE {
        for x in 0..SIDE {
            let wx = (x as f64 - 31.5) / 47.0 * extent;
            let wz = (z as f64 - 31.5) / 47.0 * extent;
            let h = worldcraft_core::geology::raw_height(settings, wx, wz);
            let temp = settings.temperature * 45.0 - 10.0 - h.max(0.0) * 0.0065;
            let values = [
                (h.signum() * h.abs().sqrt()) as f32,
                temp as f32,
                300.0,
                (settings.rainfall * 1800.0) as f32,
                55.0,
            ];
            for (ch, index) in [0, 2, 3, 4, 5].into_iter().enumerate() {
                let i = ch * PIXELS + z * SIDE + x;
                out[i] = cos * (values[ch] - MEANS[index]) / STDS[index] + sin * noise[i];
            }
        }
    }
    out
}
// ONNX Runtime 1.31+ permits exactly one environment during process lifetime.
fn shared_environment(runtime: &Path) -> Result<ort::environment::Environment, String> {
    use std::sync::{Mutex, OnceLock};
    type Shared = Option<(PathBuf, ort::environment::Environment)>;
    static ENV: OnceLock<Mutex<Shared>> = OnceLock::new();
    let path = runtime.canonicalize().map_err(|e| e.to_string())?;
    let mut slot = ENV
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    if let Some((loaded, env)) = &*slot {
        if loaded != &path {
            return Err("Restart Worldcraft before changing the native AI runtime".into());
        }
        return Ok(env.clone());
    }
    let env = ort::init_from(&path)
        .map_err(|e| e.to_string())?
        .with_name("worldcraft-terrain")
        .with_telemetry(false)
        .build()
        .map_err(|e| e.to_string())?;
    *slot = Some((path, env.clone()));
    Ok(env)
}
pub fn generate(
    settings: &GeneratorSettings,
    model: &Path,
    runtime: &Path,
    progress: &Progress,
) -> Result<LearnedRegion, String> {
    settings.validate()?;
    progress.check()?;
    verify_model(model)?;
    if !runtime.is_file() {
        return Err("The native ONNX Runtime library is missing from this distribution".into());
    }
    let env = shared_environment(runtime)?;
    let threads = std::thread::available_parallelism()
        .map_or(1, usize::from)
        .saturating_sub(1)
        .clamp(1, 4);
    let mut builder = Session::builder(&env)
        .map_err(|e| e.to_string())?
        .with_intra_threads(threads)
        .map_err(|e| e.to_string())?
        .with_inter_threads(1)
        .map_err(|e| e.to_string())?
        .with_parallel_execution(false)
        .map_err(|e| e.to_string())?
        .with_deterministic_compute(true)
        .map_err(|e| e.to_string())?;
    let mut session = builder
        .commit_from_file(model)
        .map_err(|e| format!("Cannot open Terrain Diffusion: {e}"))?;
    let expected = [
        "x",
        "noise_labels",
        "cond_0",
        "cond_1",
        "cond_2",
        "cond_3",
        "cond_4",
    ];
    if session.inputs().len() != expected.len()
        || expected
            .iter()
            .any(|name| !session.inputs().iter().any(|i| i.name() == *name))
    {
        return Err("Pinned coarse-model input contract changed".into());
    }
    let cond = conditioning(settings);
    let mut sched = scheduler::Scheduler::new(20);
    let mut sample = normal_noise(tile_seed(settings.seed.wrapping_add(1), 0, 0), 6 * PIXELS);
    for x in &mut sample {
        *x *= sched.sigmas[0];
    }
    for step in 0..20 {
        progress.check()?;
        progress.step.store(step, Ordering::Relaxed);
        let sigma = sched.sigmas[step];
        let mut x = Vec::with_capacity(11 * PIXELS);
        let c_in = 1.0 / (sigma * sigma + 0.25).sqrt();
        x.extend(sample.iter().map(|v| v * c_in));
        x.extend_from_slice(&cond);
        let mut inputs: Vec<(String, SessionInputValue<'_>)> = vec![
            (
                "x".into(),
                Tensor::from_array(([1usize, 11, SIDE, SIDE], x))
                    .map_err(|e| e.to_string())?
                    .into(),
            ),
            (
                "noise_labels".into(),
                Tensor::from_array(([1usize], vec![(sigma / 0.5).atan()]))
                    .map_err(|e| e.to_string())?
                    .into(),
            ),
        ];
        for i in 0..5 {
            inputs.push((
                format!("cond_{i}"),
                Tensor::from_array(([1usize], vec![(0.5_f32 / 8.0).ln()]))
                    .map_err(|e| e.to_string())?
                    .into(),
            ));
        }
        let outputs = session
            .run(inputs)
            .map_err(|e| format!("Diffusion step {} failed: {e}", step + 1))?;
        let (shape, result) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| e.to_string())?;
        if **shape != [1, 6, SIDE as i64, SIDE as i64]
            || result.len() != 6 * PIXELS
            || !result.iter().all(|x| x.is_finite())
        {
            return Err("Invalid coarse-model output".into());
        }
        sample = sched.step(result, &sample);
    }
    progress.check()?;
    progress.step.store(20, Ordering::Relaxed);
    let side = 48;
    let extent = 65536.0 * settings.terrain_scale;
    let mut region = LearnedRegion {
        recipe: "terrain-diffusion-coarse-hybrid-v1".into(),
        model_revision: MODEL_REVISION.into(),
        model_sha256: MODEL_SHA256.into(),
        side,
        origin: [-extent * 0.5; 2],
        spacing: extent / (side - 1) as f64,
        blend: extent * 0.15,
        height: Vec::with_capacity(side * side),
        temperature: Vec::with_capacity(side * side),
        precipitation: Vec::with_capacity(side * side),
    };
    for z in 8..56 {
        for x in 8..56 {
            let px = z * SIDE + x;
            let value = |ch: usize| sample[ch * PIXELS + px] / 0.5 * STDS[ch] + MEANS[ch];
            let root = value(0);
            region
                .height
                .push((root.signum() * root * root).clamp(-12000.0, 9000.0));
            region.temperature.push(value(2).clamp(-80.0, 60.0));
            region.precipitation.push(value(4).clamp(0.0, 20000.0));
        }
    }
    region.validate()?;
    Ok(region)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn portable_noise_repeats_without_global_state() {
        assert_eq!(normal_noise(42, 100), normal_noise(42, 100));
        assert_ne!(normal_noise(42, 100), normal_noise(43, 100));
        let n = normal_noise(0, 100000);
        let mean = n.iter().map(|x| f64::from(*x)).sum::<f64>() / n.len() as f64;
        let variance = n.iter().map(|x| f64::from(*x).powi(2)).sum::<f64>() / n.len() as f64;
        assert!(mean.abs() < 0.02 && (variance - 1.0).abs() < 0.03);
    }
    #[test]
    fn cancellation_and_model_corruption_are_errors() {
        let p = Progress::default();
        p.cancel.store(true, Ordering::Relaxed);
        assert!(p.check().is_err());
        assert!(verify_model(Path::new("missing-model.onnx")).is_err());
    }
}
