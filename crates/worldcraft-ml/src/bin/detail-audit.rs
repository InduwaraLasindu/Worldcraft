use std::{path::PathBuf, sync::atomic::Ordering, time::Instant};
use worldcraft_core::*;
use worldcraft_ml::{assets::ModelPaths, Progress};
fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let folder = PathBuf::from(args.get(1).ok_or("model folder required")?);
    let runtime = PathBuf::from(args.get(2).ok_or("runtime library required")?);
    let paths = ModelPaths::from_coarse(&folder.join("coarse_model.onnx"));
    let progress = Progress::default();
    let settings = GeneratorSettings::for_preset(Preset::Alpine, 20261116);
    let start = Instant::now();
    let region = worldcraft_ml::generate_detailed(&settings, &paths, &runtime, &progress)?;
    let repeated = worldcraft_ml::generate_detailed(&settings, &paths, &runtime, &progress)?;
    let detail = region.detail.as_ref().ok_or("Missing detailed field")?;
    let repeat = repeated.detail.as_ref().ok_or("Missing repeated field")?;
    let error = detail
        .height
        .iter()
        .zip(&repeat.height)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0_f32, f32::max);
    assert!(error < 0.01, "Repeated detail changed by {error} m");
    let mut other = settings.clone();
    other.seed = other.seed.wrapping_add(1);
    let changed = worldcraft_ml::generate_detailed(&other, &paths, &runtime, &progress)?;
    let changed_error = detail
        .height
        .iter()
        .zip(&changed.detail.as_ref().unwrap().height)
        .map(|(a, b)| f64::from(a - b).abs())
        .sum::<f64>()
        / detail.height.len() as f64;
    assert!(
        changed_error > 1.0,
        "The seed did not change learned detail"
    );
    let min = detail.height.iter().copied().fold(f32::INFINITY, f32::min);
    let max = detail
        .height
        .iter()
        .copied()
        .fold(f32::NEG_INFINITY, f32::max);
    assert!(max - min > 50.0, "Detailed model relief was flat");
    let mut world = WorldFile::new("Detailed Terrain Diffusion audit".into(), settings);
    world.learned = Some(region);
    let generator = Generator::for_world(&world)?;
    world.camera = generator.spawn_position();
    world.camera[1] += 80.0;
    world.pitch = -0.23;
    let output = PathBuf::from("detail-example.worldcraft");
    save_world(&output, &world)?;
    let reopened = load_world(&output)?;
    assert_eq!(
        world, reopened,
        "The detailed world did not round-trip exactly"
    );
    let reloaded = Generator::for_world(&reopened)?;
    let samples = [
        [-4000.1, -3000.7],
        [-300.2, 500.3],
        [0.0, 0.0],
        [3500.0, -2500.0],
        [6000.0, 1000.0],
    ];
    for [x, z] in samples {
        assert_eq!(generator.height(x, z), reloaded.height(x, z));
    }
    let p = world.learned.as_ref().unwrap().detail.as_ref().unwrap();
    let mut seam_error = 0.0_f64;
    for axis in 0..2 {
        for edge in [p.origin[axis], p.origin[axis] + p.extent()] {
            let mut a = [100.123, -100.789];
            let mut b = a;
            a[axis] = edge - 0.001;
            b[axis] = edge + 0.001;
            seam_error =
                seam_error.max((generator.height(a[0], a[1]) - generator.height(b[0], b[1])).abs());
        }
    }
    assert!(
        seam_error < 0.01,
        "The detail/coarse boundary was discontinuous: {seam_error}"
    );
    let mut stroke = Stroke {
        center: [0.0, 0.0],
        brush: Brush::default(),
    };
    stroke.brush.strength = 5.0;
    let edited = generator.with_edits(vec![stroke.clone()])?;
    assert!((edited.height(0.0, 0.0) - generator.height(0.0, 0.0) - 5.0).abs() < 1e-6);
    world.edits = vec![vec![stroke]];
    save_world(&PathBuf::from("detail-edited.worldcraft"), &world)?;
    let restored = Generator::for_world(&load_world(&PathBuf::from("detail-edited.worldcraft"))?)?;
    assert_eq!(edited.height(0.0, 0.0), restored.height(0.0, 0.0));
    assert_eq!(
        generator.height(0.0, 0.0),
        restored.with_edits(vec![])?.height(0.0, 0.0)
    );
    let mut cancelled = Progress::default();
    cancelled.cancel.store(true, Ordering::Relaxed);
    assert!(
        worldcraft_ml::generate_detailed(&world.settings, &paths, &runtime, &cancelled).is_err()
    );
    let memory = std::fs::read_to_string("/proc/self/status")
        .unwrap_or_default()
        .lines()
        .filter(|s| s.starts_with("VmHWM") || s.starts_with("VmRSS"))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let report = serde_json::json!({"status":"passed","native_rust":true,"provider":"ONNX Runtime CPU","recipe":"terrain-diffusion-detailed-region-v1","inference_runs":3,"models_per_run":3,"coarse_steps":20,"base_steps":2,"decoder_tiles":4,"detail_side":p.side,"detail_spacing_metres":p.spacing,"detail_width_metres":p.extent(),"detail_range_metres":[min,max],"repeat_max_error_metres":error,"changed_seed_mean_absolute_error_metres":changed_error,"join_difference_over_2mm_metres":seam_error,"saved_example_bytes":output.metadata().map_err(|e|e.to_string())?.len(),"exact_round_trip":true,"edit_undo_reopen":true,"full_infinite_cascade":false,"elapsed_seconds":start.elapsed().as_secs_f64(),"memory":memory});
    std::fs::write(
        "detail-audit.json",
        serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    println!("{report}");
    Ok(())
}
