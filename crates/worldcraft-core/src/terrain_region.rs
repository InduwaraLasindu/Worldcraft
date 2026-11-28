//! Deterministic, padded terrain bakes on a world-aligned metre lattice.
use crate::{
    config::MAX_WORLD_COORD,
    erosion::thermal_relax,
    geology::{gradient, smooth},
    macro_oracle::{MacroOracle, MacroOracleError, MacroSample, MACRO_METERS_PER_PIXEL},
};
use std::fmt;

pub const REGION_SIZE_M: i64 = 1024;
pub const REGION_PADDING_M: i64 = 192;
const WORLD_LIMIT_M: i64 = MAX_WORLD_COORD as i64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RegionCoord {
    pub x: i64,
    pub z: i64,
}

impl RegionCoord {
    pub fn at(x: f64, z: f64) -> Self {
        let side = REGION_SIZE_M as f64;
        Self {
            x: (x / side).floor() as i64,
            z: (z / side).floor() as i64,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RegionConfig {
    pub spacing_m: f64,
    pub thermal_iterations: usize,
}

impl Default for RegionConfig {
    fn default() -> Self {
        Self {
            spacing_m: 1.0,
            thermal_iterations: 4,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TerrainPoint {
    pub height_m: f64,
    pub temperature_c: f64,
    pub precipitation_mm: f64,
}

#[derive(Debug)]
pub enum TerrainRegionError {
    InvalidConfig,
    OutsideWorld,
    InvalidMacroPatch,
    Macro(MacroOracleError),
}

impl fmt::Display for TerrainRegionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig => f.write_str("invalid regional sampling or thermal parameters"),
            Self::OutsideWorld => f.write_str("padded terrain region exceeds the world bounds"),
            Self::InvalidMacroPatch => f.write_str("invalid coarse terrain or climate patch"),
            Self::Macro(error) => write!(f, "macro patch failed: {error}"),
        }
    }
}

impl std::error::Error for TerrainRegionError {}

#[derive(Debug)]
pub struct TerrainRegion {
    pub coord: RegionCoord,
    pub side: usize,
    pub spacing_m: f64,
    pub origin_m: [i64; 2],
    pub active_cells: usize,
    pub padding_cells: usize,
    pub elevation_m: Vec<f32>,
    pub temperature_c: Vec<f32>,
    pub precipitation_mm: Vec<f32>,
    /// Net gain from local thermal relaxation, measured in metres.
    pub talus_deposit_m: Vec<f32>,
}

impl TerrainRegion {
    pub fn bake(
        oracle: &dyn MacroOracle,
        seed: u64,
        coord: RegionCoord,
        config: RegionConfig,
    ) -> Result<Self, TerrainRegionError> {
        let spacing_m = config.spacing_m;
        if ![0.5, 1.0, 2.0, 4.0, 8.0, 16.0].contains(&spacing_m) {
            return Err(TerrainRegionError::InvalidConfig);
        }
        let active_cells = (REGION_SIZE_M as f64 / spacing_m) as usize;
        let padding_cells = (REGION_PADDING_M as f64 / spacing_m) as usize;
        // Detail reads one neighbor beyond the footprint of thermal transport.
        if config.thermal_iterations >= padding_cells || config.thermal_iterations > 16 {
            return Err(TerrainRegionError::InvalidConfig);
        }
        let side = active_cells + 2 * padding_cells + 1;
        let padded_origin = |region: i64| {
            region
                .checked_mul(REGION_SIZE_M)
                .filter(|v| {
                    *v >= -WORLD_LIMIT_M + REGION_PADDING_M
                        && *v <= WORLD_LIMIT_M - REGION_SIZE_M - REGION_PADDING_M
                })
                .map(|v| v - REGION_PADDING_M)
                .ok_or(TerrainRegionError::OutsideWorld)
        };
        let origin_m = [padded_origin(coord.x)?, padded_origin(coord.z)?];
        let coarse_index = |m: i64| (m as f64 / MACRO_METERS_PER_PIXEL).floor() as i64;
        let coarse_min = origin_m.map(|m| coarse_index(m) - 1);
        let coarse_origin = coarse_min.map(|i| i as f64 * MACRO_METERS_PER_PIXEL);
        let width = (coarse_index(origin_m[0] + REGION_SIZE_M + 2 * REGION_PADDING_M)
            - coarse_min[0]
            + 3) as usize;
        let height = (coarse_index(origin_m[1] + REGION_SIZE_M + 2 * REGION_PADDING_M)
            - coarse_min[1]
            + 3) as usize;
        let patch = oracle
            .sample_patch(coarse_origin[0], coarse_origin[1], width, height)
            .map_err(TerrainRegionError::Macro)?;
        if patch.width != width
            || patch.height != height
            || patch.meters_per_pixel as f64 != MACRO_METERS_PER_PIXEL
            || patch.data.len() != width * height
            || patch.data.iter().any(|v| {
                !v.elevation_m.is_finite()
                    || !v.temperature_c.is_finite()
                    || !v.precipitation_mm.is_finite()
            })
        {
            return Err(TerrainRegionError::InvalidMacroPatch);
        }

        let axis = |dim: usize| {
            (0..side)
                .map(|i| {
                    let w = origin_m[dim] as f64 + i as f64 * spacing_m;
                    let m = w / MACRO_METERS_PER_PIXEL;
                    let whole = m.floor();
                    let first = (whole as i64 - coarse_min[dim] - 1) as usize;
                    (first, cubic_weights(m - whole))
                })
                .collect::<Vec<_>>()
        };
        let x_axis = axis(0);
        let z_axis = axis(1);
        let mut region = Self {
            coord,
            side,
            spacing_m,
            origin_m,
            padding_cells,
            active_cells,
            elevation_m: Vec::with_capacity(side * side),
            temperature_c: Vec::with_capacity(side * side),
            precipitation_mm: Vec::with_capacity(side * side),
            talus_deposit_m: vec![0.0; side * side],
        };
        // Resample each coarse row once along X, then apply the Z weights.
        let mut rows = Vec::with_capacity(height * side);
        for row in patch.data.chunks_exact(width) {
            for &(first, weights) in &x_axis {
                rows.push(weighted(&row[first..first + 4], weights, macro_channels));
            }
        }
        for (first, weights) in z_axis {
            for x in 0..side {
                let points = std::array::from_fn::<_, 4, _>(|i| rows[(first + i) * side + x]);
                let [height, temperature, rain] = weighted(&points, weights, |p| *p);
                region.elevation_m.push(height as f32);
                region.temperature_c.push(temperature as f32);
                region.precipitation_mm.push(rain.max(0.0) as f32);
            }
        }
        if region
            .elevation_m
            .iter()
            .chain(&region.temperature_c)
            .chain(&region.precipitation_mm)
            .any(|sample| !sample.is_finite())
        {
            return Err(TerrainRegionError::InvalidMacroPatch);
        }
        region.apply_detail(seed, config.thermal_iterations);
        Ok(region)
    }

    /// Interpolate the padded field in absolute metres, including its edge nodes.
    pub fn sample(&self, x: f64, z: f64) -> Option<TerrainPoint> {
        let gx = x / self.spacing_m;
        let gz = z / self.spacing_m;
        let [ox, oz] = self.origin_m.map(|origin| origin as f64 / self.spacing_m);
        let sx = gx - ox;
        let sz = gz - oz;
        let last = self.side.checked_sub(1)?;
        let range = 0.0..=last as f64;
        if !range.contains(&sx) || !range.contains(&sz) {
            return None;
        }
        let (fx, fz) = (gx.floor(), gz.floor());
        let (ix, iz) = ((fx - ox) as usize, (fz - oz) as usize);
        let (nx, nz) = ((ix + 1).min(last), (iz + 1).min(last));
        let (u, v) = (gx - fx, gz - fz);
        let read = |field: &[f32]| {
            let a = f64::from(field[iz * self.side + ix]);
            let b = f64::from(field[iz * self.side + nx]);
            let c = f64::from(field[nz * self.side + ix]);
            let d = f64::from(field[nz * self.side + nx]);
            let top = a + (b - a) * u;
            let bottom = c + (d - c) * u;
            top + (bottom - top) * v
        };
        Some(TerrainPoint {
            height_m: read(&self.elevation_m),
            temperature_c: read(&self.temperature_c),
            precipitation_mm: read(&self.precipitation_mm),
        })
    }

    fn apply_detail(&mut self, seed: u64, iterations: usize) {
        let mut terrain: Vec<_> = self.elevation_m.iter().map(|&h| f64::from(h)).collect();
        let side = self.side;
        let delta = self.spacing_m;
        for z in 1..side - 1 {
            let wz = self.origin_m[1] as f64 + z as f64 * delta;
            for x in 1..side - 1 {
                let i = z * side + x;
                let h = f64::from(self.elevation_m[i]);
                let dx = f64::from(self.elevation_m[i + 1]) - f64::from(self.elevation_m[i - 1]);
                let dz =
                    f64::from(self.elevation_m[i + side]) - f64::from(self.elevation_m[i - side]);
                let slope = (dx * dx + dz * dz).sqrt() / (2.0 * delta);
                let wx = self.origin_m[0] as f64 + x as f64 * delta;
                let rain = f64::from(self.precipitation_mm[i]);
                let cliffs = smooth(0.25, 0.92, slope);
                if cliffs > 0.0 {
                    let fold = gradient(seed ^ 0x4f7a, wx / 96.0, wz / 92.0) * 12.0;
                    let fracture = gradient(seed ^ 0xf301, (wx + fold) / 14.0, (wz - fold) / 17.0);
                    let bed = (h + fold + wx * 0.11 + wz * 0.072) / 3.6;
                    let ridge = 1.0 - fracture.abs();
                    terrain[i] += cliffs * (1.05 * bed.sin() + 1.45 * (ridge * ridge - 0.55));
                }
                if rain < 250.0 && slope < 0.36 && h > 3.0 {
                    let wind = gradient(seed ^ 0x71ca, wx / 126.0, wz / 151.0);
                    let crest = (wx * 0.082 + wz * 0.053 + wind * 6.4).sin();
                    let dry =
                        (1.0 - smooth(110.0, 250.0, rain)) * (1.0 - smooth(0.16, 0.36, slope));
                    terrain[i] += dry * (3.8 * crest.max(0.0).powi(3) - 0.52);
                }
            }
        }
        for (height, &detail) in self.elevation_m.iter_mut().zip(&terrain) {
            *height = detail as f32;
        }
        if iterations != 0 {
            thermal_relax(&mut terrain, side, delta, iterations);
            for ((height, deposit), relaxed) in self
                .elevation_m
                .iter_mut()
                .zip(&mut self.talus_deposit_m)
                .zip(terrain)
            {
                *deposit = (relaxed as f32 - *height).max(0.0);
                *height = relaxed as f32;
            }
        }
    }
}

fn macro_channels(sample: &MacroSample) -> [f64; 3] {
    [
        sample.elevation_m,
        sample.temperature_c,
        sample.precipitation_mm,
    ]
    .map(f64::from)
}

fn cubic_weights(t: f64) -> [f64; 4] {
    let square = t * t;
    let cube = square * t;
    [
        -0.5 * t + square - 0.5 * cube,
        1.0 - 2.5 * square + 1.5 * cube,
        0.5 * t + 2.0 * square - 1.5 * cube,
        -0.5 * square + 0.5 * cube,
    ]
}

fn weighted<T>(points: &[T], weights: [f64; 4], channels: impl Fn(&T) -> [f64; 3]) -> [f64; 3] {
    let mut out = [0.0; 3];
    for (point, weight) in points.iter().zip(weights) {
        let values = channels(point);
        for i in 0..3 {
            out[i] += values[i] * weight;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::macro_oracle::MacroPatch;

    struct TestOracle(fn(f64, f64) -> [f32; 3]);

    impl MacroOracle for TestOracle {
        fn sample_patch(
            &self,
            x: f64,
            z: f64,
            width: usize,
            height: usize,
        ) -> Result<MacroPatch, MacroOracleError> {
            assert_eq!(x % MACRO_METERS_PER_PIXEL, 0.0);
            assert_eq!(z % MACRO_METERS_PER_PIXEL, 0.0);
            let data = (0..height)
                .flat_map(|iz| {
                    (0..width).map(move |ix| {
                        let [elevation_m, temperature_c, precipitation_mm] = (self.0)(
                            x + ix as f64 * MACRO_METERS_PER_PIXEL,
                            z + iz as f64 * MACRO_METERS_PER_PIXEL,
                        );
                        MacroSample {
                            elevation_m,
                            temperature_c,
                            precipitation_mm,
                            ..Default::default()
                        }
                    })
                })
                .collect();
            Ok(MacroPatch {
                width,
                height,
                meters_per_pixel: MACRO_METERS_PER_PIXEL as f32,
                data,
            })
        }
    }

    #[test]
    fn meter_grid_recovers_a_plane_from_the_global_coarse_samples() {
        let source = TestOracle(|x, z| {
            [
                (250.0 + x * 0.125 + z * 0.0625) as f32,
                12.5,
                (920.0 + x * 0.0625 - z * 0.125) as f32,
            ]
        });
        let coord = RegionCoord::at(-18.0, 0.0);
        assert_eq!(coord, RegionCoord { x: -1, z: 0 });
        let region = TerrainRegion::bake(&source, 37, coord, RegionConfig::default()).unwrap();
        assert_eq!(
            [region.active_cells, region.padding_cells, region.side],
            [1024, 192, 1409]
        );
        assert_eq!(region.origin_m, [-1216, -192]);
        assert_eq!(
            region.sample(-18.125, 28.25),
            Some(TerrainPoint {
                height_m: 249.5,
                temperature_c: 12.5,
                precipitation_mm: 915.3359375,
            })
        );
        assert!(region.sample(-1216.1, 0.0).is_none());
        assert!(region.sample(192.0, 1216.0).is_some());
        assert!(region.talus_deposit_m.iter().all(|&value| value == 0.0));
    }

    #[test]
    fn seeded_detail_and_talus_agree_across_independent_region_borders() {
        let source = TestOracle(|x, z| {
            [
                (320.0 + 216.0 * (x / 105.0).sin() * (z / 108.0).cos()) as f32,
                (9.0 + 4.5 * (z / 230.0).cos()) as f32,
                (390.0 + 260.0 * (x / 174.0).cos()) as f32,
            ]
        });
        let config = RegionConfig {
            spacing_m: 4.0,
            ..Default::default()
        };
        let west = TerrainRegion::bake(&source, 37, RegionCoord { x: -1, z: -1 }, config).unwrap();
        let east = TerrainRegion::bake(&source, 37, RegionCoord { x: 0, z: -1 }, config).unwrap();
        let south = TerrainRegion::bake(&source, 37, RegionCoord { x: -1, z: 0 }, config).unwrap();
        assert!(west.talus_deposit_m.iter().any(|&value| value > 0.01));
        for along in (-1024..=0).step_by(11) {
            let position = along as f64 + 0.317;
            for cross in [-12.3, -0.125, 0.0, 0.3, 12.1] {
                assert_eq!(west.sample(cross, position), east.sample(cross, position));
                assert_eq!(west.sample(position, cross), south.sample(position, cross));
            }
        }
    }
}
