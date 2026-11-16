use std::{path::PathBuf, time::Instant};
use worldcraft_core::*;
fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let runtime = PathBuf::from(args.get(1).ok_or("runtime library required")?);
    let folder = PathBuf::from(args.get(2).ok_or("model folder required")?);
    let output = PathBuf::from(args.get(3).ok_or("world output required")?);
    let progress = worldcraft_ml::Progress::default();
    let model = worldcraft_ml::download(&folder, &progress)?;
    let settings = GeneratorSettings::for_preset(Preset::Alpine, 20261116);
    let start = Instant::now();
    let region = worldcraft_ml::generate(&settings, &model, &runtime, &progress)?;
    let repeated = worldcraft_ml::generate(&settings, &model, &runtime, &progress)?;
    let max_error = region
        .height
        .iter()
        .zip(&repeated.height)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0_f32, f32::max);
    assert!(
        max_error < 0.01,
        "Repeated native inference changed the relief"
    );
    let mut other = settings.clone();
    other.seed += 1;
    let changed = worldcraft_ml::generate(&other, &model, &runtime, &progress)?;
    assert_ne!(
        region.height, changed.height,
        "The seed did not change learned relief"
    );
    let min = region.height.iter().copied().fold(f32::INFINITY, f32::min);
    let max = region
        .height
        .iter()
        .copied()
        .fold(f32::NEG_INFINITY, f32::max);
    assert!(max - min > 50.0, "Model relief was flat");
    let mut world = WorldFile::new("Terrain Diffusion hybrid audit".into(), settings);
    world.learned = Some(region);
    let g = Generator::for_world(&world)?;
    world.camera = g.spawn_position();
    save_world(&output, &world)?;
    let loaded = load_world(&output)?;
    assert!(
        world == loaded,
        "The saved world did not round-trip exactly"
    );
    assert_eq!(
        g.height(0.0, 0.0),
        Generator::for_world(&loaded)?.height(0.0, 0.0)
    );
    println!(
        "{}",
        serde_json::json!({"status":"passed","provider":"ONNX Runtime CPU","model_revision":worldcraft_ml::MODEL_REVISION,"model_sha256":worldcraft_ml::MODEL_SHA256,"inference_runs":3,"repeat_max_error_metres":max_error,"range_metres":[min,max],"elapsed_seconds":start.elapsed().as_secs_f64(),"patch_side":48,"native_rust":true,"full_cascade":false})
    );
    Ok(())
}
