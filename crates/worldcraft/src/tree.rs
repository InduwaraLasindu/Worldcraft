//! Original, deterministic botanical prototypes. Render-only: saved placements do not change.
use bevy::prelude::*;
use std::f32::consts::TAU;
use worldcraft_core::{generation::unit, mesh::MeshData, scatter::Species};

pub struct TreeMeshes {
    pub wood: MeshData,
    pub crown: MeshData,
    pub coarse: MeshData,
    pub wood_centers: Vec<[f32; 3]>,
    pub coarse_centers: Vec<[f32; 3]>,
}

impl TreeMeshes {
    fn branch(&mut self, a: Vec3, b: Vec3, ra: f32, rb: f32, sides: usize) {
        // Small twigs subtend only a few pixels even at walking distance.
        // Keep the full circular section on structural limbs and the trunk.
        let sides = if ra <= 0.055 { 3 } else { sides };
        tube(&mut self.wood, a, b, ra, rb, sides);
        for center in [a, b] {
            self.wood_centers
                .extend(std::iter::repeat_n(center.to_array(), sides + 1));
        }
    }
}

pub fn build(species: Species, variant: usize) -> TreeMeshes {
    let seed = 0x7472_6565 ^ (variant as u64 * 7919);
    let mut tree = TreeMeshes {
        wood: MeshData::default(),
        crown: MeshData::default(),
        coarse: MeshData::default(),
        wood_centers: Vec::new(),
        coarse_centers: Vec::new(),
    };
    let height = match species {
        Species::Pine => 13.0 + random(seed, 1, 0) * 2.2,
        Species::Birch => 12.5 + random(seed, 1, 0) * 1.6,
        Species::Acacia => 6.2 + random(seed, 1, 0) * 0.9,
        _ => 10.8 + random(seed, 1, 0) * 2.0,
    };
    let lean = Vec3::new(random(seed, 2, 0) - 0.5, 0.0, random(seed, 3, 0) - 0.5) * 0.8;
    let trunk_at = |t: f32| Vec3::Y * height * t + lean * t * t;
    let trunk_start = tree.wood.positions.len();
    for ring in 0..8 {
        let t = ring as f32 / 8.0;
        let next = (ring + 1) as f32 / 8.0;
        let radius = |t: f32| {
            0.035
                + (if species == Species::Birch {
                    0.16
                } else {
                    0.29
                }) * (1.0 - t).powf(1.1)
        };
        tree.branch(trunk_at(t), trunk_at(next), radius(t), radius(next), 9);
    }
    // Buttresses taper into the existing stem within two metres. Scale the
    // duplicated ring seams identically and correct the normals for the flare.
    let foot = match species {
        Species::Birch => 0.18,
        Species::Palm => 0.23,
        Species::Pine => 0.30,
        _ => 0.52,
    };
    for at in trunk_start..tree.wood.positions.len() {
        let p = Vec3::from_array(tree.wood.positions[at]);
        let center = Vec3::from_array(tree.wood_centers[at]);
        let radial = Vec3::new(p.x - center.x, 0.0, p.z - center.z);
        let angle = radial.z.atan2(radial.x);
        let lobes = 0.66 + 0.34 * (3.0 * angle + variant as f32 * 1.37).cos();
        let taper = (1.0 - center.y / 2.15).clamp(0.0, 1.0);
        let extra = foot * lobes * taper * taper;
        // Sink the open base below its footprint on a sloping walking
        // triangle, so the ground intersects the flare all around the stem.
        let footing = if center.y == 0.0 { 0.75 } else { 0.0 };
        tree.wood.positions[at] =
            (p + radial.normalize_or_zero() * extra - Vec3::Y * footing).to_array();
        tree.wood_centers[at][1] -= footing;
        if species == Species::Birch {
            let rough_base = taper * (0.48 + 0.24 * lobes);
            tree.wood.colors[at] = [
                1.0 - rough_base,
                1.0 - rough_base,
                1.0 - rough_base * 1.04,
                1.0,
            ];
        }
        let n = Vec3::from_array(tree.wood.normals[at]);
        tree.wood.normals[at] = (n + Vec3::Y * (2.0 * foot * lobes * taper / 2.15))
            .normalize()
            .to_array();
    }
    match species {
        Species::Pine => {
            // Irregular whorls with visible gaps and drooping boughs, never opaque cones.
            // Mirror the four distant profiles: open trunks, low skirts and a
            // narrow spire keep their silhouette as the camera approaches.
            let (skirt, spread, tiers) = [
                (0.18, 4.1, 12),
                (0.34, 3.8, 10),
                (0.13, 3.1, 13),
                (0.44, 4.3, 9),
            ][variant % 4];
            for tier in 0..tiers {
                let level = tier as f32 / (tiers - 1) as f32;
                let t = skirt + (0.94 - skirt) * level;
                let base = trunk_at(t);
                let radius = (1.0 - level).powf(0.78) * spread + 0.16;
                for bough in 0..6 {
                    let id = tier * 6 + bough;
                    let angle = bough as f32 * TAU / 6.0 + tier as f32 * 1.91 + variant as f32;
                    let dir = Vec3::new(angle.cos(), 0.0, angle.sin());
                    let reach = radius * (0.72 + random(seed, id, 4) * 0.40);
                    let tip = base + dir * reach + Vec3::Y * (-0.3 + random(seed, id, 5) * 0.5);
                    tree.branch(base, tip, 0.055 * (1.0 - t), 0.008, 5);
                    for spray in 0..8 {
                        let along = 0.16 + spray as f32 * 0.115;
                        let side = if spray % 2 == 0 { -1.0 } else { 1.0 };
                        let tangent = Vec3::new(-dir.z, 0.0, dir.x);
                        let center = base.lerp(tip, along) + tangent * side * reach * 0.13;
                        let size = 0.38 + (1.0 - t) * 0.28;
                        let u = (tangent * 0.85 + dir * side * 0.3) * size;
                        let v = (dir * 0.85 + Vec3::Y * 0.45) * size;
                        card_pair(
                            &mut tree,
                            center,
                            u,
                            v,
                            [0.68, 0.78, 0.57, 1.0],
                            spray % 3 == 0,
                            1.7,
                        );
                        // Crossed sprays retain volume from steep viewpoints.
                        card_pair(
                            &mut tree,
                            center,
                            u,
                            (Vec3::Y * 0.9 + dir * 0.2) * size,
                            [0.72, 0.85, 0.60, 1.0],
                            spray % 3 == 0,
                            1.7,
                        );
                    }
                }
            }
        }
        Species::Palm => {
            // A bowed rachis and segmented feathered fronds, with a branch-free stem.
            for frond in 0..13 {
                let angle = frond as f32 * TAU / 13.0 + variant as f32 * 0.7;
                let direction = Vec3::new(angle.cos(), 0.0, angle.sin());
                let width = Vec3::new(-direction.z, 0.0, direction.x);
                let reach = 3.5 + random(seed, frond, 11) * 0.9;
                let point = |t: f32| {
                    trunk_at(0.97) + direction * reach * t + Vec3::Y * (t * 2.3 - t * t * 3.2)
                };
                for segment in 0..6 {
                    let t = segment as f32 / 6.0;
                    let a = point(t);
                    let b = point(t + 1.0 / 6.0);
                    tree.branch(a, b, 0.022 * (1.0 - t), 0.008, 4);
                    let center = (a + b) * 0.5;
                    let halfwidth =
                        (0.28 + (t * std::f32::consts::PI).sin() * 0.36) * (1.0 - t * 0.4);
                    card_pair(
                        &mut tree,
                        center,
                        width * halfwidth,
                        (b - a) * 0.62,
                        [0.88, 0.95, 0.63, 1.0],
                        frond % 2 == 0,
                        1.35,
                    );
                }
            }
        }
        Species::Acacia => {
            // Original forked architecture supports an irregular flattened crown.
            for branch in 0..9 {
                let a = branch as f32 * 2.39996 + variant as f32 * 0.3;
                let dir = Vec3::new(a.cos(), 0.0, a.sin());
                let base = trunk_at(0.48 + random(seed, branch, 3) * 0.12);
                let reach = 3.1 + random(seed, branch, 4) * 1.6;
                let elbow = base + dir * reach * 0.48 + Vec3::Y * 1.9;
                let tip = base + dir * reach + Vec3::Y * (2.4 + random(seed, branch, 5) * 0.5);
                tree.branch(base, elbow, 0.13, 0.065, 7);
                tree.branch(elbow, tip, 0.065, 0.015, 5);
                for shoot in 0..4 {
                    let id = branch * 4 + shoot;
                    let angle = a + (shoot as f32 - 1.5) * 0.5;
                    let end = tip
                        + Vec3::new(angle.cos(), 0.1 + random(seed, id, 8) * 0.3, angle.sin())
                            * 0.9;
                    tree.branch(elbow.lerp(tip, 0.6), end, 0.025, 0.005, 4);
                    for cluster in 0..12 {
                        let angle = random(seed, id, cluster + 20) * TAU;
                        let radius = random(seed ^ 83, id, cluster + 20).sqrt() * 0.9;
                        let center = end
                            + Vec3::new(
                                angle.cos() * radius,
                                random(seed ^ 71, id, cluster + 20) * 0.36,
                                angle.sin() * radius,
                            );
                        let size = 0.45 + random(seed ^ 36, id, cluster + 20) * 0.20;
                        card_pair(
                            &mut tree,
                            center,
                            Vec3::new(angle.cos(), 0.08, angle.sin()) * size,
                            Vec3::new(-angle.sin() * 0.9, 0.28, angle.cos() * 0.9) * size,
                            [0.78, 0.88, 0.57, 1.0],
                            cluster % 3 == 0,
                            1.65,
                        );
                    }
                }
            }
        }
        _ => {
            let birch = species == Species::Birch;
            let branches = if birch { 11 } else { 16 };
            for branch in 0..branches {
                let angle = branch as f32 * 2.39996 + variant as f32;
                let t = if birch {
                    0.32 + branch as f32 * 0.051
                } else {
                    0.35 + branch as f32 * 0.035
                };
                let dir = Vec3::new(angle.cos(), 0.0, angle.sin());
                let base = trunk_at(t);
                let crown_width = [0.88, 1.12, 0.78, 1.23][variant % 4];
                let reach = crown_width
                    * ((if birch { 1.8 } else { 3.0 }) + random(seed, branch, 5))
                    * (1.1 - t * 0.4);
                let elbow = base + dir * reach * 0.62 + Vec3::Y * 0.65;
                let tip = base + dir * reach + Vec3::Y * (1.0 + random(seed, branch, 6));
                tree.branch(base, elbow, 0.10 * (1.1 - t), 0.045, 6);
                tree.branch(elbow, tip, 0.045, 0.012, 5);
                for shoot in 0..3 {
                    let id = branch * 3 + shoot;
                    let offset = angle + (shoot as f32 - 1.0) * 0.9;
                    let endpoint = tip
                        + Vec3::new(
                            offset.cos() * 0.85,
                            0.4 + random(seed, id, 7) * 0.6,
                            offset.sin() * 0.85,
                        );
                    tree.branch(elbow.lerp(tip, 0.5), endpoint, 0.024, 0.006, 4);
                    for cluster in 0..16 {
                        let a = random(seed, id, cluster + 20) * TAU;
                        let radial = random(seed ^ 91, id, cluster + 20).sqrt();
                        let center = endpoint
                            + Vec3::new(
                                a.cos() * radial,
                                random(seed ^ 42, id, cluster + 20) * 1.2 - 0.5,
                                a.sin() * radial,
                            );
                        let size = 0.50 + random(seed ^ 72, id, cluster + 20) * 0.22;
                        let u = Vec3::new(a.cos(), 0.18, a.sin()) * size;
                        let v = Vec3::new(-a.sin() * 0.55, 0.75, a.cos() * 0.55) * size;
                        let light = 0.75 + random(seed, id, cluster + 50) * 0.25;
                        card_pair(
                            &mut tree,
                            center,
                            u,
                            v,
                            [
                                light * if birch { 0.85 } else { 0.91 },
                                light,
                                light * if birch { 0.58 } else { 0.73 },
                                1.0,
                            ],
                            cluster % 3 == 0,
                            1.65,
                        );
                    }
                }
            }
        }
    }
    crate::canopy::shade(&mut tree.crown, &mut tree.coarse, &mut tree.wood);
    // Conservative visibility bounds also cover the shader's maximum 0.27 m
    // local breeze displacement. Unindexed corners add no rendered triangles.
    for (mesh, morph) in [(&mut tree.crown, true), (&mut tree.coarse, false)] {
        let (lo, hi) = mesh.positions.iter().map(|p| Vec3::from_array(*p)).fold(
            (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
            |(lo, hi), p| (lo.min(p), hi.max(p)),
        );
        for p in [lo - Vec3::splat(0.5), hi + Vec3::splat(0.5)] {
            mesh.vertex(p.to_array(), Vec3::Y.to_array(), [1.0; 4], [0.0; 2]);
            if morph {
                mesh.morph.push([p.x, p.y, p.z, 0.0]);
                mesh.morph_normals.push(Vec3::Y.to_array());
                mesh.morph_colors.push([1.0, 1.0, 1.0, 0.0]);
            } else {
                tree.coarse_centers.push(p.to_array());
            }
        }
    }
    tree
}
fn random(seed: u64, a: i64, b: i64) -> f32 {
    unit(seed, a, b) as f32
}

pub fn tube(m: &mut MeshData, a: Vec3, b: Vec3, ra: f32, rb: f32, sides: usize) {
    let direction = (b - a).normalize();
    let u = direction
        .cross(if direction.y.abs() < 0.99 {
            Vec3::Y
        } else {
            Vec3::X
        })
        .normalize();
    let v = direction.cross(u);
    let start = m.positions.len() as u32;
    for (p, radius, along) in [(a, ra, 0.0), (b, rb, a.distance(b))] {
        // Duplicate the seam so interpolation never stretches across the bark wrap.
        for side in 0..=sides {
            let angle = side as f32 / sides as f32 * TAU;
            let n = u * angle.cos() + v * angle.sin();
            let normal = (n + direction * ((ra - rb) / a.distance(b))).normalize();
            m.vertex(
                (p + n * radius).to_array(),
                normal.to_array(),
                [1.0; 4],
                [
                    side as f32 / sides as f32 * TAU * (ra + rb) * 0.5 / 0.6,
                    (a.y + along) / 1.8,
                ],
            );
        }
    }
    let row = (sides + 1) as u32;
    for side in 0..sides as u32 {
        let i = start + side;
        m.indices
            .extend_from_slice(&[i, i + 1, i + row, i + 1, i + row + 1, i + row]);
    }
}

pub fn card(m: &mut MeshData, center: Vec3, u: Vec3, v: Vec3, color: [f32; 4]) {
    let start = m.positions.len() as u32;
    // Bent-sheet lighting normal softens flat card highlights without an expensive mesh.
    let face = u.cross(v).normalize();
    for (offset, uv) in [
        (-u - v, [0.0, 1.0]),
        (u - v, [1.0, 1.0]),
        (u + v, [1.0, 0.0]),
        (-u + v, [0.0, 0.0]),
    ] {
        let n = (face * 0.8 + offset.normalize() * 0.2).normalize();
        m.vertex((center + offset).to_array(), n.to_array(), color, uv);
    }
    m.indices
        .extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 3]);
}
fn card_pair(
    tree: &mut TreeMeshes,
    center: Vec3,
    u: Vec3,
    v: Vec3,
    color: [f32; 4],
    keep: bool,
    growth: f32,
) {
    let start = tree.crown.positions.len();
    let parent_start = tree.coarse.positions.len();
    card(&mut tree.crown, center, u, v, color);
    if keep {
        card(&mut tree.coarse, center, u * growth, v * growth, color);
        tree.coarse_centers.extend([center.to_array(); 4]);
    }
    for i in 0..4 {
        let p = if keep {
            tree.coarse.positions[parent_start + i]
        } else {
            center.to_array()
        };
        tree.crown.morph.push([p[0], p[1], p[2], 1.0]);
        tree.crown.morph_normals.push(if keep {
            tree.coarse.normals[parent_start + i]
        } else {
            tree.crown.normals[start + i]
        });
        tree.crown.morph_colors.push(color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn botanical_prototypes_are_bounded_valid_and_repeatable() {
        for species in Species::TREES {
            for variant in 0..4 {
                let tree = build(species, variant);
                assert_eq!(
                    tree.crown.positions,
                    build(species, variant).crown.positions
                );
                for (mesh, centers) in [
                    (&tree.wood, &tree.wood_centers),
                    (&tree.coarse, &tree.coarse_centers),
                ] {
                    assert_eq!(mesh.positions.len(), centers.len());
                    for triangle in mesh.indices.chunks_exact(3) {
                        let [a, b, c]: [Vec3; 3] = std::array::from_fn(|i| {
                            Vec3::from_array(centers[triangle[i] as usize])
                        });
                        assert!(
                            (b - a).cross(c - a).length_squared() < 1e-12,
                            "Visible triangle at outer cutoff: {species:?}/{variant}"
                        );
                    }
                }
                for m in [&tree.wood, &tree.crown, &tree.coarse] {
                    assert!(!m.indices.is_empty());
                    assert!(m.indices.len() / 3 < 5000, "Unbounded prototype geometry");
                    assert!(m.indices.iter().all(|&i| (i as usize) < m.positions.len()));
                    assert_eq!(m.positions.len(), m.normals.len());
                    assert_eq!(m.positions.len(), m.colors.len());
                    assert_eq!(m.positions.len(), m.uv.len());
                    for p in &m.positions {
                        assert!(p.iter().all(|v| v.is_finite() && v.abs() < 24.0));
                    }
                    for n in &m.normals {
                        assert!((Vec3::from_array(*n).length() - 1.0).abs() < 0.001);
                    }
                }
                assert!(tree.coarse.indices.len() < tree.crown.indices.len());
            }
        }
        assert_ne!(
            build(Species::Pine, 0).wood.positions,
            build(Species::Palm, 0).wood.positions
        );
        assert_ne!(
            build(Species::Broadleaf, 0).crown.positions,
            build(Species::Broadleaf, 1).crown.positions
        );
    }
}
