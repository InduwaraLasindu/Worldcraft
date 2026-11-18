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
    pub birch_trunks: Vec<Handle<Mesh>>,
    pub acacia_trunks: Vec<Handle<Mesh>>,
    pub crowns: Vec<Handle<Mesh>>,
    pub pines: Vec<Handle<Mesh>>,
    pub palms: Vec<Handle<Mesh>>,
    pub birches: Vec<Handle<Mesh>>,
    pub acacias: Vec<Handle<Mesh>>,
    pub rocks: Vec<Handle<Mesh>>,
    pub coarse_crowns: Vec<Handle<Mesh>>,
    pub coarse_pines: Vec<Handle<Mesh>>,
    pub coarse_palms: Vec<Handle<Mesh>>,
    pub coarse_birches: Vec<Handle<Mesh>>,
    pub coarse_acacias: Vec<Handle<Mesh>>,
    pub bark: Handle<SurfaceMaterial>,
    pub birch_bark: Handle<SurfaceMaterial>,
    pub leaf: Handle<SurfaceMaterial>,
    pub needles: Handle<SurfaceMaterial>,
    pub fronds: Handle<SurfaceMaterial>,
    pub stone: Handle<SurfaceMaterial>,
    pub ground: Handle<SurfaceMaterial>,
    pub terrain: Handle<SurfaceMaterial>,
    pub water: Handle<SurfaceMaterial>,
}
impl Nature {
    pub fn trunk(&self, species: Species, variant: usize) -> &Handle<Mesh> {
        &(match species {
            Species::Pine => &self.pine_trunks,
            Species::Palm => &self.palm_trunks,
            Species::Birch => &self.birch_trunks,
            Species::Acacia => &self.acacia_trunks,
            _ => &self.trunks,
        })[variant]
    }
    pub fn bark_for(&self, species: Species) -> &Handle<SurfaceMaterial> {
        if species == Species::Birch {
            &self.birch_bark
        } else {
            &self.bark
        }
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
            (Species::Birch, false) => &self.birches,
            (Species::Birch, true) => &self.coarse_birches,
            (Species::Acacia, false) => &self.acacias,
            (Species::Acacia, true) => &self.coarse_acacias,
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
    mut surfaces: ResMut<Assets<SurfaceMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut buffers: ResMut<Assets<bevy::render::storage::ShaderBuffer>>,
) {
    let state = crate::surface::SurfaceFrame::new(&mut buffers);
    let frame = state.handle.clone();
    commands.insert_resource(state);
    let mut trunks = Vec::new();
    let mut pine_trunks = Vec::new();
    let mut palm_trunks = Vec::new();
    let mut birch_trunks = Vec::new();
    let mut acacia_trunks = Vec::new();
    let mut crowns = Vec::new();
    let mut pines = Vec::new();
    let mut palms = Vec::new();
    let mut birches = Vec::new();
    let mut acacias = Vec::new();
    let mut rocks = Vec::new();
    let mut coarse_crowns = Vec::new();
    let mut coarse_pines = Vec::new();
    let mut coarse_palms = Vec::new();
    let mut coarse_birches = Vec::new();
    let mut coarse_acacias = Vec::new();
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
            (
                Species::Birch,
                &mut birch_trunks,
                &mut birches,
                &mut coarse_birches,
            ),
            (
                Species::Acacia,
                &mut acacia_trunks,
                &mut acacias,
                &mut coarse_acacias,
            ),
        ] {
            let prototype = crate::tree::build(species, variant);
            let mut wood = mesh(prototype.wood);
            wood.generate_tangents()
                .expect("Tree UVs support bark normal mapping");
            wood.insert_attribute(crate::surface::OUTER_CENTER, prototype.wood_centers);
            woods.push(meshes.add(wood));
            fine.push(meshes.add(mesh(prototype.crown)));
            let mut outer = mesh(prototype.coarse);
            outer.insert_attribute(crate::surface::OUTER_CENTER, prototype.coarse_centers);
            coarse.push(meshes.add(outer));
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
    let (terrain_color, terrain_normal) = crate::terrain_texture::load();
    let terrain_color = images.add(terrain_color);
    let terrain_normal = images.add(terrain_normal);
    let (bark_color, bark_normal) = crate::nature_texture::bark();
    let bark_color = images.add(bark_color);
    let bark_normal = images.add(bark_normal);
    let (birch_color, birch_normal) = crate::nature_texture::birch_bark();
    let birch_color = images.add(birch_color);
    let birch_normal = images.add(birch_normal);
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
                frame: frame.clone(),
                terrain_color: terrain_color.clone(),
                terrain_normal: terrain_normal.clone(),
            },
        })
    };
    let leaf = foliage(Species::Broadleaf);
    let needles = foliage(Species::Pine);
    let fronds = foliage(Species::Palm);
    let mut wood_material = |color, normal, roughness| {
        surfaces.add(SurfaceMaterial {
            base: StandardMaterial {
                base_color_texture: Some(color),
                normal_map_texture: Some(normal),
                perceptual_roughness: roughness,
                ..default()
            },
            extension: SurfaceDetail {
                parameters: Vec4::new(5.0, 0.0, 0.0, 0.0),
                frame: frame.clone(),
                terrain_color: terrain_color.clone(),
                terrain_normal: terrain_normal.clone(),
            },
        })
    };
    let bark = wood_material(bark_color, bark_normal, 0.94);
    let birch_bark = wood_material(birch_color, birch_normal, 0.88);
    commands.insert_resource(Nature {
        trunks,
        pine_trunks,
        palm_trunks,
        birch_trunks,
        acacia_trunks,
        crowns,
        pines,
        palms,
        birches,
        acacias,
        rocks,
        coarse_crowns,
        coarse_pines,
        coarse_palms,
        coarse_birches,
        coarse_acacias,
        bark,
        birch_bark,
        leaf,
        needles,
        fronds,
        stone: surfaces.add(SurfaceMaterial {
            base: StandardMaterial {
                base_color: Color::srgb(0.35, 0.33, 0.28),
                perceptual_roughness: 0.94,
                ..default()
            },
            extension: SurfaceDetail {
                parameters: Vec4::new(4.0, 0.0, 1.0, 0.0),
                frame: frame.clone(),
                terrain_color: terrain_color.clone(),
                terrain_normal: terrain_normal.clone(),
            },
        }),
        ground: surfaces.add(SurfaceMaterial {
            base: StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(images.add(crate::nature_texture::grass())),
                alpha_mode: AlphaMode::Mask(0.5),
                perceptual_roughness: 0.92,
                cull_mode: None,
                double_sided: true,
                diffuse_transmission: 0.15,
                ..default()
            },
            extension: SurfaceDetail {
                parameters: Vec4::new(3.0, 0.0, 1.0, 0.0),
                frame: frame.clone(),
                terrain_color: terrain_color.clone(),
                terrain_normal: terrain_normal.clone(),
            },
        }),
        terrain: surfaces.add(SurfaceMaterial {
            base: StandardMaterial {
                base_color: Color::WHITE,
                perceptual_roughness: 0.92,
                ..default()
            },
            extension: SurfaceDetail {
                parameters: Vec4::new(0.0, 0.0, 1.0, 0.0),
                frame: frame.clone(),
                terrain_color: terrain_color.clone(),
                terrain_normal: terrain_normal.clone(),
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
                frame: frame.clone(),
                terrain_color: terrain_color.clone(),
                terrain_normal: terrain_normal.clone(),
            },
        }),
    });
}
fn leaf_card(m: &mut MeshData, p: Vec3, size: f32, angle: f32) {
    let u = Vec3::new(angle.cos(), 0.25, angle.sin()) * size;
    let v = Vec3::new(-angle.sin() * 0.5, 0.5, angle.cos() * 0.5) * size;
    let n = -u.cross(v).normalize();
    let a = m.vertex(
        (p - u).to_array(),
        n.to_array(),
        [0.045, 0.12, 0.02, 1.0],
        [0.0, 0.5],
    );
    let b = m.vertex(
        (p + v).to_array(),
        n.to_array(),
        [0.075, 0.19, 0.035, 1.0],
        [0.5, 0.0],
    );
    let c = m.vertex(
        (p + u).to_array(),
        n.to_array(),
        [0.06, 0.16, 0.025, 1.0],
        [1.0, 0.5],
    );
    let d = m.vertex(
        (p - v).to_array(),
        n.to_array(),
        [0.06, 0.16, 0.025, 1.0],
        [0.5, 1.0],
    );
    m.indices.extend_from_slice(&[a, b, c, a, c, d]);
}
pub fn ground_batch(
    instances: &[Instance],
    origin: [f64; 2],
    surface: Option<&mut crate::ground_surface::GroundSurface>,
) -> (Vec3, MeshData) {
    let mut mesh = build_ground_mesh(instances, origin, surface);
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

#[cfg(test)]
pub fn ground_mesh(instances: &[Instance], origin: [f64; 2]) -> MeshData {
    build_ground_mesh(instances, origin, None)
}
struct GrassCard {
    offset: Vec2,
    heading: Vec2,
    height: f32,
    width: f32,
}

// Rotate the shared disk and blade headings per patch; all twenty stratified
// samples are independent of terrain. Avoid forty trigonometric pairs per patch.
static GRASS_CARDS: std::sync::LazyLock<[GrassCard; 20]> = std::sync::LazyLock::new(|| {
    std::array::from_fn(|blade| {
        let (az, ax) = (blade as f32 * 2.39996).sin_cos();
        let (dz, dx) = (blade as f32 * 1.913).sin_cos();
        GrassCard {
            offset: Vec2::new(ax, az) * (((blade * 13 % 20) as f32 + 0.5) / 20.0).sqrt(),
            heading: Vec2::new(dx, dz),
            height: 0.70 + (blade % 7) as f32 * 0.078,
            width: 0.27 + (blade % 3) as f32 * 0.05,
        }
    })
});

fn build_ground_mesh(
    instances: &[Instance],
    origin: [f64; 2],
    mut surface: Option<&mut crate::ground_surface::GroundSurface>,
) -> MeshData {
    let mut m = MeshData::default();
    // Even the smallest grass prototype contributes eighty vertices. Reserve
    // its exact size up front in the common all-grass case.
    let vertices = instances.len() * 80 + 2;
    m.positions.reserve(vertices);
    m.normals.reserve(vertices);
    m.uv.reserve(vertices);
    m.colors.reserve(vertices);
    m.indices.reserve(instances.len() * 120);
    for i in instances {
        let first = m.positions.len();
        let p = Vec3::new(
            (i.position[0] - origin[0]) as f32,
            i.position[1] as f32,
            (i.position[2] - origin[1]) as f32,
        );
        let (height, normal) = match (i.species, surface.as_deref_mut()) {
            (Species::Grass, _) | (_, None) => (p.y, Vec3::Y),
            (_, Some(grid)) => grid.plane(p.x, p.z),
        };
        let p = Vec3::new(p.x, height, p.z);
        match i.species {
            Species::Grass => {
                let (low, high) = match i.variant & 3 {
                    1 => ([0.16, 0.14, 0.035], [0.36, 0.31, 0.09]),
                    2 => ([0.045, 0.085, 0.04], [0.11, 0.17, 0.075]),
                    3 => ([0.025, 0.075, 0.023], [0.075, 0.19, 0.055]),
                    _ => ([0.035, 0.105, 0.012], [0.16, 0.32, 0.045]),
                };
                let color = |t: f32| {
                    [
                        low[0] + (high[0] - low[0]) * t,
                        low[1] + (high[1] - low[1]) * t,
                        low[2] + (high[2] - low[2]) * t,
                        1.0,
                    ]
                };
                // Each admitted patch spends its twenty cards on scattered, short tufts.
                // Stratified radii cover the disk by area and avoid a ring around a bare root.
                let (spread, blade_height) = match i.variant & 3 {
                    1 => (1.58, 0.52), // open, dry grass
                    2 => (1.30, 0.22), // short conifer/tundra understory
                    3 => (1.45, 0.38), // wetland
                    _ => (1.62, 0.31),
                };
                let disk_rotation = Vec2::from_angle(i.yaw);
                let blade_rotation = Vec2::from_angle(i.yaw * 0.57);
                for card in GRASS_CARDS.iter() {
                    let offset = disk_rotation.rotate(card.offset) * (spread * i.scale);
                    let base = p + Vec3::new(offset.x, 0.0, offset.y);
                    let dir = blade_rotation.rotate(card.heading);
                    let direction = Vec3::new(dir.x, 0.0, dir.y);
                    let side = Vec3::new(-dir.y, 0.0, dir.x);
                    let height = i.scale * blade_height * card.height;
                    let width = i.scale * card.width;
                    let mut left = base - side * width;
                    let mut right = base + side * width;
                    if let Some(grid) = surface.as_deref_mut() {
                        left = grid.seat(left);
                        right = grid.seat(right);
                    }
                    let rise = Vec3::Y * height + direction * (height * 0.34);
                    let n = (right - left).cross(rise).normalize();
                    let a = m.vertex(left.to_array(), n.to_array(), color(0.0), [0.0, 0.0]);
                    let b = m.vertex(right.to_array(), n.to_array(), color(0.0), [0.75, 0.0]);
                    let c = m.vertex(
                        (left + rise).to_array(),
                        n.to_array(),
                        color(1.0),
                        [0.0, 1.0],
                    );
                    let d = m.vertex(
                        (right + rise).to_array(),
                        n.to_array(),
                        color(1.0),
                        [0.75, 1.0],
                    );
                    m.indices.extend_from_slice(&[a, b, c, b, d, c]);
                }
            }
            Species::Shrub => {
                for branch in 0..5 {
                    let a = i.yaw + branch as f32 * 2.39996;
                    let dir = Vec3::new(a.cos(), 0.0, a.sin());
                    let tip =
                        p + (dir * 0.35 + Vec3::Y * (0.45 + (branch % 3) as f32 * 0.12)) * i.scale;
                    let first_stem = m.colors.len();
                    crate::tree::tube(&mut m, p, tip, 0.012 * i.scale, 0.004 * i.scale, 3);
                    for color in &mut m.colors[first_stem..] {
                        *color = [0.065, 0.095, 0.025, 1.0];
                    }
                    for row in 0..3 {
                        let center = p.lerp(tip, 0.35 + row as f32 * 0.24);
                        for sign in [-1.0, 1.0] {
                            let angle = a + sign * 1.0;
                            leaf_card(
                                &mut m,
                                center + Vec3::new(angle.cos(), 0.1, angle.sin()) * 0.07 * i.scale,
                                (0.12 - row as f32 * 0.015) * i.scale,
                                angle,
                            );
                        }
                    }
                }
                for at in first..m.uv.len() {
                    m.uv[at][1] = ((m.positions[at][1] - p.y) / i.scale).clamp(0.0, 1.0);
                }
            }
            Species::Fern | Species::Flower | Species::Reed => {
                crate::understory::append(&mut m, i, p, normal);
            }
            _ => {}
        }
        if i.species != Species::Grass {
            // Sample the atlas's opaque white lane. Vertex pigment and the
            // root-relative wind coordinate remain independent of the texture.
            for uv in &mut m.uv[first..] {
                uv[0] = 0.97;
            }
        }
    }
    if !m.positions.is_empty() {
        let (lo, hi) = m.positions.iter().map(|p| Vec3::from_array(*p)).fold(
            (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
            |(lo, hi), p| (lo.min(p), hi.max(p)),
        );
        for p in [lo - Vec3::splat(0.3), hi + Vec3::splat(0.3)] {
            m.vertex(p.to_array(), Vec3::Y.to_array(), [1.0; 4], [0.0; 2]);
        }
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ground_lighting_normals_face_their_geometry_including_alpha_grass() {
        for species in [
            Species::Grass,
            Species::Shrub,
            Species::Fern,
            Species::Flower,
            Species::Reed,
        ] {
            let m = ground_mesh(
                &[Instance {
                    position: [0.0; 3],
                    species,
                    scale: 1.0,
                    yaw: 0.7,
                    variant: 0,
                }],
                [0.0; 2],
            );
            for triangle in m.indices.chunks_exact(3) {
                let p: [Vec3; 3] =
                    std::array::from_fn(|j| Vec3::from_array(m.positions[triangle[j] as usize]));
                let face = (p[1] - p[0]).cross(p[2] - p[0]);
                if face.length_squared() < 1e-12 {
                    continue;
                }
                let normal: Vec3 = triangle
                    .iter()
                    .map(|&at| Vec3::from_array(m.normals[at as usize]))
                    .sum();
                assert!(
                    face.dot(normal) > 0.0,
                    "{species:?} has reversed lighting normals"
                );
            }
            if species != Species::Grass {
                assert!(m.uv[..m.uv.len() - 2].iter().all(|uv| uv[0] == 0.97));
            }
        }
    }
    #[test]
    fn every_crown_morph_endpoint_equals_its_species_specific_simplified_mesh() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<Assets<Image>>()
            .init_resource::<Assets<bevy::render::storage::ShaderBuffer>>()
            .init_resource::<Assets<SurfaceMaterial>>()
            .add_systems(Startup, setup);
        app.update();
        let nature = app.world().resource::<Nature>();
        let meshes = app.world().resource::<Assets<Mesh>>();
        for species in Species::TREES {
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
    fn expanded_grass_roots_follow_slopes_without_unbounded_geometry() {
        let g = worldcraft_core::Generator::new(
            worldcraft_core::GeneratorSettings::for_preset(
                worldcraft_core::Preset::Alpine,
                20261116,
            ),
            vec![],
        );
        let origin = [-384.0, -256.0];
        let x = origin[0] + 32.0;
        let z = origin[1] + 48.0;
        let instance = Instance {
            position: [x, g.height(x, z), z],
            species: Species::Grass,
            scale: 1.2,
            yaw: 0.7,
            variant: 0,
        };
        let mut grid = crate::ground_surface::GroundSurface::new(&g, origin, None);
        let (anchor, m) = ground_batch(&[instance], origin, Some(&mut grid));
        assert!(m.indices.len() / 3 <= 40);
        assert!(m.indices.iter().all(|&i| (i as usize) < m.positions.len()));
        for blade in 0..20 {
            for edge in [0, 1] {
                let root = Vec3::from_array(m.positions[blade * 4 + edge]) + anchor;
                let terrain = crate::control::ground(
                    &g,
                    origin[0] + root.x as f64,
                    origin[1] + root.z as f64,
                );
                assert!(
                    (root.y as f64 - terrain).abs() < 0.001,
                    "Visible grass foot missed the rendered triangle"
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
        let (anchor, local) = ground_batch(&instances, origin, None);
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
