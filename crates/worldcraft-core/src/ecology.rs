//! Continuous climate habitats for generator 3. No mutable simulation or renderer.
//! Temperature/humidity/altitude regions are inspired by multi-noise biome maps;
//! these are original rules, not Minecraft assets or its world-generation code.
use crate::{Biome, Preset};

pub const BIOME_COUNT: usize = 14;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BiomeBlend {
    pub weights: [f32; BIOME_COUNT],
}

fn smooth(a: f64, b: f64, x: f64) -> f64 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl BiomeBlend {
    pub fn single(biome: Biome) -> Self {
        let mut weights = [0.0; BIOME_COUNT];
        weights[biome as usize] = 1.0;
        Self { weights }
    }

    pub fn climate(
        height: f64,
        temperature: f64,
        moisture: f64,
        preset: Preset,
        stand: f64,
    ) -> Self {
        if !height.is_finite() || !temperature.is_finite() || !moisture.is_finite() {
            return Self::single(Biome::Grassland);
        }
        if height < 0.0 {
            return Self::single(Biome::Ocean);
        }
        let t = temperature.clamp(0.0, 1.0);
        let m = moisture.clamp(0.0, 1.0);
        let beach = 1.0 - smooth(2.0, 12.0, height);
        let snow = 1.0 - smooth(0.09, 0.21, t);
        let volcanic = if preset == Preset::Volcanic {
            smooth(70.0, 200.0, height) * 0.98
        } else {
            0.0
        };
        let hot = smooth(0.58, 0.78, t);
        let cool = 1.0 - smooth(0.28, 0.48, t);
        let wood = smooth(0.32, 0.64, m);
        let dry = 1.0 - smooth(0.16, 0.30, m);
        let birch = smooth(0.38, 0.70, stand.clamp(0.0, 1.0));
        let mut raw = [0.0_f64; BIOME_COUNT];
        raw[Biome::Grassland as usize] = 0.12 + (1.0 - wood) * (1.0 - dry) * (1.0 - hot);
        raw[Biome::Forest as usize] = wood * (1.0 - cool) * (1.0 - hot) * (1.7 - birch);
        raw[Biome::BirchForest as usize] = wood * (1.0 - cool) * (1.0 - hot) * birch * 1.8;
        raw[Biome::Conifer as usize] = wood * cool * 3.0;
        raw[Biome::Rainforest as usize] = hot * smooth(0.65, 0.84, m) * 4.0;
        raw[Biome::Desert as usize] = dry * (0.8 + hot * 2.5);
        raw[Biome::Savanna as usize] = hot * (1.0 - dry) * (1.0 - smooth(0.30, 0.60, m)) * 2.5;
        raw[Biome::Tundra as usize] = (1.0 - smooth(0.20, 0.34, t)) * (1.8 - wood);
        raw[Biome::Meadow as usize] = (1.0 - hot)
            * (1.0 - cool)
            * (1.0 - wood * 0.7)
            * smooth(80.0, 360.0, height)
            * smooth(0.25, 0.42, m)
            * 1.7;
        raw[Biome::Wetland as usize] =
            smooth(0.66, 0.88, m) * (1.0 - smooth(8.0, 45.0, height)) * smooth(0.24, 0.46, t) * 5.0;
        let total: f64 = raw.iter().sum();
        let land = (1.0 - beach) * (1.0 - snow) * (1.0 - volcanic);
        let mut weights = raw.map(|v| (v / total * land) as f32);
        weights[Biome::Beach as usize] = (beach * (1.0 - snow)) as f32;
        weights[Biome::Snow as usize] = snow as f32;
        weights[Biome::Volcanic as usize] = ((1.0 - beach) * (1.0 - snow) * volcanic) as f32;
        Self { weights }
    }

    pub fn weight(&self, biome: Biome) -> f32 {
        self.weights[biome as usize]
    }

    pub fn dominant(&self) -> Biome {
        let index = self
            .weights
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .unwrap()
            .0;
        Biome::ALL[index]
    }

    pub fn painted(&mut self, biome: Biome, weight: f64) {
        let w = weight.clamp(0.0, 1.0) as f32;
        for value in &mut self.weights {
            *value *= 1.0 - w;
        }
        self.weights[biome as usize] += w;
    }

    pub fn color(&self) -> [f32; 3] {
        let mut result = [0.0; 3];
        for (biome, weight) in Biome::ALL.into_iter().zip(self.weights) {
            let color = biome.color();
            for i in 0..3 {
                result[i] += color[i] * weight;
            }
        }
        result
    }

    pub fn habitat(&self) -> Habitat {
        let mut h = Habitat::default();
        let mut woodland = 0.0;
        let mut wooded_canopy = 0.0;
        for (biome, w) in Biome::ALL.into_iter().zip(self.weights) {
            let (canopy, ground, fern, flower, reed) = match biome {
                Biome::Ocean | Biome::Snow | Biome::Volcanic => (0.0, 0.0, 0.0, 0.0, 0.0),
                Biome::Beach => (0.02, 0.05, 0.0, 0.0, 0.0),
                Biome::Desert => (0.0, 0.035, 0.0, 0.0, 0.0),
                Biome::Tundra => (0.06, 0.44, 0.0, 0.015, 0.0),
                Biome::Grassland => (0.10, 0.86, 0.0, 0.08, 0.0),
                Biome::Meadow => (0.045, 0.98, 0.0, 0.19, 0.0),
                Biome::Savanna => (0.16, 0.72, 0.0, 0.025, 0.0),
                Biome::Forest => (0.94, 0.82, 0.14, 0.02, 0.0),
                Biome::BirchForest => (0.82, 0.92, 0.08, 0.10, 0.0),
                Biome::Conifer => (0.89, 0.69, 0.10, 0.015, 0.0),
                Biome::Rainforest => (1.0, 0.92, 0.26, 0.01, 0.03),
                Biome::Wetland => (0.42, 0.95, 0.12, 0.025, 0.48),
            };
            if matches!(
                biome,
                Biome::Forest
                    | Biome::BirchForest
                    | Biome::Conifer
                    | Biome::Rainforest
                    | Biome::Wetland
            ) {
                woodland += w;
                wooded_canopy += canopy * w;
            } else {
                h.canopy += canopy * w;
            }
            h.ground += ground * w;
            h.ferns += fern * w;
            h.flowers += flower * w;
            h.reeds += reed * w;
        }
        // Small neighboring forest weights must not turn a meadow into a
        // plantation. Retain continuous ecotones, with broad clearings opening
        // before the habitat becomes predominantly woodland.
        let cover = smooth(0.20, 0.65, woodland as f64) as f32;
        let open_canopy = h.canopy / (1.0 - woodland).max(0.000001);
        h.canopy = open_canopy * (1.0 - cover) + wooded_canopy * cover;
        h
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Habitat {
    pub canopy: f32,
    pub ground: f32,
    pub ferns: f32,
    pub flowers: f32,
    pub reeds: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn open_habitats_do_not_inherit_full_forest_density_from_small_neighbor_weights() {
        let mut meadow = BiomeBlend::single(Biome::Meadow);
        meadow.painted(Biome::Forest, 0.15);
        assert!(meadow.habitat().canopy < 0.08);
        let mut previous = BiomeBlend::single(Biome::Meadow).habitat().canopy;
        for i in 0..=100 {
            let mut blend = BiomeBlend::single(Biome::Meadow);
            blend.painted(Biome::Forest, i as f64 / 100.0);
            let canopy = blend.habitat().canopy;
            assert!(canopy + 0.000001 >= previous);
            assert!(canopy - previous < 0.04);
            previous = canopy;
        }
        assert!(BiomeBlend::single(Biome::Forest).habitat().canopy > 0.9);
    }
    #[test]
    fn climate_weights_are_normalized_and_continuous_across_ecotones() {
        for h in [0.0, 3.0, 10.0, 30.0, 160.0, 800.0] {
            for ti in 0..=20 {
                for mi in 0..=20 {
                    let t = ti as f64 / 20.0;
                    let m = mi as f64 / 20.0;
                    let a = BiomeBlend::climate(h, t, m, Preset::Temperate, 0.5);
                    let b = BiomeBlend::climate(h, t + 0.00001, m, Preset::Temperate, 0.5);
                    assert!((a.weights.iter().sum::<f32>() - 1.0).abs() < 0.000002);
                    assert!(a
                        .weights
                        .iter()
                        .all(|w| w.is_finite() && *w >= 0.0 && *w <= 1.0));
                    assert!(a
                        .weights
                        .iter()
                        .zip(b.weights)
                        .all(|(x, y)| (x - y).abs() < 0.001));
                }
            }
        }
    }
    #[test]
    fn ecological_extremes_and_painted_habitats_remain_distinct() {
        for (height, t, m, expected) in [
            (-1.0, 0.8, 0.8, Biome::Ocean),
            (1.0, 0.6, 0.5, Biome::Beach),
            (300.0, 0.04, 0.5, Biome::Snow),
            (100.0, 0.9, 0.05, Biome::Desert),
            (100.0, 0.90, 0.96, Biome::Rainforest),
            (100.0, 0.30, 0.68, Biome::Conifer),
            (100.0, 0.88, 0.35, Biome::Savanna),
            (18.0, 0.65, 0.98, Biome::Wetland),
            (300.0, 0.45, 0.15, Biome::Desert),
            (300.0, 0.55, 0.43, Biome::Meadow),
        ] {
            assert_eq!(
                BiomeBlend::climate(height, t, m, Preset::Temperate, 0.2).dominant(),
                expected
            );
        }
        let mut blend = BiomeBlend::single(Biome::Forest);
        blend.painted(Biome::Desert, 1.0);
        assert_eq!(blend, BiomeBlend::single(Biome::Desert));
        assert_eq!(blend.habitat().canopy, 0.0);
    }
}
