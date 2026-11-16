use crate::surface::{SurfaceDetail, SurfaceMaterial};
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
    prelude::*,
};
use worldcraft_core::{
    generation::unit,
    mesh::MeshData,
    scatter::{Instance, Species},
};

#[derive(Resource)]
pub struct Nature {
    pub trunks: Vec<Handle<Mesh>>,
    pub crowns: Vec<Handle<Mesh>>,
    pub pines: Vec<Handle<Mesh>>,
    pub palms: Vec<Handle<Mesh>>,
    pub rocks: Vec<Handle<Mesh>>,
    pub coarse_crowns: Vec<Handle<Mesh>>,
    pub coarse_pines: Vec<Handle<Mesh>>,
    pub coarse_palms: Vec<Handle<Mesh>>,
    pub bark: Handle<StandardMaterial>,
    pub leaf: Handle<SurfaceMaterial>,
    pub stone: Handle<StandardMaterial>,
    pub ground: Handle<StandardMaterial>,
    pub terrain: Handle<SurfaceMaterial>,
    pub water: Handle<SurfaceMaterial>,
}
impl Nature {
    pub fn crown(&self, species: Species, variant: usize, coarse: bool) -> &Handle<Mesh> {
        let family = match (species, coarse) {
            (Species::Pine, false) => &self.pines,
            (Species::Pine, true) => &self.coarse_pines,
            (Species::Palm, false) => &self.palms,
            (Species::Palm, true) => &self.coarse_palms,
            (_, false) => &self.crowns,
            (_, true) => &self.coarse_crowns,
        };
        &family[variant]
    }
}
pub fn mesh(mut m: MeshData) -> Mesh {
    if !m.morph.is_empty() {
        // Bevy derives bounds from base positions. Include the whole target
        // domain using two unindexed vertices, so growing crown cards cannot
        // disappear at a frustum edge. Indexed/rendered geometry is unchanged.
        let (lo, hi) = m.morph.iter().fold(
            (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
            |(lo, hi), p| {
                let p = Vec3::new(p[0], p[1], p[2]);
                (lo.min(p), hi.max(p))
            },
        );
        for p in [lo, hi] {
            m.vertex(p.to_array(), [0.0, 1.0, 0.0], [1.0; 4], [0.0; 2]);
            m.morph.push([p.x, p.y, p.z, 0.0]);
            m.morph_normals.push([0.0, 1.0, 0.0]);
            m.morph_colors.push([1.0, 1.0, 1.0, 0.0]);
        }
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, m.positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, m.normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, m.colors)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, m.uv)
    .with_inserted_indices(Indices::U32(m.indices));
    if !m.morph.is_empty() {
        mesh.insert_attribute(crate::surface::MORPH, m.morph);
        mesh.insert_attribute(crate::surface::MORPH_NORMAL, m.morph_normals);
        mesh.insert_attribute(crate::surface::MORPH_COLOR, m.morph_colors);
    }
    mesh
}
pub fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut surfaces: ResMut<Assets<SurfaceMaterial>>,
) {
    let mut trunks = Vec::new();
    let mut crowns = Vec::new();
    let mut pines = Vec::new();
    let mut palms = Vec::new();
    let mut rocks = Vec::new();
    let mut coarse_crowns = Vec::new();
    let mut coarse_pines = Vec::new();
    let mut coarse_palms = Vec::new();
    for variant in 0..4 {
        let seed = 999 + variant;
        let mut wood = MeshData::default();
        tube(
            &mut wood,
            Vec3::ZERO,
            Vec3::new(0.15, 9.0, 0.0),
            0.31,
            0.08,
            9,
            [0.3, 0.21, 0.12, 1.0],
        );
        let mut leaves = MeshData::default();
        let mut coarse_leaves = MeshData::default();
        for branch in 0..10 {
            let angle = branch as f32 * 2.4 + variant as f32;
            let h = 3.6 + branch as f32 * 0.45;
            let tip = Vec3::new(angle.cos() * 2.6, h + 1.5, angle.sin() * 2.6);
            tube(
                &mut wood,
                Vec3::new(0.0, h, 0.0),
                tip,
                0.1,
                0.025,
                6,
                [0.25, 0.18, 0.1, 1.0],
            );
            for leaf in 0..54 {
                let a = unit(seed, branch, leaf) as f32 * std::f32::consts::TAU;
                let r = (unit(seed ^ 7, branch, leaf) as f32).sqrt() * 1.9;
                let p = tip
                    + Vec3::new(
                        a.cos() * r,
                        unit(seed ^ 8, branch, leaf) as f32 * 2.1 - 0.7,
                        a.sin() * r,
                    );
                crown_card(
                    &mut leaves,
                    &mut coarse_leaves,
                    p,
                    0.38 + unit(seed ^ 9, branch, leaf) as f32 * 0.35,
                    a,
                    leaf % 4 == 0,
                );
            }
        }
        trunks.push(meshes.add(mesh(wood)));
        crowns.push(meshes.add(mesh(leaves)));
        coarse_crowns.push(meshes.add(mesh(coarse_leaves)));
        let mut pine = MeshData::default();
        let mut coarse_pine = MeshData::default();
        // Uneven boughs and needle clumps break up the old stacked-cone silhouette.
        for layer in 0..10 {
            let y = 2.0 + layer as f32 * 0.72;
            let radius =
                (3.0 - layer as f32 * 0.25) * (0.88 + unit(seed ^ 81, layer, 0) as f32 * 0.18);
            let start = pine.positions.len();
            cone(
                &mut pine,
                Vec3::new(0.0, y, 0.0),
                radius * 0.72,
                1.35,
                10,
                [0.12, 0.25, 0.11, 1.0],
            );
            fixed_crown_part(&mut pine, &mut coarse_pine, start);
            for branch in 0..8 {
                let angle = branch as f32 * std::f32::consts::TAU / 8.0
                    + layer as f32 * 1.7
                    + variant as f32;
                for clump in 0..4 {
                    let t = 0.35 + clump as f32 * 0.21;
                    let p = Vec3::new(
                        angle.cos() * radius * t,
                        y + 0.45 - t * 0.15,
                        angle.sin() * radius * t,
                    );
                    crown_card(
                        &mut pine,
                        &mut coarse_pine,
                        p,
                        0.42 + (1.0 - t) * 0.28,
                        angle,
                        clump % 2 == 0,
                    );
                }
            }
        }
        pines.push(meshes.add(mesh(pine)));
        coarse_pines.push(meshes.add(mesh(coarse_pine)));
        let mut palm = MeshData::default();
        let mut coarse_palm = MeshData::default();
        for branch in 0..13 {
            let a = branch as f32 * std::f32::consts::TAU / 13.0;
            for leaf in 0..9 {
                let t = leaf as f32 / 8.0;
                crown_card(
                    &mut palm,
                    &mut coarse_palm,
                    Vec3::new(
                        a.cos() * t * 4.0,
                        8.5 + t * 1.8 - t * t * 2.8,
                        a.sin() * t * 4.0,
                    ),
                    0.55,
                    a,
                    leaf % 3 == 0,
                );
            }
        }
        palms.push(meshes.add(mesh(palm)));
        coarse_palms.push(meshes.add(mesh(coarse_palm)));
        let mut rock = Sphere::new(1.0).mesh().ico(2).unwrap();
        if let Some(VertexAttributeValues::Float32x3(positions)) =
            rock.attribute_mut(Mesh::ATTRIBUTE_POSITION)
        {
            for (i, p) in positions.iter_mut().enumerate() {
                let n = unit(seed ^ 93, i as i64, variant as i64) as f32;
                let scale = 0.82 + n * 0.32;
                p[0] *= scale * (1.05 + variant as f32 * 0.09);
                p[1] *= scale * (0.65 + variant as f32 * 0.06);
                p[2] *= scale;
            }
        }
        rock.compute_smooth_normals();
        rocks.push(meshes.add(rock));
    }
    commands.insert_resource(Nature {
        trunks,
        crowns,
        pines,
        palms,
        rocks,
        coarse_crowns,
        coarse_pines,
        coarse_palms,
        bark: materials.add(StandardMaterial {
            base_color: Color::srgb(0.5, 0.4, 0.29),
            perceptual_roughness: 0.95,
            ..default()
        }),
        leaf: surfaces.add(SurfaceMaterial {
            base: StandardMaterial {
                base_color: Color::srgb(0.31, 0.52, 0.17),
                perceptual_roughness: 0.85,
                cull_mode: None,
                double_sided: true,
                diffuse_transmission: 0.25,
                ..default()
            },
            extension: SurfaceDetail {
                parameters: Vec4::new(2.0, 0.0, 0.0, 0.0),
                origin: Vec4::ZERO,
                observer: Vec4::new(0.0, 0.0, 0.0, 128.0),
            },
        }),
        stone: materials.add(StandardMaterial {
            base_color: Color::srgb(0.35, 0.33, 0.28),
            perceptual_roughness: 0.94,
            ..default()
        }),
        ground: materials.add(StandardMaterial {
            base_color: Color::WHITE,
            perceptual_roughness: 0.95,
            cull_mode: None,
            double_sided: true,
            ..default()
        }),
        terrain: surfaces.add(SurfaceMaterial {
            base: StandardMaterial {
                base_color: Color::WHITE,
                perceptual_roughness: 0.92,
                ..default()
            },
            extension: SurfaceDetail {
                parameters: Vec4::new(0.0, 0.0, 1.0, 0.0),
                origin: Vec4::ZERO,
                observer: Vec4::ZERO,
            },
        }),
        water: surfaces.add(SurfaceMaterial {
            base: StandardMaterial {
                base_color: Color::srgba(0.055, 0.27, 0.32, 0.72),
                perceptual_roughness: 0.16,
                metallic: 0.12,
                reflectance: 0.65,
                alpha_mode: AlphaMode::Opaque,
                cull_mode: None,
                ..default()
            },
            extension: SurfaceDetail {
                parameters: Vec4::new(1.0, 0.0, 1.0, 0.0),
                origin: Vec4::ZERO,
                observer: Vec4::ZERO,
            },
        }),
    });
}
fn tube(m: &mut MeshData, a: Vec3, b: Vec3, ra: f32, rb: f32, sides: usize, color: [f32; 4]) {
    let direction = (b - a).normalize();
    let u = direction
        .cross(if direction.dot(Vec3::Y).abs() < 0.99 {
            Vec3::Y
        } else {
            Vec3::X
        })
        .normalize();
    let v = direction.cross(u);
    let start = m.positions.len() as u32;
    for (p, r) in [(a, ra), (b, rb)] {
        for i in 0..sides {
            let angle = i as f32 / sides as f32 * std::f32::consts::TAU;
            let n = u * angle.cos() + v * angle.sin();
            m.vertex(
                (p + n * r).to_array(),
                n.to_array(),
                color,
                [i as f32 / sides as f32, p.y],
            );
        }
    }
    for i in 0..sides {
        let j = (i + 1) % sides;
        let (a, b, c, d) = (
            start + i as u32,
            start + j as u32,
            start + sides as u32 + i as u32,
            start + sides as u32 + j as u32,
        );
        m.indices.extend_from_slice(&[a, b, c, b, d, c]);
    }
}
fn cone(m: &mut MeshData, p: Vec3, radius: f32, height: f32, sides: usize, color: [f32; 4]) {
    tube(m, p, p + Vec3::Y * height, radius, 0.02, sides, color);
}
fn leaf_card(m: &mut MeshData, p: Vec3, size: f32, angle: f32) {
    let u = Vec3::new(angle.cos(), 0.25, angle.sin()) * size;
    let v = Vec3::new(-angle.sin() * 0.5, 0.5, angle.cos() * 0.5) * size;
    let n = u.cross(v).normalize();
    let a = m.vertex(
        (p - u).to_array(),
        n.to_array(),
        [0.8, 1.0, 0.65, 1.0],
        [0.0, 0.5],
    );
    let b = m.vertex(
        (p + v).to_array(),
        n.to_array(),
        [0.9, 1.0, 0.7, 1.0],
        [0.5, 0.0],
    );
    let c = m.vertex(
        (p + u).to_array(),
        n.to_array(),
        [0.7, 0.9, 0.6, 1.0],
        [1.0, 0.5],
    );
    let d = m.vertex(
        (p - v).to_array(),
        n.to_array(),
        [0.7, 0.9, 0.6, 1.0],
        [0.5, 1.0],
    );
    m.indices.extend_from_slice(&[a, b, c, a, c, d]);
}
fn fixed_crown_part(fine: &mut MeshData, coarse: &mut MeshData, start: usize) {
    let base = coarse.positions.len() as u32;
    for i in start..fine.positions.len() {
        let p = fine.positions[i];
        fine.morph.push([p[0], p[1], p[2], 1.0]);
        fine.morph_normals.push(fine.normals[i]);
        fine.morph_colors.push(fine.colors[i]);
        coarse.vertex(p, fine.normals[i], fine.colors[i], fine.uv[i]);
    }
    coarse.indices.extend(
        fine.indices
            .iter()
            .filter(|&&i| i >= start as u32)
            .map(|&i| base + i - start as u32),
    );
}

/// Non-retained cards collapse to zero area; retained cards grow into exactly
/// the simplified mesh. No alpha checkerboard is needed at the topology switch.
fn crown_card(
    fine: &mut MeshData,
    coarse: &mut MeshData,
    center: Vec3,
    size: f32,
    yaw: f32,
    keep: bool,
) {
    let start = fine.positions.len();
    let parent_start = coarse.positions.len();
    leaf_card(fine, center, size, yaw);
    if keep {
        leaf_card(coarse, center, size * 1.6, yaw);
    }
    for i in 0..4 {
        let p = if keep {
            coarse.positions[parent_start + i]
        } else {
            center.to_array()
        };
        // parent.w == 1 identifies the crown morph domain; terrain uses metres.
        fine.morph.push([p[0], p[1], p[2], 1.0]);
        fine.morph_normals.push(if keep {
            coarse.normals[parent_start + i]
        } else {
            fine.normals[start + i]
        });
        fine.morph_colors.push(if keep {
            coarse.colors[parent_start + i]
        } else {
            fine.colors[start + i]
        });
    }
}

pub fn ground_batch(instances: &[Instance], origin: [f64; 2]) -> (Vec3, MeshData) {
    let mut mesh = ground_mesh(instances, origin);
    // CPU range culling and the shader both measure from the entity origin.
    // Use a real 3D anchor, including terrain altitude, instead of the tile corner.
    let center = instances.iter().fold(Vec3::ZERO, |sum, instance| {
        sum + Vec3::new(
            (instance.position[0] - origin[0]) as f32,
            instance.position[1] as f32,
            (instance.position[2] - origin[1]) as f32,
        )
    }) / instances.len().max(1) as f32;
    for p in &mut mesh.positions {
        *p = (Vec3::from_array(*p) - center).to_array();
    }
    (center, mesh)
}

pub fn ground_mesh(instances: &[Instance], origin: [f64; 2]) -> MeshData {
    let mut m = MeshData::default();
    for i in instances {
        let p = Vec3::new(
            (i.position[0] - origin[0]) as f32,
            i.position[1] as f32,
            (i.position[2] - origin[1]) as f32,
        );
        match i.species {
            Species::Grass => {
                for blade in 0..7 {
                    let angle = i.yaw + blade as f32 * 2.4;
                    let u = Vec3::new(angle.cos(), 0.0, angle.sin());
                    let base = p + u * (blade as f32 * 0.05);
                    let height = i.scale * (0.45 + blade as f32 * 0.07);
                    let n = Vec3::new(-u.z, 0.3, u.x).normalize();
                    let a = m.vertex(
                        (base - u * 0.06).to_array(),
                        n.to_array(),
                        [0.08, 0.17, 0.035, 1.0],
                        [0.0, 0.0],
                    );
                    let b = m.vertex(
                        (base + u * 0.06).to_array(),
                        n.to_array(),
                        [0.08, 0.17, 0.035, 1.0],
                        [1.0, 0.0],
                    );
                    let c = m.vertex(
                        (base + Vec3::Y * height + u * 0.18).to_array(),
                        n.to_array(),
                        [0.28, 0.4, 0.09, 1.0],
                        [0.5, 1.0],
                    );
                    m.indices.extend_from_slice(&[a, b, c]);
                }
            }
            Species::Shrub => {
                for leaf in 0..14 {
                    let a = i.yaw + leaf as f32 * 2.4;
                    leaf_card(
                        &mut m,
                        p + Vec3::new(a.cos() * 0.5, 0.35 + (leaf % 3) as f32 * 0.2, a.sin() * 0.5)
                            * i.scale,
                        0.35 * i.scale,
                        a,
                    );
                }
            }
            _ => {}
        }
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_crown_morph_endpoint_equals_its_species_specific_simplified_mesh() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<Assets<SurfaceMaterial>>()
            .add_systems(Startup, setup);
        app.update();
        let nature = app.world().resource::<Nature>();
        let meshes = app.world().resource::<Assets<Mesh>>();
        for species in [Species::Broadleaf, Species::Pine, Species::Palm] {
            for variant in 0..4 {
                let fine = meshes.get(nature.crown(species, variant, false)).unwrap();
                let coarse = meshes.get(nature.crown(species, variant, true)).unwrap();
                let Some(VertexAttributeValues::Float32x4(targets)) =
                    fine.attribute(crate::surface::MORPH)
                else {
                    panic!("Missing crown morph");
                };
                let Some(VertexAttributeValues::Float32x3(positions)) =
                    coarse.attribute(Mesh::ATTRIBUTE_POSITION)
                else {
                    panic!("Missing coarse positions");
                };
                assert_eq!(targets.len(), fine.count_vertices());
                let Some(VertexAttributeValues::Float32x3(base)) =
                    fine.attribute(Mesh::ATTRIBUTE_POSITION)
                else {
                    panic!("Missing fine positions");
                };
                let lo = base
                    .iter()
                    .map(|p| Vec3::from_array(*p))
                    .fold(Vec3::splat(f32::INFINITY), Vec3::min);
                let hi = base
                    .iter()
                    .map(|p| Vec3::from_array(*p))
                    .fold(Vec3::splat(f32::NEG_INFINITY), Vec3::max);
                for target in targets {
                    let p = Vec3::new(target[0], target[1], target[2]);
                    assert!(
                        p.cmpge(lo).all() && p.cmple(hi).all(),
                        "Morph exceeded frustum bounds"
                    );
                }
                let fine_indices: Vec<_> = fine.indices().unwrap().iter().collect();
                let coarse_indices: Vec<_> = coarse.indices().unwrap().iter().collect();
                assert!(coarse_indices.len() < fine_indices.len());
                let end_triangles: Vec<_> = fine_indices
                    .chunks_exact(3)
                    .filter_map(|t| {
                        let vertices: [Vec3; 3] = std::array::from_fn(|j| {
                            Vec3::new(targets[t[j]][0], targets[t[j]][1], targets[t[j]][2])
                        });
                        if (vertices[1] - vertices[0])
                            .cross(vertices[2] - vertices[0])
                            .length_squared()
                            < 1e-12
                        {
                            return None;
                        }
                        Some(vertices.map(|p| p.to_array().map(f32::to_bits)))
                    })
                    .collect();
                let expected: Vec<_> = coarse_indices
                    .chunks_exact(3)
                    .map(|t| std::array::from_fn::<_, 3, _>(|j| positions[t[j]].map(f32::to_bits)))
                    .collect();
                assert_eq!(
                    end_triangles, expected,
                    "Species {species:?}, variant {variant} changes shape at the topology switch"
                );
            }
        }
    }
    #[test]
    fn anchored_ground_preserves_geometry_on_high_negative_coordinate_tiles() {
        let origin = [-1024.0, -2048.0];
        let instances = [
            Instance {
                position: [-1018.0, 2800.0, -2040.0],
                species: Species::Grass,
                scale: 0.8,
                yaw: 0.4,
                variant: 0,
            },
            Instance {
                position: [-1010.0, 2802.0, -2032.0],
                species: Species::Shrub,
                scale: 1.2,
                yaw: 1.5,
                variant: 0,
            },
        ];
        let original = ground_mesh(&instances, origin);
        let (anchor, local) = ground_batch(&instances, origin);
        assert!((anchor.y - 2801.0).abs() < 0.001);
        assert_eq!(original.indices, local.indices);
        assert_eq!(original.normals, local.normals);
        assert_eq!(original.colors, local.colors);
        for (before, after) in original.positions.iter().zip(&local.positions) {
            assert!(
                (Vec3::from_array(*before) - Vec3::from_array(*after) - anchor).length() < 0.001
            );
        }
    }
}
