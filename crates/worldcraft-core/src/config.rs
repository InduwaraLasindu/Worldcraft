use serde::{Deserialize, Serialize};

pub const GENERATOR_VERSION: u32 = 3;
pub const MAX_WORLD_COORD: f64 = 1_000_000_000.0;

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum Preset {
    #[default]
    Temperate,
    Alpine,
    Tropical,
    Desert,
    Arctic,
    Islands,
    Savanna,
    Volcanic,
    Empty,
    Estuary,
}

impl Preset {
    pub const ALL: [Self; 10] = [
        Self::Temperate,
        Self::Alpine,
        Self::Tropical,
        Self::Desert,
        Self::Arctic,
        Self::Islands,
        Self::Savanna,
        Self::Volcanic,
        Self::Estuary,
        Self::Empty,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Temperate => "Temperate wilderness",
            Self::Alpine => "Alpine mountains",
            Self::Tropical => "Tropical highlands",
            Self::Desert => "Desert and canyons",
            Self::Arctic => "Arctic wilderness",
            Self::Islands => "Archipelago",
            Self::Savanna => "Grassland and savanna",
            Self::Volcanic => "Volcanic islands",
            Self::Estuary => "Tidal estuary",
            Self::Empty => "Empty canvas",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GeneratorSettings {
    pub seed: u64,
    pub preset: Preset,
    pub sea_level: f64,
    pub vertical_scale: f64,
    pub terrain_scale: f64,
    pub mountain_strength: f64,
    pub river_strength: f64,
    pub vegetation: f64,
    pub temperature: f64,
    pub rainfall: f64,
}

impl Default for GeneratorSettings {
    fn default() -> Self {
        Self::for_preset(Preset::Temperate, 20261116)
    }
}

impl GeneratorSettings {
    pub fn for_preset(preset: Preset, seed: u64) -> Self {
        let mut s = Self {
            seed,
            preset,
            sea_level: 0.0,
            vertical_scale: 1.0,
            terrain_scale: 1.0,
            mountain_strength: 1.0,
            river_strength: 1.0,
            vegetation: 0.8,
            temperature: 0.55,
            rainfall: 0.65,
        };
        match preset {
            Preset::Alpine => {
                s.mountain_strength = 2.2;
                s.temperature = 0.57;
            }
            Preset::Tropical => {
                s.temperature = 0.93;
                s.rainfall = 0.95;
                s.vegetation = 1.2;
            }
            Preset::Desert => {
                s.temperature = 0.9;
                s.rainfall = 0.08;
                s.vegetation = 0.06;
            }
            Preset::Arctic => {
                s.temperature = 0.04;
                s.rainfall = 0.35;
                s.vegetation = 0.15;
            }
            Preset::Islands => {
                s.sea_level = 65.0;
                s.terrain_scale = 0.65;
            }
            Preset::Savanna => {
                s.temperature = 0.83;
                s.rainfall = 0.3;
                s.mountain_strength = 0.5;
            }
            Preset::Volcanic => {
                s.sea_level = 25.0;
                s.mountain_strength = 2.5;
                s.vegetation = 0.2;
            }
            Preset::Estuary => {
                s.temperature = 0.59;
                s.rainfall = 0.78;
                s.vegetation = 0.87;
            }
            Preset::Empty => {
                s.vegetation = 0.0;
                s.sea_level = -5.0;
            }
            Preset::Temperate => {}
        }
        s
    }

    pub fn validate(&self) -> Result<(), String> {
        for (name, value, min, max) in [
            ("sea level", self.sea_level, -1000.0, 1000.0),
            ("vertical scale", self.vertical_scale, 0.1, 5.0),
            ("terrain scale", self.terrain_scale, 0.2, 5.0),
            ("mountains", self.mountain_strength, 0.0, 4.0),
            ("river valleys", self.river_strength, 0.0, 3.0),
            ("vegetation", self.vegetation, 0.0, 2.0),
            ("temperature", self.temperature, 0.0, 1.0),
            ("rainfall", self.rainfall, 0.0, 1.0),
        ] {
            if !value.is_finite() || !(min..=max).contains(&value) {
                return Err(format!("{name} must be between {min} and {max}"));
            }
        }
        Ok(())
    }

    /// FNV-1a over canonical serialized settings; not a cryptographic signature.
    pub fn fingerprint(&self) -> u64 {
        let bytes = serde_json::to_vec(self).expect("validated settings serialize");
        bytes.into_iter().fold(0xcbf29ce484222325, |h, b| {
            (h ^ u64::from(b)).wrapping_mul(0x100000001b3)
        })
    }
}

pub fn seed_from_text(text: &str) -> u64 {
    text.trim().parse().unwrap_or_else(|_| {
        text.as_bytes().iter().fold(0xcbf29ce484222325u64, |h, b| {
            (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
        })
    })
}
