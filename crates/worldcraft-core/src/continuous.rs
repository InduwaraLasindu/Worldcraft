//! Distance-continuous near terrain. Topology is a bounded internal optimization.
//! A tile coarsens only after every vertex has reached the same parent lattice.
use crate::{ChunkCoord, ChunkKey, Generator, MeshData, CHUNK_SIZE};
pub fn end_distance(lod: u8) -> f64 {
    512.0 * f64::from(1u32 << lod.min(3))
}
pub fn tile_distance(coord: ChunkCoord, eye: [f64; 2]) -> f64 {
    let o = coord.origin();
    let x = (eye[0] - eye[0].clamp(o[0], o[0] + CHUNK_SIZE)).abs();
    let z = (eye[1] - eye[1].clamp(o[1], o[1] + CHUNK_SIZE)).abs();
    x.max(z)
}
pub fn lod_at(coord: ChunkCoord, eye: [f64; 2]) -> u8 {
    let d = tile_distance(coord, eye);
    (0..3).find(|&l| d < end_distance(l)).unwrap_or(3)
}
/// CPU reference for the GPU morph; absolute eye and vertex coordinates use the same origin.
pub fn morph_factor(distance: f64, end: f64) -> f64 {
    if end <= 0.0 {
        return 0.0;
    }
    let t = ((distance - (end - 192.0)) / 192.0).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
pub fn build_continuous(generator: &Generator, key: ChunkKey) -> MeshData {
    // Boundaries are governed by one observer-distance function, not per-tile edge snapping.
    let mut m = crate::build_mesh(
        generator,
        ChunkKey {
            edges: [key.lod; 4],
            ..key
        },
    );
    let end = if key.lod < 3 {
        end_distance(key.lod) as f32
    } else {
        0.0
    };
    let cells = 32usize >> key.lod.min(3);
    let row = cells + 1;
    let step = CHUNK_SIZE / cells as f64;
    for i in 0..m.positions.len() {
        let p = m.positions[i];
        let x = (f64::from(p[0]) / step).round() as usize / 2 * 2;
        let z = (f64::from(p[2]) / step).round() as usize / 2 * 2;
        let j = z * row + x;
        let mut parent = m.positions[j];
        if i >= row * row {
            parent[1] -= 48.0;
        }
        m.morph.push([parent[0], parent[1], parent[2], end]);
        m.morph_normals.push(m.normals[j]);
        m.morph_colors.push(m.colors[j]);
    }
    m
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn distance_and_lod_work_in_negative_world_and_at_boundaries() {
        let c = ChunkCoord { x: -1, z: -1 };
        assert_eq!(tile_distance(c, [-64.0, -64.0]), 0.0);
        assert_eq!(lod_at(c, [-640.0, -64.0]), 1);
        assert_eq!(lod_at(c, [-639.99, -64.0]), 0);
        assert_eq!(morph_factor(320.0, 512.0), 0.0);
        assert_eq!(morph_factor(512.0, 512.0), 1.0);
        assert!((morph_factor(416.0, 512.0) - 0.5).abs() < 1e-12);
    }
    #[test]
    fn fully_morphed_grid_is_exactly_the_parent_mesh_with_same_attributes() {
        let g = Generator::new(crate::GeneratorSettings::default(), vec![]);
        for lod in 0..3 {
            let key = ChunkKey {
                coord: ChunkCoord { x: -2, z: 1 },
                lod,
                edges: [lod; 4],
                revision: 0,
            };
            let fine = build_continuous(&g, key);
            let coarse = build_continuous(
                &g,
                ChunkKey {
                    lod: lod + 1,
                    ..key
                },
            );
            let cells = 32usize >> lod;
            let parent_row = cells / 2 + 1;
            for z in 0..=cells {
                for x in 0..=cells {
                    let i = z * (cells + 1) + x;
                    let j = (z / 2) * parent_row + x / 2;
                    assert_eq!(&fine.morph[i][..3], &coarse.positions[j]);
                    assert_eq!(fine.morph_normals[i], coarse.normals[j]);
                    assert_eq!(fine.morph_colors[i], coarse.colors[j]);
                }
            }
        }
    }
    #[test]
    fn neighboring_resolution_edges_have_identical_morphed_vertices() {
        let g = Generator::new(crate::GeneratorSettings::default(), vec![]);
        let a = build_continuous(
            &g,
            ChunkKey {
                coord: ChunkCoord { x: -1, z: 0 },
                lod: 0,
                edges: [0; 4],
                revision: 0,
            },
        );
        let b = build_continuous(
            &g,
            ChunkKey {
                coord: ChunkCoord { x: 0, z: 0 },
                lod: 1,
                edges: [1; 4],
                revision: 0,
            },
        );
        for z in 0..=32 {
            let p = a.morph[z * 33 + 32];
            let q = b.positions[(z / 2) * 17];
            assert_eq!(p[0] - 128.0, q[0]);
            assert_eq!(p[1], q[1]);
            assert_eq!(p[2], q[2]);
        }
    }
}

#[cfg(test)]
mod perimeter_tests {
    use super::*;
    fn position(m: &MeshData, i: usize, origin: [f64; 2], eye: [f64; 2]) -> [f64; 3] {
        let p = m.positions[i];
        let t = m.morph[i];
        let distance = (origin[0] + f64::from(p[0]) - eye[0])
            .abs()
            .max((origin[1] + f64::from(p[2]) - eye[1]).abs());
        let weight = morph_factor(distance, f64::from(t[3])) * f64::from(m.morph_colors[i][3]);
        std::array::from_fn(|c| {
            f64::from(p[c]) * (1.0 - weight)
                + f64::from(t[c]) * weight
                + if c == 0 {
                    origin[0]
                } else if c == 2 {
                    origin[1]
                } else {
                    0.0
                }
        })
    }
    #[test]
    fn horizon_collar_meets_near_grid_at_every_radius_including_morph_bands() {
        let g = Generator::new(crate::GeneratorSettings::default(), vec![]);
        let center = ChunkCoord { x: -3, z: 2 };
        let origin = center.origin();
        for radius in [2, 3, 4, 6, 7, 8, 9, 12] {
            let far = crate::vista::build_continuous_vista(&g, center, radius, 2048.0);
            let east = (center.x + radius + 1) as f64 * CHUNK_SIZE;
            let coord = ChunkCoord {
                x: center.x + radius,
                z: center.z,
            };
            for offset in [5.0, 64.0, 123.0] {
                let eye = [origin[0] + offset, origin[1] + offset];
                let lod = lod_at(coord, eye);
                let near = build_continuous(
                    &g,
                    ChunkKey {
                        coord,
                        lod,
                        edges: [lod; 4],
                        revision: 0,
                    },
                );
                let cells = 32usize >> lod;
                let step = CHUNK_SIZE / cells as f64;
                let mut count = 0;
                for i in 0..far.positions.len() {
                    let p = far.positions[i];
                    let wx = origin[0] + f64::from(p[0]);
                    let wz = origin[1] + f64::from(p[2]);
                    if wx != east || wz < origin[1] || wz > origin[1] + CHUNK_SIZE {
                        continue;
                    }
                    let iz = ((wz - origin[1]) / step).floor() as usize;
                    let j = iz * (cells + 1) + cells;
                    let a = position(&far, i, origin, eye);
                    let b = position(&near, j, coord.origin(), eye);
                    for c in 0..3 {
                        assert!(
                            (a[c] - b[c]).abs() < 0.002,
                            "radius={radius} offset={offset} lod={lod} far={a:?} near={b:?}"
                        );
                    }
                    count += 1;
                }
                assert!(count > 0);
            }
        }
    }
}
