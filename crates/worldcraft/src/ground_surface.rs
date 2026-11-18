//! The visible near terrain and walking collision share this four-metre triangle grid.
use bevy::prelude::*;
use worldcraft_core::{Generator, MeshData};

const CELLS: usize = 32;
const STEP: f64 = 4.0;
// One extra vertex row/column on each side seats blades that span a tile edge.
const SIDE: usize = CELLS + 3;

fn triangle(u: f64, v: f64, h: [f64; 4]) -> (f64, [f64; 2]) {
    let [a, b, c, d] = h;
    if u + v <= 1.0 {
        (a + (b - a) * u + (c - a) * v, [b - a, c - a])
    } else {
        (
            d + (c - d) * (1.0 - u) + (b - d) * (1.0 - v),
            [d - c, d - b],
        )
    }
}

pub fn walk_height(g: &Generator, x: f64, z: f64) -> f64 {
    let ax = (x / STEP).floor() * STEP;
    let az = (z / STEP).floor() * STEP;
    let heights = [
        (ax, az),
        (ax + STEP, az),
        (ax, az + STEP),
        (ax + STEP, az + STEP),
    ]
    .map(|(x, z)| g.height(x, z));
    triangle((x - ax) / STEP, (z - az) / STEP, heights).0
}

pub struct GroundSurface<'a> {
    generator: &'a Generator,
    origin: [f64; 2],
    heights: Box<[f32]>,
}
impl<'a> GroundSurface<'a> {
    pub fn new(generator: &'a Generator, origin: [f64; 2], terrain: Option<&MeshData>) -> Self {
        let mut heights = vec![f32::NAN; SIDE * SIDE].into_boxed_slice();
        if let Some(mesh) = terrain {
            // Ground batches only run at LOD 0. Skirts follow the grid vertices.
            for z in 0..=CELLS {
                for x in 0..=CELLS {
                    heights[(z + 1) * SIDE + x + 1] = mesh.positions[z * (CELLS + 1) + x][1];
                }
            }
        }
        Self {
            generator,
            origin,
            heights,
        }
    }

    fn height(&mut self, x: i32, z: i32) -> f64 {
        let index = (z + 1) as usize * SIDE + (x + 1) as usize;
        let value = &mut self.heights[index];
        if value.is_nan() {
            *value = self.generator.height(
                self.origin[0] + f64::from(x) * STEP,
                self.origin[1] + f64::from(z) * STEP,
            ) as f32;
        }
        f64::from(*value)
    }

    pub fn plane(&mut self, x: f32, z: f32) -> (f32, Vec3) {
        let (u, v) = (f64::from(x) / STEP, f64::from(z) / STEP);
        let (ix, iz) = (u.floor() as i32, v.floor() as i32);
        let h = [(ix, iz), (ix + 1, iz), (ix, iz + 1), (ix + 1, iz + 1)]
            .map(|(x, z)| self.height(x, z));
        let (y, [dx, dz]) = triangle(u - f64::from(ix), v - f64::from(iz), h);
        (
            y as f32,
            Vec3::new(-dx as f32, STEP as f32, -dz as f32).normalize(),
        )
    }

    pub fn seat(&mut self, mut p: Vec3) -> Vec3 {
        // The grass hot path needs only three cached f32 heights and no normal.
        let (x, z) = (p.x * 0.25, p.z * 0.25);
        let (ix, iz) = (x.floor() as i32, z.floor() as i32);
        let (u, v) = (x - ix as f32, z - iz as f32);
        let b = self.height(ix + 1, iz) as f32;
        let c = self.height(ix, iz + 1) as f32;
        p.y = if u + v <= 1.0 {
            let a = self.height(ix, iz) as f32;
            a + (b - a) * u + (c - a) * v
        } else {
            let d = self.height(ix + 1, iz + 1) as f32;
            d + (c - d) * (1.0 - u) + (b - d) * (1.0 - v)
        };
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use worldcraft_core::{ChunkCoord, ChunkKey, GeneratorSettings, Preset};
    #[test]
    fn cached_triangle_planes_join_across_tile_edges_and_agree_with_collision() {
        let g = Generator::new(GeneratorSettings::for_preset(Preset::Alpine, 324), vec![]);
        let coord = ChunkCoord { x: -3, z: -2 };
        let origin = coord.origin();
        let terrain = worldcraft_core::continuous::build_continuous(
            &g,
            ChunkKey {
                coord,
                lod: 0,
                edges: [0; 4],
                revision: 0,
            },
        );
        let mut filled = GroundSurface::new(&g, origin, Some(&terrain));
        let mut adjacent = GroundSurface::new(&g, [origin[0] + 128.0, origin[1]], None);
        for x in [126.25, 128.0, 129.75] {
            for z in [-1.5, 1.75, 63.8, 128.8] {
                let (y, n) = filled.plane(x, z);
                let (other, other_n) = adjacent.plane(x - 128.0, z);
                assert_eq!((y, n), (other, other_n));
                assert!(
                    (f64::from(y)
                        - walk_height(&g, origin[0] + f64::from(x), origin[1] + f64::from(z)))
                    .abs()
                        < 0.001
                );
            }
        }
    }
}
