//! Persisted learned regional relief. Exploring and reopening need no model.
//! Coarse and finite three-stage recipes share the same saved, offline sampling API.
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LearnedRegion {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<LearnedDetail>,
    pub recipe: String,
    pub model_revision: String,
    pub model_sha256: String,
    pub side: usize,
    pub origin: [f64; 2],
    pub spacing: f64,
    pub blend: f64,
    pub height: Vec<f32>,
    pub temperature: Vec<f32>,
    pub precipitation: Vec<f32>,
}
impl LearnedRegion {
    pub fn validate(&self) -> Result<(), String> {
        if ![
            "terrain-diffusion-coarse-hybrid-v1",
            "terrain-diffusion-detailed-region-v1",
        ]
        .contains(&self.recipe.as_str())
            || self.model_revision.len() != 40
            || !self.model_revision.bytes().all(|c| c.is_ascii_hexdigit())
            || self.model_sha256.len() != 64
            || !self.model_sha256.bytes().all(|c| c.is_ascii_hexdigit())
        {
            return Err("Unsupported learned-region recipe or model identity".into());
        }
        if !(16..=512).contains(&self.side)
            || !self.spacing.is_finite()
            || !(64.0..=(if self.detail.is_some() {
                65536.0
            } else {
                8192.0
            }))
                .contains(&self.spacing)
            || !self.origin.iter().all(|c| c.is_finite() && c.abs() < 1e8)
            || !self.blend.is_finite()
            || self.blend < self.spacing
            || self.blend > self.extent() * 0.5
        {
            return Err("Invalid learned-region grid or transition band".into());
        }
        if let Some(detail) = &self.detail {
            if self.recipe != "terrain-diffusion-detailed-region-v1" {
                return Err("Detailed samples require the detailed recipe".into());
            }
            detail.validate()?;
            for axis in 0..2 {
                if detail.origin[axis] < self.origin[axis] + self.blend
                    || detail.origin[axis] + detail.extent()
                        > self.origin[axis] + self.extent() - self.blend
                {
                    return Err("Detailed relief must fit within its coarse parent".into());
                }
            }
        } else if self.recipe == "terrain-diffusion-detailed-region-v1" {
            return Err("Detailed recipe is missing its saved detail field".into());
        }
        let n = self.side * self.side;
        if self.height.len() != n
            || self.temperature.len() != n
            || self.precipitation.len() != n
            || !self
                .height
                .iter()
                .all(|h| h.is_finite() && h.abs() <= 20000.0)
            || !self
                .temperature
                .iter()
                .all(|h| h.is_finite() && (-100.0..=80.0).contains(h))
            || !self
                .precipitation
                .iter()
                .all(|h| h.is_finite() && (0.0..=30000.0).contains(h))
        {
            return Err("Invalid learned-region samples".into());
        }
        Ok(())
    }
    pub fn fine_detail_scale(&self, x: f64, z: f64) -> f64 {
        self.detail
            .as_ref()
            .and_then(|d| d.sample(x, z))
            .map_or(1.0, |[_, w]| 1.0 - w * (5.0 / 6.0))
    }
    pub fn extent(&self) -> f64 {
        (self.side - 1) as f64 * self.spacing
    }
    /// Height in metres, temperature in Celsius, precipitation in mm/year, blend weight.
    pub fn sample(&self, x: f64, z: f64) -> Option<[f64; 4]> {
        let u = (x - self.origin[0]) / self.spacing;
        let v = (z - self.origin[1]) / self.spacing;
        let limit = (self.side - 1) as f64;
        if !u.is_finite() || !v.is_finite() || u <= 0.0 || v <= 0.0 || u >= limit || v >= limit {
            return None;
        }
        let ix = (u.floor() as usize).min(self.side - 2);
        let iz = (v.floor() as usize).min(self.side - 2);
        let (tx, tz) = (u - ix as f64, v - iz as f64);
        let a = iz * self.side + ix;
        let read = |data: &[f32]| {
            let top = f64::from(data[a]) * (1.0 - tx) + f64::from(data[a + 1]) * tx;
            let bottom = f64::from(data[a + self.side]) * (1.0 - tx)
                + f64::from(data[a + self.side + 1]) * tx;
            top * (1.0 - tz) + bottom * tz
        };
        let d = u.min(v).min(limit - u).min(limit - v) * self.spacing;
        let t = (d / self.blend).clamp(0.0, 1.0);
        let coarse_height = read(&self.height);
        let height = self
            .detail
            .as_ref()
            .and_then(|d| d.sample(x, z))
            .map_or(coarse_height, |[h, w]| coarse_height * (1.0 - w) + h * w);
        let temperature =
            (read(&self.temperature) - (height - coarse_height) * 0.0065).clamp(-100.0, 80.0);
        Some([
            height,
            temperature,
            read(&self.precipitation),
            t * t * (3.0 - 2.0 * t),
        ])
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LearnedDetail {
    pub base_model_sha256: String,
    pub decoder_model_sha256: String,
    pub side: usize,
    pub origin: [f64; 2],
    pub spacing: f64,
    pub blend: f64,
    pub height: Vec<f32>,
}
impl LearnedDetail {
    pub fn extent(&self) -> f64 {
        (self.side - 1) as f64 * self.spacing
    }
    pub fn validate(&self) -> Result<(), String> {
        if !(32..=512).contains(&self.side)
            || !self.spacing.is_finite()
            || !(1.0..=1024.0).contains(&self.spacing)
            || !self.origin.iter().all(|v| v.is_finite() && v.abs() < 1e8)
            || !self.blend.is_finite()
            || self.blend < self.spacing
            || self.blend > self.extent() * 0.4
            || self.height.len() != self.side * self.side
            || !self
                .height
                .iter()
                .all(|v| v.is_finite() && v.abs() <= 20000.0)
            || [&self.base_model_sha256, &self.decoder_model_sha256]
                .iter()
                .any(|s| s.len() != 64 || !s.bytes().all(|c| c.is_ascii_hexdigit()))
        {
            return Err("Invalid saved detailed terrain field".into());
        }
        Ok(())
    }
    /// Absolute height in metres and a smooth transition weight.
    pub fn sample(&self, x: f64, z: f64) -> Option<[f64; 2]> {
        let u = (x - self.origin[0]) / self.spacing;
        let v = (z - self.origin[1]) / self.spacing;
        let limit = (self.side - 1) as f64;
        if !u.is_finite() || !v.is_finite() || u <= 0.0 || v <= 0.0 || u >= limit || v >= limit {
            return None;
        }
        let ix = (u.floor() as usize).min(self.side - 2);
        let iz = (v.floor() as usize).min(self.side - 2);
        let (tx, tz) = (u - ix as f64, v - iz as f64);
        let a = iz * self.side + ix;
        let top = f64::from(self.height[a]) * (1.0 - tx) + f64::from(self.height[a + 1]) * tx;
        let bottom = f64::from(self.height[a + self.side]) * (1.0 - tx)
            + f64::from(self.height[a + self.side + 1]) * tx;
        let t =
            (u.min(v).min(limit - u).min(limit - v) * self.spacing / self.blend).clamp(0.0, 1.0);
        Some([top * (1.0 - tz) + bottom * tz, t * t * (3.0 - 2.0 * t)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn patch() -> LearnedRegion {
        LearnedRegion {
            detail: None,
            recipe: "terrain-diffusion-coarse-hybrid-v1".into(),
            model_revision: "a".repeat(40),
            model_sha256: "b".repeat(64),
            side: 16,
            origin: [-1000.0, -1000.0],
            spacing: 128.0,
            blend: 256.0,
            height: vec![300.0; 256],
            temperature: vec![20.0; 256],
            precipitation: vec![1200.0; 256],
        }
    }
    #[test]
    fn saved_regional_relief_is_continuous_and_models_are_not_needed_to_reopen() {
        let region = patch();
        region.validate().unwrap();
        let mut world =
            crate::WorldFile::new("Learned".into(), crate::GeneratorSettings::default());
        world.learned = Some(region.clone());
        let decoded: crate::WorldFile =
            serde_json::from_str(&serde_json::to_string(&world).unwrap()).unwrap();
        let g = crate::Generator::for_world(&decoded).unwrap();
        let before = g.height(0.0, 0.0);
        assert!((before - 300.0).abs() < 20.0);
        let mut stroke = crate::Stroke {
            center: [0.0, 0.0],
            brush: crate::Brush::default(),
        };
        stroke.brush.strength = 5.0;
        let edited = g.with_edits(vec![stroke]).unwrap();
        assert!((edited.height(0.0, 0.0) - before - 5.0).abs() < 1e-6);
        let plain = crate::Generator::new(world.settings.clone(), vec![]);
        let x = region.origin[0];
        assert_eq!(g.height(x, -500.0), plain.height(x, -500.0));
        assert!((g.height(x + 0.001, -500.0) - g.height(x - 0.001, -500.0)).abs() < 0.01);
        assert_eq!(
            g.height(0.0, 0.0),
            crate::Generator::for_world(&decoded)
                .unwrap()
                .height(0.0, 0.0)
        );
        assert_eq!(g.preview_sample(0.0, 0.0).height, before);
    }
    #[test]
    fn malformed_grids_and_nonfinite_samples_are_rejected() {
        let mut p = patch();
        p.height.pop();
        assert!(p.validate().is_err());
        let mut p = patch();
        p.temperature[0] = f32::NAN;
        assert!(p.validate().is_err());
        let mut p = patch();
        p.side = usize::MAX;
        assert!(p.validate().is_err());
    }
}

#[cfg(test)]
mod precision_regression {
    #[test]
    fn saved_nontrivial_f64_values_keep_their_exact_bits() {
        let mut settings = crate::GeneratorSettings::default();
        settings.terrain_scale = 1.2345678901234567;
        let mut world = crate::WorldFile::new("Precision regression".into(), settings);
        world.camera = [-10000000.12345679, 178.94454593138767, 99999999.87654321];
        let json = serde_json::to_vec(&world).unwrap();
        let restored: crate::WorldFile = serde_json::from_slice(&json).unwrap();
        assert_eq!(world, restored);
        restored.validate().unwrap();
    }
}

#[cfg(test)]
mod detail_tests {
    use super::*;
    fn region() -> LearnedRegion {
        LearnedRegion {
            recipe: "terrain-diffusion-detailed-region-v1".into(),
            model_revision: "a".repeat(40),
            model_sha256: "b".repeat(64),
            side: 16,
            origin: [-6000.0; 2],
            spacing: 800.0,
            blend: 800.0,
            height: vec![100.0; 256],
            temperature: vec![20.0; 256],
            precipitation: vec![1200.0; 256],
            detail: Some(LearnedDetail {
                base_model_sha256: "c".repeat(64),
                decoder_model_sha256: "d".repeat(64),
                side: 32,
                origin: [-465.0; 2],
                spacing: 30.0,
                blend: 120.0,
                height: vec![500.0; 32 * 32],
            }),
        }
    }
    #[test]
    fn detailed_world_round_trips_and_joins_coarse_without_inference() {
        let region = region();
        region.validate().unwrap();
        assert!((region.sample(0.0, 0.0).unwrap()[0] - 500.0).abs() < 1e-6);
        assert!((region.sample(0.0, 0.0).unwrap()[1] - 17.4).abs() < 1e-6);
        assert_eq!(region.sample(-465.0, 0.0).unwrap()[0], 100.0);
        assert!(
            (region.sample(-465.001, 0.0).unwrap()[0] - region.sample(-464.999, 0.0).unwrap()[0])
                .abs()
                < 0.001
        );
        let mut world = crate::WorldFile::new(
            "Detailed fixture".into(),
            crate::GeneratorSettings::default(),
        );
        world.learned = Some(region);
        world.validate().unwrap();
        let decoded: crate::WorldFile =
            serde_json::from_str(&serde_json::to_string(&world).unwrap()).unwrap();
        assert_eq!(world, decoded);
        let generator = crate::Generator::for_world(&decoded).unwrap();
        let before = generator.height(0.0, 0.0);
        let edit = crate::Stroke {
            center: [0.0, 0.0],
            brush: crate::Brush::default(),
        };
        assert!(
            (generator.with_edits(vec![edit]).unwrap().height(0.0, 0.0) - before - 5.0).abs()
                < 1e-6
        );
        world.format_version = 2;
        assert!(world.validate().is_err());
    }
    #[test]
    fn malformed_or_misplaced_detailed_fields_are_rejected() {
        let mut r = region();
        r.detail.as_mut().unwrap().height[0] = f32::NAN;
        assert!(r.validate().is_err());
        let mut r = region();
        r.detail.as_mut().unwrap().origin[0] = 50000.0;
        assert!(r.validate().is_err());
        let mut r = region();
        r.detail.as_mut().unwrap().side = usize::MAX;
        assert!(r.validate().is_err());
        let mut r = region();
        r.recipe = "terrain-diffusion-coarse-hybrid-v1".into();
        assert!(r.validate().is_err());
    }
}
