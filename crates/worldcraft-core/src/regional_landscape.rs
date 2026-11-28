//! Seeded geomorphology shared by exploration and offline tools.
use crate::{
    config::REGIONAL_GENERATOR_VERSION,
    macro_oracle::{MacroOracleError, TectonicClimateOracle},
    region_cache::{RegionCache, RegionCacheKey},
    terrain_region::{RegionConfig, RegionCoord, TerrainPoint, TerrainRegion, TerrainRegionError},
    GeneratorSettings,
};
use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard, OnceLock},
};

const RESIDENT_REGIONS: usize = 4;

pub struct RegionalLandscape {
    settings: GeneratorSettings,
    oracle: TectonicClimateOracle,
    disk: OnceLock<RegionCache>,
    resident: Mutex<VecDeque<(RegionCoord, Arc<TerrainRegion>)>>,
    bake_lock: Mutex<()>,
}

impl std::fmt::Debug for RegionalLandscape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RegionalLandscape")
            .field("seed", &self.settings.seed)
            .finish_non_exhaustive()
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|error| error.into_inner())
}

impl RegionalLandscape {
    pub fn new(settings: &GeneratorSettings) -> Result<Self, MacroOracleError> {
        Ok(Self {
            settings: settings.clone(),
            oracle: TectonicClimateOracle::with_settings(settings)?,
            disk: OnceLock::new(),
            resident: Mutex::new(VecDeque::new()),
            bake_lock: Mutex::new(()),
        })
    }

    pub fn set_cache_dir(&self, directory: PathBuf) {
        let _ = self.disk.set(RegionCache::new(directory));
    }

    fn cached(&self, coord: RegionCoord) -> Option<Arc<TerrainRegion>> {
        let mut regions = lock(&self.resident);
        let index = regions.iter().position(|(key, _)| *key == coord)?;
        let entry = regions.remove(index)?;
        let region = Arc::clone(&entry.1);
        regions.push_front(entry);
        Some(region)
    }

    pub fn region(&self, coord: RegionCoord) -> Result<Arc<TerrainRegion>, TerrainRegionError> {
        if let Some(region) = self.cached(coord) {
            return Ok(region);
        }
        // A bake or disk I/O must never hold the lock used by movement queries.
        let _single_bake = lock(&self.bake_lock);
        if let Some(region) = self.cached(coord) {
            return Ok(region);
        }
        let config = RegionConfig::default();
        let key = RegionCacheKey {
            seed: self.settings.seed,
            settings_hash: self.settings.fingerprint(),
            generator_version: REGIONAL_GENERATOR_VERSION,
            coord,
        };
        let disk = self.disk.get();
        let stored = disk
            .and_then(|cache| cache.load(key))
            .filter(|region| region.spacing_m == config.spacing_m);
        let baked = match stored {
            Some(region) => region,
            None => {
                let region = TerrainRegion::bake(&self.oracle, self.settings.seed, coord, config)?;
                if let Some(cache) = disk {
                    // A read-only or full cache must never stop exploration.
                    let _ = cache.store(key, &region);
                }
                region
            }
        };
        let region = Arc::new(baked);
        let mut resident = lock(&self.resident);
        resident.truncate(RESIDENT_REGIONS - 1);
        resident.push_front((coord, Arc::clone(&region)));
        Ok(region)
    }

    pub fn estimated_elevation(&self, x: f64, z: f64) -> f64 {
        f64::from(self.oracle.elevation_at(x, z)) + self.settings.sea_level
    }

    pub fn preview(&self, x: f64, z: f64) -> TerrainPoint {
        let point = self.oracle.sample_at(x, z);
        TerrainPoint {
            height_m: f64::from(point.elevation_m) + self.settings.sea_level,
            temperature_c: f64::from(point.temperature_c),
            precipitation_mm: f64::from(point.precipitation_mm),
        }
    }

    pub fn sample(&self, x: f64, z: f64) -> TerrainPoint {
        self.region(RegionCoord::at(x, z))
            .ok()
            .and_then(|region| region.sample(x, z))
            .map(|point| self.absolute_height(point))
            .unwrap_or_else(|| self.preview(x, z))
    }

    pub fn resident_sample(&self, x: f64, z: f64) -> Option<TerrainPoint> {
        let coord = RegionCoord::at(x, z);
        self.cached(coord)?
            .sample(x, z)
            .map(|point| self.absolute_height(point))
    }

    fn absolute_height(&self, mut point: TerrainPoint) -> TerrainPoint {
        point.height_m += self.settings.sea_level;
        point
    }
}
