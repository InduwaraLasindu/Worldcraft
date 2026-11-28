//! Populate the same .wcr cache used by the Godot client before exploring.
use std::{env, path::PathBuf, process::ExitCode, thread};
use worldcraft_core::{
    config::{seed_from_text, MAX_WORLD_COORD, REGIONAL_GENERATOR_VERSION},
    hydrology::drain_region,
    macro_oracle::TectonicClimateOracle,
    region_cache::{RegionCache, RegionCacheKey},
    terrain_region::{RegionConfig, RegionCoord, TerrainRegion, REGION_PADDING_M, REGION_SIZE_M},
    GeneratorSettings, Preset,
};

fn number<T: std::str::FromStr>(text: &str, field: &str) -> Result<T, String> {
    text.parse().map_err(|_| format!("Invalid {field}"))
}

fn bake(args: &[String]) -> Result<(), String> {
    const USAGE: &str =
        "Usage: worldcraft-bake CACHE_DIR SEED PRESET_INDEX REGION_X REGION_Z RADIUS [--drainage]";
    let [directory, seed, preset, x, z, radius, options @ ..] = args else {
        return Err(USAGE.into());
    };
    let drainage = match options {
        [] => false,
        [flag] if flag == "--drainage" => true,
        _ => return Err(USAGE.into()),
    };
    let preset = Preset::ALL
        .get(number::<usize>(preset, "preset index")?)
        .ok_or("Invalid preset index")?;
    let (x, z) = (number::<i64>(x, "region X")?, number::<i64>(z, "region Z")?);
    let radius = number::<i64>(radius, "radius")?;
    let region_limit = (MAX_WORLD_COORD as i64 - REGION_SIZE_M - REGION_PADDING_M) / REGION_SIZE_M;
    if !(0..=8).contains(&radius)
        || x.unsigned_abs().max(z.unsigned_abs()) > (region_limit - radius) as u64
    {
        return Err("Radius must be 0–8 and the region must be within the world bounds".into());
    }
    if *preset == Preset::Empty {
        return Err("Empty canvas does not use regional bakes".into());
    }
    let settings = GeneratorSettings::for_preset(*preset, seed_from_text(seed));
    let oracle = TectonicClimateOracle::with_settings(&settings).map_err(|e| e.to_string())?;
    let cache = RegionCache::new(PathBuf::from(directory));
    let coordinates: Vec<_> = (z - radius..=z + radius)
        .flat_map(|z| (x - radius..=x + radius).map(move |x| RegionCoord { x, z }))
        .collect();
    let workers = thread::available_parallelism()
        .map_or(1, usize::from)
        .min(2);
    let coordinates = &coordinates;
    let settings = &settings;
    let cache = &cache;
    let oracle = &oracle;
    thread::scope(|scope| {
        let handles: Vec<_> = (0..workers)
            .map(|worker| {
                scope.spawn(move || -> Result<(), String> {
                    for &coord in coordinates.iter().skip(worker).step_by(workers) {
                        let key = RegionCacheKey {
                            seed: settings.seed,
                            settings_hash: settings.fingerprint(),
                            generator_version: REGIONAL_GENERATOR_VERSION,
                            coord,
                        };
                        let region = match cache.load(key).filter(|region| region.spacing_m == 1.0)
                        {
                            Some(region) => region,
                            None => {
                                let region = TerrainRegion::bake(
                                    oracle,
                                    settings.seed,
                                    coord,
                                    RegionConfig::default(),
                                )
                                .map_err(|e| e.to_string())?;
                                cache.store(key, &region).map_err(|e| e.to_string())?;
                                region
                            }
                        };
                        if drainage {
                            let water = drain_region(
                                region.side,
                                region.side,
                                region.spacing_m,
                                0.0,
                                &region.elevation_m,
                                &region.precipitation_mm,
                                Some(&region.temperature_c),
                            )?;
                            let frozen = water.lakes.iter().filter(|lake| lake.frozen).count();
                            println!(
                                "Region {}, {}: {} padded-grid depressions ({} frozen)",
                                coord.x,
                                coord.z,
                                water.lakes.len(),
                                frozen
                            );
                        }
                    }
                    Ok(())
                })
            })
            .collect();
        for handle in handles {
            handle.join().map_err(|_| "Region bake worker stopped")??;
        }
        Ok::<(), String>(())
    })?;
    println!("WORLDCRAFT_BAKE_OK {} region(s)", coordinates.len());
    Ok(())
}

fn main() -> ExitCode {
    match bake(&env::args().skip(1).collect::<Vec<_>>()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
