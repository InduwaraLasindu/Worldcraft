use crate::{config::Preset, edit::BrushKind, GeneratorSettings, Stroke};
use crate::{
    config::GENERATOR_VERSION,
    geology::{self, CacheStats, TerrainCache},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum Biome {
    Ocean,
    Beach,
    #[default]
    Grassland,
    Forest,
    Rainforest,
    Conifer,
    Desert,
    Tundra,
    Snow,
    Volcanic,
    Savanna,
    Wetland,
    Meadow,
    BirchForest,
}
impl Biome {
    pub const ALL: [Self; 14] = [
        Self::Ocean,
        Self::Beach,
        Self::Grassland,
        Self::Forest,
        Self::Rainforest,
        Self::Conifer,
        Self::Desert,
        Self::Tundra,
        Self::Snow,
        Self::Volcanic,
        Self::Savanna,
        Self::Wetland,
        Self::Meadow,
        Self::BirchForest,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Ocean => "Ocean",
            Self::Beach => "Beach",
            Self::Grassland => "Grassland",
            Self::Forest => "Temperate forest",
            Self::Rainforest => "Rainforest",
            Self::Conifer => "Conifer forest",
            Self::Desert => "Desert",
            Self::Tundra => "Tundra",
            Self::Snow => "Snow",
            Self::Volcanic => "Volcanic",
            Self::Savanna => "Savanna",
            Self::Wetland => "Wet woodland",
            Self::Meadow => "Mountain meadow",
            Self::BirchForest => "Birch woodland",
        }
    }
    pub fn color(self) -> [f32; 3] {
        match self {
            Self::Ocean => [0.09, 0.16, 0.17],
            Self::Beach => [0.55, 0.45, 0.28],
            Self::Grassland => [0.18, 0.27, 0.065],
            Self::Forest => [0.105, 0.19, 0.055],
            Self::Rainforest => [0.055, 0.17, 0.06],
            Self::Conifer => [0.12, 0.19, 0.1],
            Self::Desert => [0.60, 0.34, 0.16],
            Self::Tundra => [0.31, 0.32, 0.23],
            Self::Snow => [0.82, 0.86, 0.89],
            Self::Volcanic => [0.11, 0.10, 0.09],
            Self::Savanna => [0.32, 0.31, 0.095],
            Self::Wetland => [0.085, 0.19, 0.095],
            Self::Meadow => [0.17, 0.31, 0.075],
            Self::BirchForest => [0.16, 0.27, 0.07],
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    pub height: f64,
    pub temperature: f64,
    pub moisture: f64,
    pub biome: Biome,
    pub vegetation: f64,
}
const EDIT_CELL: f64 = 256.0;
#[derive(Clone, Debug)]
pub struct Generator {
    settings: GeneratorSettings,
    edits: Vec<Stroke>,
    version: u32,
    index: HashMap<(i64, i64), Vec<usize>>,
    cache: Arc<TerrainCache>,
    regional: Option<Arc<crate::regional::LearnedRegion>>,
}
impl Generator {
    pub fn new(settings: GeneratorSettings, edits: Vec<Stroke>) -> Self {
        Self::for_version(settings, edits, GENERATOR_VERSION).expect("valid generation inputs")
    }
    pub fn for_version(
        settings: GeneratorSettings,
        edits: Vec<Stroke>,
        version: u32,
    ) -> Result<Self, String> {
        if !(1..=GENERATOR_VERSION).contains(&version) {
            return Err(format!("Unsupported terrain generator version {version}"));
        }
        settings.validate()?;
        let index = Self::index_edits(&edits)?;
        Ok(Self {
            settings,
            edits,
            version,
            index,
            cache: Arc::new(TerrainCache::default()),
            regional: None,
        })
    }
    pub fn for_world(world: &crate::WorldFile) -> Result<Self, String> {
        world.validate()?;
        let mut g = Self::for_version(
            world.settings.clone(),
            world.edits.iter().flatten().cloned().collect(),
            world.generator_version,
        )?;
        g.regional = world.learned.clone().map(Arc::new);
        Ok(g)
    }
    fn mix_regional(&self, x: f64, z: f64, classical: impl FnOnce() -> f64) -> f64 {
        if let Some(patch) = &self.regional {
            if let Some([height, _, _, w]) = patch.sample(x, z) {
                let detail = geology::fbm(self.settings.seed ^ 0x483, x / 128.0, z / 128.0, 4)
                    * 12.0
                    * patch.fine_detail_scale(x, z);
                if w >= 1.0 {
                    return height + detail;
                }
                return classical() * (1.0 - w) + (height + detail) * w;
            }
        }
        classical()
    }
    fn index_edits(edits: &[Stroke]) -> Result<HashMap<(i64, i64), Vec<usize>>, String> {
        let mut index: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
        for (i, stroke) in edits.iter().enumerate() {
            stroke.validate()?;
            let [x, z] = stroke.center;
            let r = stroke.brush.radius;
            let a = (
                ((x - r) / EDIT_CELL).floor() as i64,
                ((z - r) / EDIT_CELL).floor() as i64,
            );
            let b = (
                ((x + r) / EDIT_CELL).floor() as i64,
                ((z + r) / EDIT_CELL).floor() as i64,
            );
            for cz in a.1..=b.1 {
                for cx in a.0..=b.0 {
                    index.entry((cx, cz)).or_default().push(i);
                }
            }
        }
        Ok(index)
    }
    fn local_edits(&self, x: f64, z: f64) -> impl Iterator<Item = &Stroke> {
        self.index
            .get(&(
                (x / EDIT_CELL).floor() as i64,
                (z / EDIT_CELL).floor() as i64,
            ))
            .into_iter()
            .flatten()
            .map(|&i| &self.edits[i])
    }
    pub fn settings(&self) -> &GeneratorSettings {
        &self.settings
    }
    pub fn edits(&self) -> &[Stroke] {
        &self.edits
    }
    pub fn version(&self) -> u32 {
        self.version
    }
    pub fn cache_stats(&self) -> CacheStats {
        self.cache.stats()
    }
    /// Immutable edit snapshots share the settings-specific, bounded terrain cache.
    pub fn with_edits(&self, edits: Vec<Stroke>) -> Result<Self, String> {
        let index = Self::index_edits(&edits)?;
        Ok(Self {
            settings: self.settings.clone(),
            edits,
            version: self.version,
            index,
            cache: Arc::clone(&self.cache),
            regional: self.regional.clone(),
        })
    }
    pub fn base_height(&self, x: f64, z: f64) -> f64 {
        if self.version == 1 {
            self.legacy_base_height(x, z)
        } else if self.settings.preset == Preset::Empty {
            0.0
        } else {
            self.mix_regional(x, z, || self.cache.height(&self.settings, x, z))
        }
    }
    fn noise(&self, x: f64, z: f64, salt: u64) -> f64 {
        noise(self.settings.seed ^ salt, x, z)
    }
    fn legacy_base_height(&self, x: f64, z: f64) -> f64 {
        let s = &self.settings;
        if s.preset == Preset::Empty {
            return 0.0;
        }
        let x = x / s.terrain_scale;
        let z = z / s.terrain_scale;
        let wx = x + self.noise(x / 1700.0, z / 1700.0, 10) * 260.0;
        let wz = z + self.noise(x / 1700.0, z / 1700.0, 20) * 260.0;
        let continent = self.noise(wx / 6000.0, wz / 6000.0, 30);
        let foothill = fbm(s.seed ^ 40, wx / 650.0, wz / 650.0, 4);
        let ridge = 1.0 - self.noise(wx / 1200.0, wz / 1200.0, 50).abs();
        let range_mask = smooth(0.1, 0.85, self.noise(wx / 3200.0, wz / 3200.0, 60));
        let mountains = ridge.powi(3) * range_mask * 650.0 * s.mountain_strength;
        // Continuous analytic valley masks; this is not a watershed/flow simulation.
        let channel = self.noise(wx / 850.0, wz / 850.0, 70).abs();
        let valley = (1.0 - smooth(0.0, 0.075, channel)) * 30.0 * s.river_strength;
        let detail = fbm(s.seed ^ 80, wx / 90.0, wz / 90.0, 3) * 6.0;
        let mut h = 65.0 + continent * 180.0 + foothill * 70.0 + mountains + detail - valley;
        if s.preset == Preset::Desert {
            let terrace = (h / 22.0).floor() * 22.0;
            h = h * 0.45 + terrace * 0.55;
        }
        if s.preset == Preset::Islands || s.preset == Preset::Volcanic {
            h -= 65.0;
        }
        h * s.vertical_scale
    }
    pub fn height(&self, x: f64, z: f64) -> f64 {
        let mut h = self.base_height(x, z);
        for edit in self.local_edits(x, z) {
            let w = edit.weight(x, z);
            if w > 0.0 {
                h = edit.height(h, w);
            }
        }
        h
    }
    pub fn slope(&self, x: f64, z: f64) -> f64 {
        let dx = (self.height(x + 1.0, z) - self.height(x - 1.0, z)) * 0.5;
        let dz = (self.height(x, z + 1.0) - self.height(x, z - 1.0)) * 0.5;
        dx.hypot(dz)
    }
    pub fn normal(&self, x: f64, z: f64) -> [f32; 3] {
        let dx = (self.height(x + 1.0, z) - self.height(x - 1.0, z)) * 0.5;
        let dz = (self.height(x, z + 1.0) - self.height(x, z - 1.0)) * 0.5;
        let l = (dx * dx + 1.0 + dz * dz).sqrt();
        [(-dx / l) as f32, (1.0 / l) as f32, (-dz / l) as f32]
    }
    pub fn sample(&self, x: f64, z: f64) -> Sample {
        self.classify(x, z, self.height(x, z))
    }
    /// Fast regional preview: analytic geology plus durable edits and climate.
    /// This intentionally omits erosion; broad map views must not churn the
    /// bounded exploration cache by baking hundreds of distant terrain tiles.
    pub fn preview_sample(&self, x: f64, z: f64) -> Sample {
        let mut h = if self.version == 1 {
            self.legacy_base_height(x, z)
        } else {
            self.mix_regional(x, z, || geology::raw_height(&self.settings, x, z))
        };
        for edit in self.local_edits(x, z) {
            let w = edit.weight(x, z);
            if w > 0.0 {
                h = edit.height(h, w);
            }
        }
        self.classify(x, z, h)
    }
    fn classify(&self, x: f64, z: f64, h: f64) -> Sample {
        let s = &self.settings;
        let (cx, cz) = if self.version >= 3 {
            let scale = s.terrain_scale;
            let x = x / scale;
            let z = z / scale;
            (
                x + geology::gradient(s.seed ^ 0xb101, x / 3400.0, z / 3400.0) * 420.0,
                z + geology::gradient(s.seed ^ 0xb102, x / 3400.0, z / 3400.0) * 420.0,
            )
        } else {
            (x, z)
        };
        let climate_noise = |x: f64, z: f64, salt: u64| {
            if self.version == 1 {
                self.noise(x, z, salt)
            } else {
                if self.version >= 3 {
                    geology::gradient(s.seed ^ salt, x, z) * 0.55
                        + noise(s.seed ^ (salt + 431), x * 0.57, z * 0.57) * 0.45
                } else {
                    geology::gradient(s.seed ^ salt, x, z)
                }
            }
        };
        let lapse = if self.version == 1 { 0.00065 } else { 0.00045 };
        let diversity = |v: f64| 0.25 + 0.75 * (1.0 - (v - 0.5).abs() * 2.0);
        let temp_amplitude = if self.version >= 3 {
            0.85 * diversity(s.temperature)
        } else {
            0.22
        };
        let wet_amplitude = if self.version >= 3 {
            1.10 * diversity(s.rainfall)
        } else {
            0.32
        };
        let mut temperature = (s.temperature
            + climate_noise(cx / 2600.0, cz / 2600.0, 90) * temp_amplitude
            - (h - s.sea_level).max(0.0) * lapse)
            .clamp(0.0, 1.0);
        let wet_scale = if self.version >= 3 { 1800.0 } else { 1200.0 };
        let mut moisture = (s.rainfall
            + climate_noise(cx / wet_scale, cz / wet_scale, 100) * wet_amplitude)
            .clamp(0.0, 1.0);
        if let Some(patch) = &self.regional {
            if let Some([_, temp, precip, w]) = patch.sample(x, z) {
                temperature = temperature * (1.0 - w) + ((temp + 10.0) / 45.0).clamp(0.0, 1.0) * w;
                moisture = moisture * (1.0 - w) + (precip / 1800.0).clamp(0.0, 1.0) * w;
            }
        }
        let mut biome = if h < s.sea_level {
            Biome::Ocean
        } else if h < s.sea_level + 3.5 {
            Biome::Beach
        } else if temperature < 0.12 {
            Biome::Snow
        } else if s.preset == Preset::Volcanic && h > 110.0 {
            Biome::Volcanic
        } else if temperature < 0.25 {
            Biome::Tundra
        } else if moisture < 0.22 {
            Biome::Desert
        } else if temperature < 0.43 && moisture > 0.4 {
            Biome::Conifer
        } else if moisture > 0.72 && temperature > 0.7 {
            Biome::Rainforest
        } else if moisture > 0.48 {
            Biome::Forest
        } else {
            Biome::Grassland
        };
        let mut blend = (self.version >= 3).then(|| {
            crate::ecology::BiomeBlend::climate(
                h - s.sea_level,
                temperature,
                moisture,
                s.preset,
                self.stand_phase(x, z),
            )
        });
        let mut vegetation = s.vegetation;
        for e in self.local_edits(x, z) {
            let w = e.weight(x, z);
            if e.brush.kind == BrushKind::Biome {
                if let Some(blend) = &mut blend {
                    blend.painted(e.brush.biome, w);
                } else if w > 0.4 {
                    biome = e.brush.biome;
                }
            }
            if e.brush.kind == BrushKind::Vegetation {
                vegetation += (e.brush.density - vegetation) * w;
            }
        }
        if let Some(blend) = blend {
            biome = blend.dominant();
        }
        Sample {
            height: h,
            temperature,
            moisture,
            biome,
            vegetation,
        }
    }
    pub fn color(&self, x: f64, z: f64) -> [f32; 4] {
        let s = self.sample(x, z);
        let base = if self.version >= 3 {
            self.biome_blend(x, z, &s).color()
        } else {
            s.biome.color()
        };
        let rock = [0.29, 0.27, 0.23];
        if self.version >= 2 {
            let slope = self.slope(x, z);
            let rock_mix = smooth(0.55, 1.15, slope) as f32;
            let patch = geology::gradient(self.settings.seed ^ 313, x / 32.0, z / 32.0) as f32;
            let soil = match s.biome {
                Biome::Desert | Biome::Beach => [0.50, 0.37, 0.20],
                Biome::Snow => [0.72, 0.78, 0.82],
                Biome::Volcanic => [0.08, 0.075, 0.065],
                _ => [0.22, 0.17, 0.09],
            };
            let dry = ((0.38 - s.moisture) * 1.1).clamp(0.0, 0.35) as f32;
            let soil_mix = ((patch - 0.14) * 0.35 + dry).clamp(0.0, 0.4);
            let variation = 0.96 + patch * 0.12;
            let mut c = [1.0; 4];
            for i in 0..3 {
                let grass = base[i] * (1.0 - soil_mix) + soil[i] * soil_mix;
                c[i] = (grass * (1.0 - rock_mix) + rock[i] * rock_mix) * variation;
            }
            return c;
        }
        let rock_mix = smooth(0.45, 1.25, self.slope(x, z)) as f32;
        let variation = 0.88 + self.noise(x / 18.0, z / 18.0, 110) as f32 * 0.12;
        let mut c = [1.0; 4];
        for i in 0..3 {
            c[i] = (base[i] * (1.0 - rock_mix) + rock[i] * rock_mix) * variation;
        }
        c
    }
    fn stand_phase(&self, x: f64, z: f64) -> f64 {
        0.5 + geology::gradient(self.settings.seed ^ 0xb10b, x / 640.0, z / 640.0) * 0.8
    }
    /// Renderer-independent ecotone weights. Legacy generations retain their
    /// original discrete habitats; explicit biome paint blends in saved order.
    pub fn biome_blend(&self, x: f64, z: f64, sample: &Sample) -> crate::ecology::BiomeBlend {
        use crate::ecology::BiomeBlend;
        if self.version < 3 {
            return BiomeBlend::single(sample.biome);
        }
        let mut blend = BiomeBlend::climate(
            sample.height - self.settings.sea_level,
            sample.temperature,
            sample.moisture,
            self.settings.preset,
            self.stand_phase(x, z),
        );
        for edit in self.local_edits(x, z) {
            if edit.brush.kind == BrushKind::Biome {
                blend.painted(edit.brush.biome, edit.weight(x, z));
            }
        }
        blend
    }
    /// Search a small deterministic region so first launch never spawns underwater.
    pub fn spawn_position(&self) -> [f64; 3] {
        for ring in 0i32..=18 {
            for z in -ring..=ring {
                for x in -ring..=ring {
                    if ring > 0 && x.abs() != ring && z.abs() != ring {
                        continue;
                    }
                    let (wx, wz) = (f64::from(x) * 64.0, f64::from(z) * 64.0);
                    // Cheap analytic rejection avoids baking flooded regions during spawn search.
                    if self.version >= 2
                        && self.regional.is_none()
                        && geology::raw_height(&self.settings, wx, wz)
                            < self.settings.sea_level + 3.0
                    {
                        continue;
                    }
                    let h = self.height(wx, wz);
                    if h > self.settings.sea_level + 5.0 && self.slope(wx, wz) < 0.6 {
                        return [wx, h + 1.75, wz];
                    }
                }
            }
        }
        [
            0.0,
            self.height(0.0, 0.0).max(self.settings.sea_level) + 3.0,
            0.0,
        ]
    }
}
pub fn hash(seed: u64, x: i64, z: i64) -> u64 {
    let mut v = seed
        ^ (x as u64).wrapping_mul(0x9e3779b97f4a7c15)
        ^ (z as u64).wrapping_mul(0xbf58476d1ce4e5b9);
    v = (v ^ (v >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    v = (v ^ (v >> 27)).wrapping_mul(0x94d049bb133111eb);
    v ^ (v >> 31)
}
pub fn unit(seed: u64, x: i64, z: i64) -> f64 {
    (hash(seed, x, z) >> 11) as f64 / 9_007_199_254_740_992.0
}
pub fn noise(seed: u64, x: f64, z: f64) -> f64 {
    let ix = x.floor() as i64;
    let iz = z.floor() as i64;
    let sx = fade(x - x.floor());
    let sz = fade(z - z.floor());
    let a = unit(seed, ix, iz) * 2.0 - 1.0;
    let b = unit(seed, ix + 1, iz) * 2.0 - 1.0;
    let c = unit(seed, ix, iz + 1) * 2.0 - 1.0;
    let d = unit(seed, ix + 1, iz + 1) * 2.0 - 1.0;
    (a + (b - a) * sx) * (1.0 - sz) + (c + (d - c) * sx) * sz
}
fn fbm(seed: u64, mut x: f64, mut z: f64, octaves: u32) -> f64 {
    let (mut result, mut amplitude, mut total) = (0.0, 1.0, 0.0);
    for i in 0..octaves {
        result += noise(seed.wrapping_add(u64::from(i) * 7919), x, z) * amplitude;
        total += amplitude;
        amplitude *= 0.5;
        x *= 2.03;
        z *= 2.03;
    }
    result / total
}
fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}
fn smooth(a: f64, b: f64, v: f64) -> f64 {
    let t = ((v - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
