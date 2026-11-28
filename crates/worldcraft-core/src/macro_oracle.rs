//! Random-access world-scale elevation and climate, independent of the renderer.
use crate::{
    config::MAX_WORLD_COORD,
    generation::unit,
    geology::{fbm, gradient, smooth},
    GeneratorSettings, Preset,
};
use std::{error::Error, f64::consts::TAU, fmt};

pub const MACRO_METERS_PER_PIXEL: f64 = 30.0;
pub const MAX_MACRO_SAMPLES: usize = 65_536;
const PLATE_SPACING_M: f64 = 32_000.0;
const UPWIND_METERS: [f64; 6] = [56_000.0, 34_000.0, 19_000.0, 10_000.0, 4_500.0, 1_400.0];

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MacroSample {
    /// Elevation in metres relative to sea level.
    pub elevation_m: f32,
    pub temperature_c: f32,
    pub temp_std_c: f32,
    pub precipitation_mm: f32,
    pub precip_cv: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MacroPatch {
    pub width: usize,
    pub height: usize,
    pub meters_per_pixel: f32,
    /// Samples are ordered by increasing x within each row of increasing z.
    pub data: Vec<MacroSample>,
}

impl MacroPatch {
    pub fn get(&self, x: usize, z: usize) -> Option<MacroSample> {
        if x >= self.width || z >= self.height {
            return None;
        }
        self.data
            .get(z.checked_mul(self.width)?.checked_add(x)?)
            .copied()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MacroOracleError {
    EmptyPatch,
    PatchTooLarge,
    CoordinateOutOfRange,
    InvalidSettings(String),
}

impl fmt::Display for MacroOracleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPatch => f.write_str("macro patch dimensions must be positive"),
            Self::PatchTooLarge => write!(f, "macro patch exceeds {MAX_MACRO_SAMPLES} samples"),
            Self::CoordinateOutOfRange => f.write_str("macro patch is outside the supported world"),
            Self::InvalidSettings(reason) => f.write_str(reason),
        }
    }
}

impl Error for MacroOracleError {}

pub trait MacroOracle: Send + Sync {
    /// Samples start at the requested world origin and advance 30 m along each axis.
    fn sample_patch(
        &self,
        world_origin_x_m: f64,
        world_origin_z_m: f64,
        width_px: usize,
        height_px: usize,
    ) -> Result<MacroPatch, MacroOracleError>;
}

#[derive(Clone, Debug)]
pub struct TectonicClimateOracle {
    settings: GeneratorSettings,
    wind: [f64; 2],
}

#[derive(Clone, Copy, Default)]
struct Plate {
    site: [f64; 2],
    velocity: [f64; 2],
    distance: f64,
    crust_m: f64,
}

impl TectonicClimateOracle {
    pub fn new(seed: u64) -> Self {
        Self::from_validated(GeneratorSettings::for_preset(Preset::Temperate, seed))
    }

    pub fn with_settings(settings: &GeneratorSettings) -> Result<Self, MacroOracleError> {
        settings
            .validate()
            .map_err(MacroOracleError::InvalidSettings)?;
        Ok(Self::from_validated(settings.clone()))
    }

    fn from_validated(settings: GeneratorSettings) -> Self {
        let azimuth = TAU * unit(settings.seed ^ 0x77a7_12a9, 0, 0);
        Self {
            settings,
            wind: [azimuth.cos(), azimuth.sin()],
        }
    }

    /// Elevation alone for finite world coordinates, without the climate ray.
    pub fn elevation_at(&self, x: f64, z: f64) -> f32 {
        self.elevation(x, z) as f32
    }

    /// A point sample for finite world coordinates. Use `sample_patch` for checked bounds.
    pub fn sample_at(&self, x: f64, z: f64) -> MacroSample {
        let settings = &self.settings;
        let seed = settings.seed;
        let elevation_m = self.elevation_at(x, z);
        let land_m = f64::from(elevation_m).max(0.0);
        let latitude = (0.48 + z * TAU / 40_000_000.0).sin().abs();
        let temperature_c = 28.0 - 54.0 * latitude.powf(1.65)
            + 46.0 * (settings.temperature - 0.55)
            + 2.4 * gradient(seed ^ 0x149f, x / 48_000.0, z / 48_000.0)
            - 0.0065 * land_m;

        let mut moisture = 0.36 + 0.46 * settings.rainfall;
        let mut previous = None::<f64>;
        for distance in UPWIND_METERS {
            let upstream = self.elevation(x - self.wind[0] * distance, z - self.wind[1] * distance);
            if let Some(height) = previous {
                moisture *= (-(upstream.max(0.0) - height.max(0.0)).max(0.0) / 1800.0).exp();
            }
            let open_water = 1.0 - smooth(-125.0, 160.0, upstream);
            moisture += (1.0 - moisture) * open_water * 0.48;
            previous = Some(upstream);
        }
        let ascent_m = (land_m - previous.unwrap_or_default().max(0.0)).max(0.0);
        let rain_band = 1.0 + 0.44 * fbm(seed ^ 0x4aad, x / 14_000.0, z / 14_000.0, 3);
        let orographic = 3_200.0 * (1.0 - (-ascent_m / 850.0).exp());
        let precipitation_mm = (moisture
            * ((90.0 + settings.rainfall * 1_490.0) * rain_band + orographic))
            .clamp(12.0, 8_000.0);
        let inland = smooth(-100.0, 550.0, f64::from(elevation_m));

        MacroSample {
            elevation_m,
            temperature_c: temperature_c as f32,
            temp_std_c: (2.5 + (3.0 + 12.0 * latitude) * (0.42 + inland * 0.58)) as f32,
            precipitation_mm: precipitation_mm as f32,
            precip_cv: (0.23 + 0.68 * (1.0 - (precipitation_mm / 2_100.0).min(1.0))) as f32,
        }
    }

    fn elevation(&self, x: f64, z: f64) -> f64 {
        let settings = &self.settings;
        if settings.preset == Preset::Empty {
            return -settings.sea_level;
        }
        let seed = settings.seed;
        let x = x / settings.terrain_scale;
        let z = z / settings.terrain_scale;
        let px = (x + 1700.0 * gradient(seed ^ 0x3819, x / 8400.0, z / 8400.0)) / PLATE_SPACING_M;
        let pz = (z + 1700.0 * gradient(seed ^ 0x50aa, x / 8400.0, z / 8400.0)) / PLATE_SPACING_M;
        let cx = px.floor() as i64;
        let cz = pz.floor() as i64;

        // Sites remain within the central half of their cells, so nearby faults
        // and their crust blend are covered by the surrounding nine sites.
        let mut plates = [Plate::default(); 9];
        let mut nearest = f64::INFINITY;
        for (index, plate) in plates.iter_mut().enumerate() {
            let ix = cx + index as i64 % 3 - 1;
            let iz = cz + index as i64 / 3 - 1;
            let random = |salt| unit(seed ^ salt, ix, iz);
            plate.site = [
                ix as f64 + 0.25 + 0.5 * random(0xf451),
                iz as f64 + 0.25 + 0.5 * random(0x019f),
            ];
            plate.distance = (px - plate.site[0]).hypot(pz - plate.site[1]);
            nearest = nearest.min(plate.distance);
            plate.crust_m = if random(0xc2d4) < 0.62 {
                165.0 + 535.0 * random(0x70e3)
            } else {
                -680.0 - 2900.0 * random(0x70e3)
            };
            plate.velocity = [2.0 * random(0xfc38) - 1.0, 2.0 * random(0x79db) - 1.0];
        }

        let proximity = plates.map(|p| 1.0 - smooth(0.0, 0.17, p.distance - nearest));
        let mut crust = 0.0;
        let mut closing_fault: f64 = 0.0;
        for (i, &a) in plates.iter().enumerate() {
            let weight = proximity[i];
            crust += weight * a.crust_m;
            for (j, &b) in plates.iter().enumerate().skip(i + 1) {
                let pair = weight * proximity[j];
                if pair <= 0.0 {
                    continue;
                }
                let dx = b.site[0] - a.site[0];
                let dz = b.site[1] - a.site[1];
                let separation = dx.hypot(dz);
                let closing = ((a.velocity[0] - b.velocity[0]) * dx
                    + (a.velocity[1] - b.velocity[1]) * dz)
                    / separation;
                let signed_fault = (a.distance.powi(2) - b.distance.powi(2)) / (2.0 * separation);
                // The shallower plate carries the wider hanging-wall slope.
                let side = if b.crust_m > a.crust_m {
                    signed_fault
                } else {
                    -signed_fault
                };
                let width = 0.038 + 0.114 * smooth(-0.035, 0.035, side);
                let profile = 1.0 - smooth(0.0, width, signed_fault.abs());
                closing_fault = closing_fault.max(pair * profile * closing.clamp(0.0, 1.7));
            }
        }
        crust /= proximity.iter().sum::<f64>();

        let hills = 168.0 * fbm(seed ^ 0xad07, x / 2800.0, z / 2800.0, 4);
        let mut uplift = 0.0;
        if closing_fault > 0.0 {
            let mut ridge = 0.0;
            let mut weight = 1.0;
            let mut frequency = 1.0 / 2900.0;
            for octave in 0..4 {
                ridge += weight
                    * (1.0
                        - gradient(seed ^ (0x6a07 + octave * 719), x * frequency, z * frequency)
                            .abs())
                    .powi(3);
                frequency *= 2.09;
                weight *= 0.47;
            }
            uplift = closing_fault * settings.mountain_strength * (510.0 + 1460.0 * ridge);
        }
        ((crust + hills + uplift) * settings.vertical_scale - settings.sea_level)
            .clamp(-10_500.0, 8_900.0)
    }
}

impl MacroOracle for TectonicClimateOracle {
    fn sample_patch(
        &self,
        world_origin_x_m: f64,
        world_origin_z_m: f64,
        width_px: usize,
        height_px: usize,
    ) -> Result<MacroPatch, MacroOracleError> {
        if width_px == 0 || height_px == 0 {
            return Err(MacroOracleError::EmptyPatch);
        }
        let count = width_px
            .checked_mul(height_px)
            .filter(|&n| n <= MAX_MACRO_SAMPLES)
            .ok_or(MacroOracleError::PatchTooLarge)?;
        let origin = [world_origin_x_m, world_origin_z_m];
        let end = [
            origin[0] + (width_px - 1) as f64 * MACRO_METERS_PER_PIXEL,
            origin[1] + (height_px - 1) as f64 * MACRO_METERS_PER_PIXEL,
        ];
        if origin
            .into_iter()
            .chain(end)
            .any(|v| !(-MAX_WORLD_COORD..=MAX_WORLD_COORD).contains(&v))
        {
            return Err(MacroOracleError::CoordinateOutOfRange);
        }

        let mut data = Vec::with_capacity(count);
        for z in 0..height_px {
            for x in 0..width_px {
                data.push(self.sample_at(
                    origin[0] + x as f64 * MACRO_METERS_PER_PIXEL,
                    origin[1] + z as f64 * MACRO_METERS_PER_PIXEL,
                ));
            }
        }
        Ok(MacroPatch {
            width: width_px,
            height: height_px,
            meters_per_pixel: MACRO_METERS_PER_PIXEL as f32,
            data,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patch_samples_agree_across_borders_and_with_the_fast_elevation_path() {
        let oracle = TectonicClimateOracle::new(0x12c0_fabc);
        let a = oracle
            .sample_patch(-987_654_016.0, -78_060.0, 3, 4)
            .unwrap();
        let b = oracle
            .sample_patch(-987_653_956.0, -78_060.0, 5, 4)
            .unwrap();
        let c = oracle
            .sample_patch(-987_654_016.0, -77_970.0, 3, 2)
            .unwrap();
        for z in 0..4 {
            assert_eq!(a.get(2, z), b.get(0, z));
            let elevation = oracle.elevation_at(-987_653_956.0, -78_060.0 + z as f64 * 30.0);
            assert_eq!(a.get(2, z).unwrap().elevation_m, elevation);
        }
        for x in 0..3 {
            assert_eq!(a.get(x, 3), c.get(x, 0));
        }
        assert_eq!(
            a,
            TectonicClimateOracle::new(0x12c0_fabc)
                .sample_patch(-987_654_016.0, -78_060.0, 3, 4)
                .unwrap()
        );
        assert_ne!(
            a,
            TectonicClimateOracle::new(8)
                .sample_patch(-987_654_016.0, -78_060.0, 3, 4)
                .unwrap()
        );
    }

    #[test]
    fn patch_errors_precede_sampling_or_allocation() {
        let oracle = TectonicClimateOracle::new(1);
        assert_eq!(
            oracle.sample_patch(0.0, 0.0, 0, 1),
            Err(MacroOracleError::EmptyPatch)
        );
        assert_eq!(
            oracle.sample_patch(0.0, 0.0, usize::MAX, 2),
            Err(MacroOracleError::PatchTooLarge)
        );
        assert_eq!(
            oracle.sample_patch(MAX_WORLD_COORD, 0.0, 2, 1),
            Err(MacroOracleError::CoordinateOutOfRange)
        );
        assert_eq!(
            oracle.sample_patch(f64::NAN, 0.0, 1, 1),
            Err(MacroOracleError::CoordinateOutOfRange)
        );
    }

    #[test]
    fn macro_landforms_and_climate_span_land_sea_and_mountain_ranges() {
        let oracle = TectonicClimateOracle::new(57);
        let mut min = f32::INFINITY;
        let mut max = f32::NEG_INFINITY;
        for z in -12..12 {
            for x in -12..12 {
                let sample = oracle.sample_at(f64::from(x) * 9000.0, f64::from(z) * 9000.0);
                min = min.min(sample.elevation_m);
                max = max.max(sample.elevation_m);
                assert!(sample.temperature_c.is_finite() && sample.temp_std_c > 0.0);
                assert!((12.0..=8000.0).contains(&sample.precipitation_mm));
                assert!((0.0..=1.0).contains(&sample.precip_cv));
            }
        }
        assert!(min < -1000.0 && max > 1400.0);
    }

    #[test]
    fn uplift_cools_the_ground_and_drains_the_downwind_air() {
        let oracle = TectonicClimateOracle::new(42);
        let along_wind =
            |distance| oracle.sample_at(oracle.wind[0] * distance, oracle.wind[1] * distance);
        let crest = along_wind(4500.0);
        let leeward = along_wind(6300.0);
        assert!(crest.elevation_m > leeward.elevation_m + 1000.0);
        assert!(crest.temperature_c < leeward.temperature_c - 7.0);
        assert!(crest.precipitation_mm > leeward.precipitation_mm * 3.0);
    }
}
