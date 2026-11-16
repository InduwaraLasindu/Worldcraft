//! Version 2 analytic geology and halo-safe thermal erosion tiles.
use crate::{
    erosion::thermal_relax,
    generation::{hash, unit},
    GeneratorSettings, Preset,
};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, OnceLock,
    },
};

pub const TILE_SIZE: f64 = 512.0;
pub const SPACING: f64 = 8.0;
pub const CELLS: usize = 64;
pub const ITERATIONS: usize = 8;
// A flux update has a two-cell dependency radius. The extra two cells protect
// the shared edge from finite simulation-boundary conditions.
const HALO: usize = 2 * ITERATIONS + 2;
const CAPACITY: usize = 32;
type Coord = (i64, i64);
type Slot = Arc<OnceLock<Arc<Tile>>>;

#[derive(Debug)]
pub struct Tile {
    pub heights: Vec<f64>,
}
impl Tile {
    pub fn sample(&self, x: f64, z: f64) -> f64 {
        let u = (x / SPACING).clamp(0.0, CELLS as f64);
        let v = (z / SPACING).clamp(0.0, CELLS as f64);
        let ix = (u.floor() as usize).min(CELLS - 1);
        let iz = (v.floor() as usize).min(CELLS - 1);
        let tx = u - ix as f64;
        let tz = v - iz as f64;
        let row = CELLS + 1;
        let a = iz * row + ix;
        let top = self.heights[a] * (1.0 - tx) + self.heights[a + 1] * tx;
        let bottom = self.heights[a + row] * (1.0 - tx) + self.heights[a + row + 1] * tx;
        top * (1.0 - tz) + bottom * tz
    }
}
#[derive(Debug, Default)]
struct Entries {
    clock: u64,
    tiles: HashMap<Coord, (Slot, u64)>,
}
#[derive(Debug, Default)]
pub struct TerrainCache {
    entries: Mutex<Entries>,
    bakes: AtomicUsize,
    hits: AtomicUsize,
}
#[derive(Clone, Copy, Debug)]
pub struct CacheStats {
    pub tiles: usize,
    pub bakes: usize,
    pub hits: usize,
}
impl TerrainCache {
    pub fn height(&self, s: &GeneratorSettings, x: f64, z: f64) -> f64 {
        let c = (
            (x / TILE_SIZE).floor() as i64,
            (z / TILE_SIZE).floor() as i64,
        );
        let slot = {
            let mut e = self.entries.lock().unwrap_or_else(|p| p.into_inner());
            e.clock = e.clock.wrapping_add(1);
            let stamp = e.clock;
            if let Some((slot, last)) = e.tiles.get_mut(&c) {
                *last = stamp;
                self.hits.fetch_add(1, Ordering::Relaxed);
                Arc::clone(slot)
            } else {
                if e.tiles.len() >= CAPACITY {
                    let oldest = e
                        .tiles
                        .iter()
                        .min_by_key(|(_, (_, stamp))| *stamp)
                        .map(|(c, _)| *c)
                        .unwrap();
                    e.tiles.remove(&oldest);
                }
                let slot = Arc::new(OnceLock::new());
                e.tiles.insert(c, (Arc::clone(&slot), stamp));
                slot
            }
        };
        let tile = slot.get_or_init(|| {
            self.bakes.fetch_add(1, Ordering::Relaxed);
            Arc::new(bake(s, c))
        });
        tile.sample(x - c.0 as f64 * TILE_SIZE, z - c.1 as f64 * TILE_SIZE)
    }
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            tiles: self
                .entries
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .tiles
                .len(),
            bakes: self.bakes.load(Ordering::Relaxed),
            hits: self.hits.load(Ordering::Relaxed),
        }
    }
}
pub fn bake(s: &GeneratorSettings, c: Coord) -> Tile {
    let n = CELLS + 1 + HALO * 2;
    let ox = c.0 * CELLS as i64 - HALO as i64;
    let oz = c.1 * CELLS as i64 - HALO as i64;
    let mut h = Vec::with_capacity(n * n);
    for z in 0..n {
        for x in 0..n {
            h.push(raw_height(
                s,
                (ox + x as i64) as f64 * SPACING,
                (oz + z as i64) as f64 * SPACING,
            ));
        }
    }
    thermal_relax(&mut h, n, SPACING, ITERATIONS);
    let mut heights = Vec::with_capacity((CELLS + 1) * (CELLS + 1));
    for z in 0..=CELLS {
        for x in 0..=CELLS {
            heights.push(h[(z + HALO) * n + x + HALO]);
        }
    }
    Tile { heights }
}

pub fn raw_height(s: &GeneratorSettings, x: f64, z: f64) -> f64 {
    if s.preset == Preset::Empty {
        return 0.0;
    }
    let x = x / s.terrain_scale;
    let z = z / s.terrain_scale;
    let seed = s.seed;
    let wx = x + gradient(seed ^ 11, x / 2700.0, z / 2700.0) * 350.0;
    let wz = z + gradient(seed ^ 29, x / 2700.0, z / 2700.0) * 350.0;
    let continent = fbm(seed ^ 41, wx / 10_000.0, wz / 10_000.0, 3);
    let belts = smooth(-0.35, 0.65, fbm(seed ^ 57, wx / 4700.0, wz / 4700.0, 3));
    let ridges = ridged(seed ^ 73, wx / 1500.0, wz / 1500.0);
    let hills = fbm(seed ^ 97, wx / 550.0, wz / 550.0, 5);
    let detail = fbm(seed ^ 113, wx / 100.0, wz / 100.0, 3) * 7.0;
    let channel = gradient(seed ^ 127, wx / 1700.0, wz / 1700.0).abs();
    let valley = (1.0 - smooth(0.015, 0.09, channel)) * 26.0 * s.river_strength;
    let mut h = 72.0
        + continent * 210.0
        + hills * 60.0
        + ridges * belts * 430.0 * s.mountain_strength
        + detail
        - valley;
    match s.preset {
        Preset::Alpine => {
            h = 145.0
                + continent * 150.0
                + hills * 80.0
                + ridges * belts * 530.0 * s.mountain_strength
                + detail
                - valley;
        }
        Preset::Desert => {
            let mesa = smooth(-0.15, 0.45, fbm(seed ^ 139, wx / 1200.0, wz / 1200.0, 3));
            h = 70.0 + continent * 140.0 + mesa * 170.0 + hills * 20.0;
            let level = h / 26.0;
            h = (level.floor() + smooth(0.2, 0.8, level - level.floor())) * 26.0;
            h -= valley * 2.5;
            h += detail * 0.4;
        }
        Preset::Islands | Preset::Volcanic => {
            let cell = 4200.0;
            let cx = (x / cell).floor() as i64;
            let cz = (z / cell).floor() as i64;
            let mut island: f64 = 0.0;
            for dz in -1..=1 {
                for dx in -1..=1 {
                    let gx = cx + dx;
                    let gz = cz + dz;
                    let px = (gx as f64 + 0.15 + unit(seed ^ 151, gx, gz) * 0.7) * cell;
                    let pz = (gz as f64 + 0.15 + unit(seed ^ 163, gx, gz) * 0.7) * cell;
                    let r = 1250.0 + unit(seed ^ 179, gx, gz) * 850.0;
                    let d = (x - px).hypot(z - pz) / r;
                    let dome = (1.0 - smooth(0.05, 1.15, d)).powf(1.5);
                    let mut feature = dome * (280.0 + ridges * 200.0 * s.mountain_strength);
                    if s.preset == Preset::Volcanic {
                        let cone = (1.0 - d).max(0.0);
                        let crater = (1.0 - smooth(0.0, 0.14, d)) * 210.0;
                        feature = cone * 600.0 * s.mountain_strength - crater;
                    }
                    island = island.max(feature);
                }
            }
            h = -95.0 + island + hills * 15.0 + detail * 0.7;
        }
        Preset::Savanna => {
            h = 65.0
                + continent * 170.0
                + hills * 35.0
                + ridges * belts * 170.0 * s.mountain_strength
                + detail
                - valley;
        }
        _ => {}
    }
    h * s.vertical_scale
}
pub fn gradient(seed: u64, x: f64, z: f64) -> f64 {
    // Noninteger offsets avoid a seed-independent flat point at the world origin.
    let x = x + 19.731;
    let z = z - 71.193;
    let ix = x.floor() as i64;
    let iz = z.floor() as i64;
    let fx = x - x.floor();
    let fz = z - z.floor();
    let dot = |dx: i64, dz: i64| {
        let (gx, gz) = match hash(seed, ix + dx, iz + dz) & 7 {
            0 => (1.0, 0.0),
            1 => (-1.0, 0.0),
            2 => (0.0, 1.0),
            3 => (0.0, -1.0),
            4 => (
                std::f64::consts::FRAC_1_SQRT_2,
                std::f64::consts::FRAC_1_SQRT_2,
            ),
            5 => (
                -std::f64::consts::FRAC_1_SQRT_2,
                std::f64::consts::FRAC_1_SQRT_2,
            ),
            6 => (
                std::f64::consts::FRAC_1_SQRT_2,
                -std::f64::consts::FRAC_1_SQRT_2,
            ),
            _ => (
                -std::f64::consts::FRAC_1_SQRT_2,
                -std::f64::consts::FRAC_1_SQRT_2,
            ),
        };
        gx * (fx - dx as f64) + gz * (fz - dz as f64)
    };
    let u = fade(fx);
    let v = fade(fz);
    let a = dot(0, 0) * (1.0 - u) + dot(1, 0) * u;
    let b = dot(0, 1) * (1.0 - u) + dot(1, 1) * u;
    ((a * (1.0 - v) + b * v) * 1.42).clamp(-1.0, 1.0)
}
pub(crate) fn fbm(seed: u64, mut x: f64, mut z: f64, octaves: u32) -> f64 {
    let (mut sum, mut amp, mut weight) = (0.0, 1.0, 0.0);
    for i in 0..octaves {
        sum += gradient(seed.wrapping_add(i as u64 * 7919), x, z) * amp;
        weight += amp;
        amp *= 0.52;
        x *= 2.01;
        z *= 2.01;
    }
    sum / weight
}
fn ridged(seed: u64, mut x: f64, mut z: f64) -> f64 {
    let (mut sum, mut amp, mut weight, mut previous) = (0.0, 1.0, 0.0, 1.0);
    for i in 0..5 {
        let ridge = (1.0 - gradient(seed.wrapping_add(i * 3571), x, z).abs()).powi(2);
        sum += ridge * previous * amp;
        weight += amp;
        previous = (ridge * 1.7).clamp(0.0, 1.0);
        amp *= 0.5;
        x *= 2.03;
        z *= 2.03;
    }
    sum / weight
}
fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}
fn smooth(a: f64, b: f64, v: f64) -> f64 {
    let t = ((v - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn neighboring_eroded_tiles_share_identical_edges() {
        let s = GeneratorSettings::for_preset(Preset::Alpine, 114);
        let a = bake(&s, (-1, -1));
        let b = bake(&s, (0, -1));
        let c = bake(&s, (-1, 0));
        for i in 0..=CELLS {
            assert_eq!(
                a.heights[i * (CELLS + 1) + CELLS],
                b.heights[i * (CELLS + 1)]
            );
            assert_eq!(a.heights[CELLS * (CELLS + 1) + i], c.heights[i]);
        }
    }
    #[test]
    fn cache_is_bounded_and_eviction_does_not_change_results() {
        let s = GeneratorSettings::default();
        let cache = TerrainCache::default();
        let expected = cache.height(&s, -91.25, 87.75);
        for i in 0..40 {
            cache.height(&s, i as f64 * TILE_SIZE + 10.0, 10.0);
        }
        assert!(cache.stats().tiles <= CAPACITY);
        assert_eq!(expected, cache.height(&s, -91.25, 87.75));
    }
    #[test]
    fn shared_region_bakes_once_across_threads() {
        let s = GeneratorSettings::default();
        let cache = Arc::new(TerrainCache::default());
        std::thread::scope(|scope| {
            for i in 0..4 {
                let c = Arc::clone(&cache);
                let s = &s;
                scope.spawn(move || c.height(s, 10.0 + i as f64, 10.0));
            }
        });
        assert_eq!(cache.stats().bakes, 1);
    }
    #[test]
    fn preset_landforms_are_distinct_and_have_relief() {
        for preset in [
            Preset::Temperate,
            Preset::Alpine,
            Preset::Desert,
            Preset::Islands,
            Preset::Volcanic,
        ] {
            let s = GeneratorSettings::for_preset(preset, 20261116);
            let mut min = f64::INFINITY;
            let mut max = f64::NEG_INFINITY;
            for z in -8..=8 {
                for x in -8..=8 {
                    let h = raw_height(&s, x as f64 * 256.0, z as f64 * 256.0);
                    min = min.min(h);
                    max = max.max(h);
                }
            }
            assert!(
                max - min > 70.0,
                "{preset:?} has insufficient geographic variation: {}",
                max - min
            );
        }
    }
    #[test]
    fn erosion_actually_changes_steep_geology() {
        let s = GeneratorSettings::for_preset(Preset::Volcanic, 9001);
        let mut change: f64 = 0.0;
        // Cover multiple islands rather than assuming a particular tile contains a cone.
        for cz in 0..6 {
            for cx in 0..6 {
                let t = bake(&s, (cx, cz));
                for z in 0..=CELLS {
                    for x in 0..=CELLS {
                        let raw = raw_height(
                            &s,
                            cx as f64 * TILE_SIZE + x as f64 * SPACING,
                            cz as f64 * TILE_SIZE + z as f64 * SPACING,
                        );
                        change = change.max((raw - t.heights[z * (CELLS + 1) + x]).abs());
                    }
                }
                if change > 1e-6 {
                    break;
                }
            }
            if change > 1e-6 {
                break;
            }
        }
        assert!(change > 1e-6);
    }
}
