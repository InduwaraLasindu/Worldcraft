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
    pub pine_trunks: Vec<Handle<Mesh>>,
    pub palm_trunks: Vec<Handle<Mesh>>,
    pub crowns: Vec<Handle<Mesh>>,
    pub pines: Vec<Handle<Mesh>>,
    pub palms: Vec<Handle<Mesh>>,
    pub rocks: Vec<Handle<Mesh>>,
    pub coarse_crowns: Vec<Handle<Mesh>>,
    pub coarse_pines: Vec<Handle<Mesh>>,
    pub coarse_palms: Vec<Handle<Mesh>>,
    pub bark: Handle<StandardMaterial>,
    pub leaf: Handle<SurfaceMaterial>,
    pub needles: Handle<SurfaceMaterial>,
    pub fronds: Handle<SurfaceMaterial>,
    pub stone: Handle<StandardMaterial>,
    pub ground: Handle<StandardMaterial>,
    pub terrain: Handle<SurfaceMaterial>,
    pub water: Handle<SurfaceMaterial>,
}
impl Nature {
    pub fn trunk(&self, species: Species, variant: usize) -> &Handle<Mesh> {
        &(match species {
            Species::Pine => &self.pine_trunks,
            Species::Palm => &self.palm_trunks,
            _ => &self.trunks,
        })[variant]
    }
    pub fn foliage(&self, species: Species) -> &Handle<SurfaceMaterial> {
        match species {
            Species::Pine => &self.needles,
            Species::Palm => &self.fronds,
            _ => &self.leaf,
        }
    }

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
    mut images: ResMut<Assets<Image>>,
) {
    let mut trunks = Vec::new();
    let mut pine_trunks = Vec::new();
    let mut palm_trunks = Vec::new();
    let mut crowns = Vec::new();
    let mut pines = Vec::new();
    let mut palms = Vec::new();
    let mut rocks = Vec::new();
    let mut coarse_crowns = Vec::new();
    let mut coarse_pines = Vec::new();
    let mut coarse_palms = Vec::new();
    for variant in 0..4 {
        let seed = 999 + variant as u64;
        for (species, woods, fine, coarse) in [
            (
                Species::Broadleaf,
                &mut trunks,
                &mut crowns,
                &mut coarse_crowns,
            ),
            (
                Species::Pine,
                &mut pine_trunks,
                &mut pines,
                &mut coarse_pines,
            ),
            (
                Species::Palm,
                &mut palm_trunks,
                &mut palms,
                &mut coarse_palms,
            ),
        ] {
            let prototype = crate::tree::build(species, variant);
            let mut wood = mesh(prototype.wood);
            wood.generate_tangents()
                .expect("Tree UVs support bark normal mapping");
            woods.push(meshes.add(wood));
            fine.push(meshes.add(mesh(prototype.crown)));
            coarse.push(meshes.add(mesh(prototype.coarse)));
        }
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
    let (bark_color, bark_normal) = crate::nature_texture::bark();
    let bark_color = images.add(bark_color);
    let bark_normal = images.add(bark_normal);
    let mut foliage = |species| {
        surfaces.add(SurfaceMaterial {
            base: StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(images.add(crate::nature_texture::foliage(species))),
                alpha_mode: AlphaMode::Mask(0.5),
                perceptual_roughness: 0.82,
                cull_mode: None,
                double_sided: true,
                diffuse_transmission: 0.35,
                ..default()
            },
            extension: SurfaceDetail {
                parameters: Vec4::new(2.0, 0.0, 0.0, 0.0),
                origin: Vec4::ZERO,
                observer: Vec4::new(0.0, 0.0, 0.0, 128.0),
            },
        })
    };
    let leaf = foliage(Species::Broadleaf);
    let needles = foliage(Species::Pine);
    let fronds = foliage(Species::Palm);
    commands.insert_resource(Nature {
        trunks,
        pine_trunks,
        palm_trunks,
        crowns,
        pines,
        palms,
        rocks,
        coarse_crowns,
        coarse_pines,
        coarse_palms,
        bark: materials.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(bark_color),
            normal_map_texture: Some(bark_normal),
            perceptual_roughness: 0.94,
            ..default()
        }),
        leaf,
        needles,
        fronds,
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
            .init_resource::<Assets<Image>>()
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
