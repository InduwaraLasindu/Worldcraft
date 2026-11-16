//! Capability-gated native ray queries. Raster remains the default.
//! Secondary rays use static geometry and averaged vertex albedo; the primary
//! raster pass keeps the full procedural terrain shader.
use crate::{
    capabilities::Capabilities,
    scene::MainCamera,
    session::{Graphics, Quality},
    surface::SurfaceMaterial,
};
use bevy::{
    asset::AssetId,
    camera::CameraMainTextureUsages,
    core_pipeline::prepass::{DeferredPrepass, DepthPrepass, MotionVectorPrepass},
    material::OpaqueRendererMethod,
    mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
    prelude::*,
    render::render_resource::TextureUsages,
    solari::prelude::{RaytracingMesh3d, SolariLighting},
};
use std::collections::{HashMap, HashSet};

#[derive(Component)]
pub struct Prepared;
#[derive(Component)]
pub struct Proxy;
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum MaterialKey {
    Standard(AssetId<StandardMaterial>),
    Surface(AssetId<SurfaceMaterial>),
}
type Key = (AssetId<Mesh>, MaterialKey);
#[derive(Clone)]
struct PreparedAssets {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}
#[derive(Resource, Default)]
pub struct RayState {
    pub active: bool,
    pub instances: usize,
    pub pending: usize,
    pub rejected: usize,
    cache: HashMap<Key, PreparedAssets>,
}
pub fn allowed(requested: bool, caps: &Capabilities) -> bool {
    requested && caps.hardware_rays && !caps.software
}
/// Convert only supported immutable source data; never mutate a raster mesh.
fn compatible(source: &Mesh) -> Result<(Mesh, [f32; 4]), &'static str> {
    if source.primitive_topology() != PrimitiveTopology::TriangleList {
        return Err("non-triangle geometry");
    }
    let mut result = Mesh::new(
        PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::default(),
    );
    let count = source.count_vertices();
    if count < 3 {
        return Err("empty geometry");
    }
    for attribute in [
        Mesh::ATTRIBUTE_POSITION,
        Mesh::ATTRIBUTE_NORMAL,
        Mesh::ATTRIBUTE_UV_0,
    ] {
        let value = source
            .attribute(attribute)
            .ok_or("missing vertex attribute")?
            .clone();
        result.insert_attribute(attribute, value);
    }
    let indices = source
        .indices()
        .map(|i| i.iter().map(|i| i as u32).collect())
        .unwrap_or_else(|| (0..count as u32).collect());
    result.insert_indices(Indices::U32(indices));
    if let Some(tangents) = source.attribute(Mesh::ATTRIBUTE_TANGENT) {
        result.insert_attribute(Mesh::ATTRIBUTE_TANGENT, tangents.clone());
    } else if result.generate_tangents().is_err() {
        let Some(VertexAttributeValues::Float32x3(normals)) =
            result.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            return Err("invalid normals");
        };
        let tangents: Vec<[f32; 4]> = normals
            .iter()
            .map(|n| {
                let normal = Vec3::from_array(*n).normalize_or_zero();
                let axis = if normal.x.abs() < 0.8 {
                    Vec3::X
                } else {
                    Vec3::Z
                };
                let t = normal.cross(axis).normalize_or_zero();
                [t.x, t.y, t.z, 1.0]
            })
            .collect();
        result.insert_attribute(Mesh::ATTRIBUTE_TANGENT, tangents);
    }
    let mut mean = [1.0; 4];
    if let Some(VertexAttributeValues::Float32x4(colors)) = source.attribute(Mesh::ATTRIBUTE_COLOR)
    {
        if !colors.is_empty() {
            for channel in 0..4 {
                mean[channel] =
                    colors.iter().map(|c| c[channel]).sum::<f32>() / colors.len() as f32;
            }
        }
    }
    result.enable_raytracing = true;
    Ok((result, mean))
}
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn update(
    mut commands: Commands,
    caps: Res<Capabilities>,
    mut graphics: ResMut<Graphics>,
    mut state: ResMut<RayState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut surfaces: ResMut<Assets<SurfaceMaterial>>,
    sources: Query<(
        Entity,
        &Mesh3d,
        Option<&MeshMaterial3d<StandardMaterial>>,
        Option<&MeshMaterial3d<SurfaceMaterial>>,
        Has<Prepared>,
    )>,
    proxies: Query<Entity, With<Proxy>>,
    cameras: Query<Entity, With<MainCamera>>,
) {
    let active = allowed(graphics.ray_tracing, &caps);
    if graphics.ray_tracing && !active {
        graphics.ray_tracing = false;
    }
    if active != state.active {
        let method = if active {
            OpaqueRendererMethod::Deferred
        } else {
            OpaqueRendererMethod::Forward
        };
        for (_, mut material) in materials.iter_mut() {
            material.opaque_render_method = method;
        }
        for (_, mut material) in surfaces.iter_mut() {
            material.base.opaque_render_method = method;
        }
        for camera in &cameras {
            if active {
                commands.entity(camera).insert((
                    CameraMainTextureUsages::default().with(TextureUsages::STORAGE_BINDING),
                    SolariLighting {
                        primary_di_samples: 4,
                        secondary_di_samples: 2,
                        max_bounces: 2,
                        world_cache_cell_updates_soft_target: 12000,
                        world_cache_direct_light_sample_count: 8,
                        ..default()
                    },
                ));
            } else {
                commands
                    .entity(camera)
                    .remove::<(SolariLighting, DeferredPrepass, MotionVectorPrepass)>();
                if graphics.quality != Quality::High {
                    commands.entity(camera).remove::<DepthPrepass>();
                }
                commands
                    .entity(camera)
                    .insert(CameraMainTextureUsages::default());
            }
        }
        state.active = active;
        info!(
            "RAY: native ray-query lighting {}",
            if active { "enabled" } else { "disabled" }
        );
    }
    if !active {
        for proxy in &proxies {
            commands.entity(proxy).despawn();
        }
        for (entity, _, _, _, prepared) in &sources {
            if prepared {
                commands.entity(entity).remove::<Prepared>();
            }
        }
        state.cache.clear();
        state.instances = 0;
        state.pending = 0;
        state.rejected = 0;
        return;
    }
    let mut live = HashSet::new();
    let mut created = 0;
    let mut converted = 0;
    let mut pending = 0;
    for (entity, mesh, standard, surface, prepared) in &sources {
        let (material_key, base) = if let Some(handle) = standard {
            let Some(base) = materials.get(&handle.0) else {
                continue;
            };
            (MaterialKey::Standard(handle.0.id()), base.clone())
        } else if let Some(handle) = surface {
            let Some(base) = surfaces.get(&handle.0) else {
                continue;
            };
            (MaterialKey::Surface(handle.0.id()), base.base.clone())
        } else {
            continue;
        };
        let key = (mesh.0.id(), material_key);
        live.insert(key);
        if prepared {
            continue;
        }
        if !matches!(base.alpha_mode, AlphaMode::Opaque | AlphaMode::Mask(_)) {
            commands.entity(entity).insert(Prepared);
            continue;
        }
        pending += 1;
        if created >= 16 || (!state.cache.contains_key(&key) && converted >= 2) {
            continue;
        }
        let assets = if let Some(a) = state.cache.get(&key) {
            a.clone()
        } else {
            let Some(source) = meshes.get(&mesh.0) else {
                continue;
            };
            let (mesh, color) = match compatible(source) {
                Ok(result) => result,
                Err(reason) => {
                    warn!("RAY: skipped unsupported mesh: {reason}");
                    commands.entity(entity).insert(Prepared);
                    state.rejected += 1;
                    continue;
                }
            };
            let mut material = base;
            let c = material.base_color.to_linear();
            material.base_color = Color::linear_rgba(
                c.red * color[0],
                c.green * color[1],
                c.blue * color[2],
                c.alpha * color[3],
            );
            let a = PreparedAssets {
                mesh: meshes.add(mesh),
                material: materials.add(material),
            };
            state.cache.insert(key, a.clone());
            converted += 1;
            a
        };
        commands.entity(entity).insert(Prepared).with_children(|p| {
            p.spawn((
                Proxy,
                RaytracingMesh3d(assets.mesh),
                MeshMaterial3d(assets.material),
                Transform::IDENTITY,
            ));
        });
        created += 1;
        pending -= 1;
    }
    state.cache.retain(|key, _| live.contains(key));
    state.instances = proxies.iter().count() + created;
    state.pending = pending;
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unsupported_and_software_adapters_always_keep_raster() {
        let mut c = Capabilities {
            gpu: "test".into(),
            backend: "test".into(),
            ray_api: false,
            hardware_rays: false,
            software: false,
            cpu_threads: 1,
            ray_reason: String::new(),
        };
        assert!(!allowed(true, &c));
        c.ray_api = true;
        c.software = true;
        assert!(!allowed(true, &c));
        c.hardware_rays = true;
        assert!(!allowed(true, &c));
        c.software = false;
        assert!(allowed(true, &c));
        assert!(!allowed(false, &c));
    }
    #[test]
    fn ray_geometry_has_exact_supported_layout_and_preserves_the_source() {
        let original = Cuboid::default().mesh().build();
        let mut original =
            original.with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, vec![[0.2, 0.4, 0.6, 1.0]; 24]);
        original.insert_attribute(Mesh::ATTRIBUTE_UV_1, vec![[0.0, 0.0]; 24]);
        let (converted, color) = compatible(&original).unwrap();
        assert!(original.contains_attribute(Mesh::ATTRIBUTE_COLOR));
        assert!(!converted.contains_attribute(Mesh::ATTRIBUTE_COLOR));
        assert!(!converted.contains_attribute(Mesh::ATTRIBUTE_UV_1));
        assert!(converted.contains_attribute(Mesh::ATTRIBUTE_TANGENT));
        assert!(matches!(converted.indices(), Some(Indices::U32(_))));
        assert!(converted.enable_raytracing);
        assert!((color[1] - 0.4).abs() < 1e-6);
        assert_eq!(
            original.attribute(Mesh::ATTRIBUTE_POSITION),
            converted.attribute(Mesh::ATTRIBUTE_POSITION)
        );
    }
}
