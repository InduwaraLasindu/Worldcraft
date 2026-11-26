//! Leaf-area voxels approximate how much of the sky reaches each part of a crown.
//! Built once per shared prototype. Only the resulting vertex pigments reach the GPU.
use bevy::prelude::*;
use worldcraft_core::MeshData;

const SIDE: usize = 16;
const CELLS: usize = SIDE * SIDE * SIDE;

struct CrownLight {
    origin: Vec3,
    step: f32,
    exposure: Vec<f32>,
}

impl CrownLight {
    fn bake(crown: &MeshData) -> Self {
        let (lo, hi) = crown.positions.iter().map(|&p| Vec3::from_array(p)).fold(
            (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
            |(lo, hi), p| (lo.min(p), hi.max(p)),
        );
        let step = (hi - lo).max_element().max(0.1) / (SIDE - 3) as f32;
        let origin = lo - Vec3::splat(step);
        let mut density = vec![0.0_f32; CELLS];
        // Splat the area of each cutout spray to four interior samples. This
        // captures whorls, gaps and asymmetric branches in the actual model.
        for card in crown.positions.chunks_exact(4) {
            let a = Vec3::from_array(card[0]);
            let u = Vec3::from_array(card[1]) - a;
            let v = Vec3::from_array(card[3]) - a;
            let area = u.cross(v).length() * 0.25 / (step * step);
            for s in [0.25, 0.75] {
                for t in [0.25, 0.75] {
                    let cell = ((a + s * u + t * v - origin) / step).round().as_uvec3();
                    density[index(cell.x as usize, cell.y as usize, cell.z as usize)] += area;
                }
            }
        }
        let mut exposure = vec![0.0; CELLS];
        let mut beam = vec![1.0; CELLS];
        for (dx, dz, weight) in [
            (0, 0, 0.36),
            (1, 0, 0.16),
            (-1, 0, 0.16),
            (0, 1, 0.16),
            (0, -1, 0.16),
        ] {
            let path = if dx == 0 && dz == 0 {
                1.0
            } else {
                std::f32::consts::SQRT_2
            };
            for y in (0..SIDE).rev() {
                for z in 0..SIDE {
                    for x in 0..SIDE {
                        let i = index(x, y, z);
                        let (sx, sz) = (x as i32 + dx, z as i32 + dz);
                        let incident = if y + 1 < SIDE
                            && (0..SIDE as i32).contains(&sx)
                            && (0..SIDE as i32).contains(&sz)
                        {
                            beam[index(sx as usize, y + 1, sz as usize)]
                        } else {
                            1.0
                        };
                        // Account for empty space within alpha-tested leaf cards.
                        let transmitted = incident / (1.0 + 0.18 * path * density[i]);
                        exposure[i] += weight * (incident + transmitted) * 0.5;
                        beam[i] = transmitted;
                    }
                }
            }
        }
        Self {
            origin,
            step,
            exposure,
        }
    }

    fn sample(&self, p: [f32; 3]) -> f32 {
        let q = ((Vec3::from_array(p) - self.origin) / self.step)
            .clamp(Vec3::ZERO, Vec3::splat((SIDE - 1) as f32));
        let cell = q.floor().as_uvec3();
        let f = q - q.floor();
        let mut light = 0.0;
        for (dz, wz) in [(0, 1.0 - f.z), (1, f.z)] {
            for (dy, wy) in [(0, 1.0 - f.y), (1, f.y)] {
                for (dx, wx) in [(0, 1.0 - f.x), (1, f.x)] {
                    let at = index(
                        (cell.x as usize + dx).min(SIDE - 1),
                        (cell.y as usize + dy).min(SIDE - 1),
                        (cell.z as usize + dz).min(SIDE - 1),
                    );
                    light += wx * wy * wz * self.exposure[at];
                }
            }
        }
        light.clamp(0.0, 1.0)
    }

    fn shade(&self, color: &mut [f32; 4], p: [f32; 3]) {
        let sky = self.sample(p);
        // Modest contrast leaves room for directional light and real shadows.
        for (channel, base) in color[..3].iter_mut().zip([0.56, 0.65, 0.70]) {
            *channel *= base + (1.06 - base) * sky;
        }
    }
}

fn index(x: usize, y: usize, z: usize) -> usize {
    (y * SIDE + z) * SIDE + x
}

pub fn shade(crown: &mut MeshData, coarse: &mut MeshData, wood: &mut MeshData) {
    let light = CrownLight::bake(crown);
    for mesh in [coarse, wood, &mut *crown] {
        for (color, &position) in mesh.colors.iter_mut().zip(&mesh.positions) {
            light.shade(color, position);
        }
    }
    // Paired LODs must agree exactly at the topology change. The morph color
    // alpha is a geometry weight and must survive the pigment bake.
    for (color, p) in crown.morph_colors.iter_mut().zip(&crown.morph) {
        light.shade(color, [p[0], p[1], p[2]]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overhead_foliage_shades_the_interior_and_preserves_open_sky() {
        let mut crown = MeshData::default();
        for _ in 0..8 {
            for z in -3..=3 {
                for x in -3..=3 {
                    crate::tree::card(
                        &mut crown,
                        Vec3::new(x as f32, 4.0, z as f32),
                        Vec3::X * 0.5,
                        Vec3::Z * 0.5,
                        [1.0; 4],
                    );
                }
            }
        }
        let field = CrownLight::bake(&crown);
        let below = field.sample([0.0, 3.8, 0.0]);
        let above = field.sample([0.0, 5.0, 0.0]);
        assert!(below + 0.20 < above && above > 0.99);
        assert!(field
            .exposure
            .iter()
            .all(|v| v.is_finite() && (0.0..=1.00001).contains(v)));
    }
}
