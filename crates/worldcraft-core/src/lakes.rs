//! Seeded glacial basins in cold, wet mountain regions. Each basin supplies
//! both the terrain profile and the water polygon used by the renderer.
use crate::{
    generation::unit,
    geology::{gradient, smooth},
    GeneratorSettings, MeshData, Preset,
};
use std::f64::consts::{PI, TAU};

const CELL: f64 = 2432.0;

#[derive(Clone, Copy, Debug)]
pub struct Lake {
    pub cell: [i64; 2],
    pub center: [f64; 2],
    pub level: f64,
    radius: f64,
    aspect: f64,
    phase: f64,
    depth: f64,
    ridge: f64,
    base_x: f64,
    base_z: f64,
    salt: u64,
}

pub fn cell(x: f64, z: f64) -> [i64; 2] {
    [
        (x / CELL + 0.5).floor() as i64,
        (z / CELL + 0.5).floor() as i64,
    ]
}

pub fn enabled(s: &GeneratorSettings) -> bool {
    s.mountain_strength >= 1.2
        && s.rainfall >= 0.4
        && s.temperature <= 0.67
        && !matches!(
            s.preset,
            Preset::Empty
                | Preset::Estuary
                | Preset::RiverValley
                | Preset::Islands
                | Preset::Volcanic
        )
}

impl Lake {
    pub fn at(s: &GeneratorSettings, key: [i64; 2]) -> Option<Self> {
        let [cx, cz] = key;
        let seed = s.seed ^ 0x4c61_6b65_4669_656c;
        let u = |id| unit(seed ^ id, cx, cz);
        if !enabled(s)
            || u(0)
                > if s.preset == Preset::Alpine {
                    0.78
                } else {
                    0.27
                }
        {
            return None;
        }
        let x = cx as f64 * CELL;
        let z = cz as f64 * CELL;
        let center = [x + (u(2) - 0.5) * 298.0, z + (u(3) - 0.5) * 298.0];
        let landscape = crate::geology::raw_height(s, center[0], center[1]);
        let level = (landscape - (36.0 + 26.0 * u(1)) * s.vertical_scale)
            .max(s.sea_level + 54.0 * s.vertical_scale);
        Some(Self {
            cell: key,
            center,
            level,
            radius: 177.0 + u(4) * 136.0,
            aspect: 0.59 + u(5) * 0.33,
            phase: u(6) * TAU,
            depth: (18.0 + u(7) * 29.0) * s.vertical_scale,
            ridge: (270.0 + 220.0 * u(8))
                * s.vertical_scale
                * (s.mountain_strength / 2.2).clamp(0.55, 1.42),
            base_x: x,
            base_z: z,
            salt: seed ^ (cx as u64).wrapping_mul(0xca37) ^ (cz as u64).wrapping_mul(0xb951),
        })
    }

    fn direction(self) -> [f64; 2] {
        [self.phase.cos(), self.phase.sin()]
    }

    fn local(self, x: f64, z: f64) -> [f64; 2] {
        let [u, v] = self.direction();
        let dx = x - self.center[0];
        let dz = z - self.center[1];
        [dx * u + dz * v, -dx * v + dz * u]
    }

    fn boundary(self, a: f64) -> f64 {
        let [sin, cos] = [a.sin(), a.cos()];
        let ellipse = self.radius / (cos * cos + (sin / self.aspect).powi(2)).sqrt();
        ellipse
            * (1.0
                + 0.112 * (a * 3.0 + self.phase).sin()
                + 0.067 * (a * 5.0 - self.phase * 0.57).cos()
                + 0.026 * (a * 11.0 + self.phase * 0.93).sin())
    }

    pub fn distance(self, x: f64, z: f64) -> f64 {
        if (x - self.center[0]).abs().max((z - self.center[1]).abs()) > self.radius * 2.92 {
            return f64::INFINITY;
        }
        let [x, z] = self.local(x, z);
        x.hypot(z) / self.boundary(z.atan2(x))
    }

    pub fn terrain(self, x: f64, z: f64, terrain: f64) -> f64 {
        let [a, b] = self.local(x, z);
        if a < -self.radius * 3.1 || a > self.radius * 5.4 || b.abs() > self.radius * 3.35 {
            return terrain;
        }
        let distance = a.hypot(b) / self.boundary(b.atan2(a));
        let cell_fade = 1.0
            - smooth(
                850.0,
                1150.0,
                (x - self.base_x).abs().max((z - self.base_z).abs()),
            );
        let highland = {
            // The cirque headwall rises up-valley with multiple overlapping aretes.
            let arch = a / self.radius;
            let side = b / self.radius;
            let ridge_noise = gradient(self.salt ^ 17, x / 152.0, z / 152.0);
            let spine =
                (arch - (3.22 + 0.43 * ridge_noise + 0.31 * (side * 2.6 + self.phase).sin())).abs();
            let core = (1.0 - (spine * 0.69).hypot(side * 0.32))
                .max(0.0)
                .powf(1.27);
            let buttress = (1.0 - gradient(self.salt ^ 53, x / 200.0, z / 200.0).abs()).powi(3);
            let gullies = gradient(self.salt ^ 62, x / 55.0, z / 106.0);
            let faces = gradient(self.salt ^ 71, x / 28.0, z / 43.0);
            (core * (self.ridge * (0.63 + 0.40 * buttress) + 37.0 * gullies + 18.0 * faces))
                .max(0.0)
        };
        let headwall = self.level + 22.0 + highland;
        let raised = terrain
            + (headwall - terrain).max(0.0)
                * cell_fade
                * smooth(2.08, 2.61, distance)
                * smooth(3.0, 100.0, highland);
        if distance > 2.58 {
            return raised;
        }
        let shelf = self.level + (distance - 1.0) * (if distance < 1.0 { 18.0 } else { 32.0 });
        let slope = smooth(0.95, 0.25, distance);
        let noise = gradient(self.salt ^ 33, x / 43.0, z / 43.0);
        let bed = shelf - self.depth * slope + noise * (1.0 - smooth(0.77, 0.97, distance)) * 1.5;
        let terrace = bed + (4.0 + 7.0 * noise) * smooth(1.035, 1.49, distance);
        let outside = smooth(1.28, 2.58, distance);
        terrace + (raised - terrace) * outside
    }

    pub fn shore(self) -> ([f64; 2], [f64; 2]) {
        let [u, v] = self.direction();
        let r = self.boundary(PI) * 1.018;
        (
            [self.center[0] - r * u, self.center[1] - r * v],
            self.center,
        )
    }

    pub fn mesh(self) -> MeshData {
        const RIM: usize = 128;
        let mut mesh = MeshData::default();
        mesh.vertex([0.0; 3], [0.0, 1.0, 0.0], [1.0; 4], [0.0; 2]);
        for i in 0..RIM {
            let a = TAU * i as f64 / RIM as f64;
            let r = self.boundary(a) * 1.035;
            let a = a + self.phase;
            let (x, z) = (r * a.cos(), r * a.sin());
            mesh.vertex(
                [x as f32, 0.0, z as f32],
                [0.0, 1.0, 0.0],
                [1.0; 4],
                [0.0; 2],
            );
            mesh.indices
                .extend([0, ((i + 1) % RIM + 1) as u32, (i + 1) as u32]);
        }
        mesh
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Generator;

    #[test]
    fn lake_field_is_seeded_and_versioned() {
        for seed in [57, 324, 712] {
            let settings = GeneratorSettings::for_preset(Preset::Alpine, seed);
            let world = Generator::new(settings.clone(), vec![]);
            let legacy = Generator::for_version(settings, vec![], 4).unwrap();
            let lake = world.spawn_lake().unwrap();
            let [x, z] = lake.center;
            assert_eq!(world.water_level(x, z), lake.level);
            assert!(world.height(x, z) < lake.level - 10.0);
            assert_eq!(world.sample(x, z).biome, crate::Biome::Ocean);
            assert!(legacy.lake(lake.cell).is_none());
            let [sx, sy, sz] = world.spawn_position();
            assert!(sy > lake.level + 1.75);
            assert!(lake.distance(sx, sz) > 1.0);
            let mesh = lake.mesh();
            assert!(mesh
                .positions
                .iter()
                .skip(1)
                .all(|p| { world.height(x + f64::from(p[0]), z + f64::from(p[2])) > lake.level }));
            let other = (-3..=3)
                .flat_map(|cz| {
                    (-3..=3).filter_map(move |cx| {
                        (cx != lake.cell[0] || cz != lake.cell[1]).then_some([cx, cz])
                    })
                })
                .find_map(|cell| world.lake(cell))
                .unwrap();
            assert_ne!(other.mesh().positions, mesh.positions);
            assert_ne!(other.center, lake.center);
        }
    }
}
