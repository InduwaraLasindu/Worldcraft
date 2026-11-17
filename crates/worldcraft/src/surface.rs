//! Embedded, world-anchored procedural surface detail; no runtime texture downloads.
use crate::{
    assets::Nature,
    session::{Graphics, Player, Quality, Session},
};
use bevy::{
    asset::{load_internal_asset, uuid_handle},
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::AsBindGroup,
    shader::{Shader, ShaderRef},
};
pub const MORPH: bevy::mesh::MeshVertexAttribute = bevy::mesh::MeshVertexAttribute::new(
    "TerrainMorph",
    0x7a1001,
    bevy::render::render_resource::VertexFormat::Float32x4,
);
pub const MORPH_NORMAL: bevy::mesh::MeshVertexAttribute = bevy::mesh::MeshVertexAttribute::new(
    "TerrainMorphNormal",
    0x7a1002,
    bevy::render::render_resource::VertexFormat::Float32x3,
);
pub const MORPH_COLOR: bevy::mesh::MeshVertexAttribute = bevy::mesh::MeshVertexAttribute::new(
    "TerrainMorphColor",
    0x7a1003,
    bevy::render::render_resource::VertexFormat::Float32x4,
);
const VERTEX: Handle<Shader> = uuid_handle!("6384c9de-3a43-42b0-9ac2-d349a8cdb8dd");
const SHADER: Handle<Shader> = uuid_handle!("679ebd4f-5336-4664-92d3-c11b8c994132");
pub type SurfaceMaterial = ExtendedMaterial<StandardMaterial, SurfaceDetail>;
/// Only immutable surface identity belongs in a pipeline key. Animation time,
/// observer movement and quality uniforms must never create fresh pipelines.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SurfaceKey(u8);
impl From<&SurfaceDetail> for SurfaceKey {
    fn from(value: &SurfaceDetail) -> Self {
        Self(value.parameters.x as u8)
    }
}
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
#[bind_group_data(SurfaceKey)]
pub struct SurfaceDetail {
    /// kind (0 terrain, 1 water, 2 crowns, 3 ground, 4 rocks), time, detail enabled, foliage breeze
    #[uniform(100)]
    pub parameters: Vec4,
    #[uniform(101)]
    pub origin: Vec4,
    #[uniform(102)]
    pub observer: Vec4,
    #[texture(110, dimension = "2d_array")]
    #[sampler(111)]
    pub terrain_color: Handle<Image>,
    #[texture(112, dimension = "2d_array")]
    pub terrain_normal: Handle<Image>,
}
impl MaterialExtension for SurfaceDetail {
    fn vertex_shader() -> ShaderRef {
        VERTEX.into()
    }
    fn prepass_vertex_shader() -> ShaderRef {
        VERTEX.into()
    }
    fn deferred_vertex_shader() -> ShaderRef {
        VERTEX.into()
    }
    fn specialize(
        _pipeline: &bevy::pbr::MaterialExtensionPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        layout: &bevy::mesh::MeshVertexBufferLayoutRef,
        key: bevy::pbr::MaterialExtensionKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        // Do not compile triplanar terrain sampling into every leaf, water and
        // ground pipeline. This also bounds DXC's optimization work on Windows.
        let kind = match key.bind_group_data.0 {
            0 => "SURFACE_TERRAIN",
            1 => "SURFACE_WATER",
            2 => "SURFACE_FOLIAGE",
            3 => "SURFACE_GROUND",
            4 => "SURFACE_ROCK",
            _ => unreachable!("Unknown surface kind"),
        };
        descriptor.vertex.shader_defs.push(kind.into());
        if let Some(fragment) = &mut descriptor.fragment {
            fragment.shader_defs.push(kind.into());
        }
        let mut attributes = vec![Mesh::ATTRIBUTE_POSITION.at_shader_location(0)];
        for (attribute, location) in [
            (Mesh::ATTRIBUTE_NORMAL, 1),
            (Mesh::ATTRIBUTE_UV_0, 2),
            (Mesh::ATTRIBUTE_COLOR, 5),
        ] {
            if layout.0.contains(attribute.id) {
                attributes.push(attribute.at_shader_location(location));
            }
        }
        if layout.0.contains(MORPH.id) {
            descriptor.vertex.shader_defs.push("TERRAIN_MORPH".into());
            attributes.extend([
                MORPH.at_shader_location(8),
                MORPH_NORMAL.at_shader_location(9),
                MORPH_COLOR.at_shader_location(10),
            ]);
        }
        descriptor.vertex.buffers = vec![layout.0.get_layout(&attributes)?];
        Ok(())
    }

    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }
    fn deferred_fragment_shader() -> ShaderRef {
        SHADER.into()
    }
}
pub struct SurfacePlugin;
impl Plugin for SurfacePlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, SHADER, "surface.wesl", Shader::from_wesl);
        load_internal_asset!(app, VERTEX, "terrain_vertex.wesl", Shader::from_wesl);
        app.add_plugins(MaterialPlugin::<SurfaceMaterial>::default());
    }
}
pub fn update(
    session: Res<Session>,
    player: Res<Player>,
    graphics: Res<Graphics>,
    time: Res<Time>,
    limits: Res<crate::performance::Performance>,
    nature: Res<Nature>,
    rays: Res<crate::raytrace::RayState>,
    mut materials: ResMut<Assets<SurfaceMaterial>>,
) {
    if limits.suspended {
        return;
    }
    // All noise periods divide 4096 m. Wrapping the f64 origin keeps detail
    // stable under origin rebases without uploading imprecise large f32 positions.
    let origin = Vec4::new(
        session.origin.x.rem_euclid(4096.0) as f32,
        0.0,
        session.origin.z.rem_euclid(4096.0) as f32,
        0.0,
    );
    for handle in [
        &nature.terrain,
        &nature.water,
        &nature.leaf,
        &nature.needles,
        &nature.fronds,
        &nature.ground,
        &nature.stone,
    ] {
        if let Some(m) = materials.get(handle) {
            let mut observer = Vec4::ZERO;
            let kind = m.extension.parameters.x;
            if kind == 0.0 || kind == 2.0 {
                observer = (player.position - session.origin).as_vec3().extend(0.0);
            }
            if kind == 2.0 {
                observer.w = crate::foliage::Foliage::crown_end(limits.radius(graphics.radius));
            }
            // Secondary ray proxies are static: pause movement in experimental
            // ray mode rather than letting animated primary leaves disagree.
            let breeze = if rays.active || (kind != 2.0 && kind != 3.0) {
                0.0
            } else {
                graphics.breeze.clamp(0.0, 1.0)
            };
            let detail = if limits.quality(graphics.quality) == Quality::Low {
                0.0
            } else {
                1.0
            };
            let seconds = if breeze > 0.0 || (kind == 1.0 && detail > 0.0) {
                time.elapsed_secs()
            } else {
                0.0
            };
            let parameters = Vec4::new(kind, seconds, detail, breeze);
            // get_mut emits an asset event even when assigned identical values.
            // Stationary terrain/rocks and paused leaves need no extraction,
            // material preparation or uniform uploads on subsequent frames.
            if m.extension.origin != origin
                || m.extension.observer != observer
                || m.extension.parameters != parameters
            {
                let extension = &mut materials.get_mut(handle).unwrap().extension;
                extension.origin = origin;
                extension.observer = observer;
                extension.parameters = parameters;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::asset::{AssetEvent, AssetPlugin};

    #[test]
    fn stationary_materials_do_not_reprepare_and_movement_keeps_pipeline_identity() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<SurfaceMaterial>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<Image>>()
            .init_resource::<Session>()
            .init_resource::<Player>()
            .init_resource::<Graphics>()
            .init_resource::<crate::raytrace::RayState>()
            .insert_resource(crate::performance::Performance::load(true))
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_millis(16),
            ))
            .add_systems(Startup, crate::assets::setup)
            .add_systems(Update, update);
        app.world_mut().resource_mut::<Graphics>().breeze = 0.0;
        for _ in 0..3 {
            app.update();
        }
        let (water, terrain, leaf) = {
            let nature = app.world().resource::<Nature>();
            (nature.water.id(), nature.terrain.id(), nature.leaf.id())
        };
        let key = SurfaceKey::from(
            &app.world()
                .resource::<Assets<SurfaceMaterial>>()
                .get(leaf)
                .unwrap()
                .extension,
        );
        app.world_mut()
            .resource_mut::<Messages<AssetEvent<SurfaceMaterial>>>()
            .clear();
        for _ in 0..4 {
            app.update();
            for event in app
                .world_mut()
                .resource_mut::<Messages<AssetEvent<SurfaceMaterial>>>()
                .drain()
            {
                if let AssetEvent::Modified { id } = event {
                    assert_eq!(id, water, "A stationary nonanimated surface was reuploaded");
                }
            }
        }
        app.world_mut().resource_mut::<Player>().position.x += 12.0;
        app.update();
        let changed: Vec<_> = app
            .world_mut()
            .resource_mut::<Messages<AssetEvent<SurfaceMaterial>>>()
            .drain()
            .collect();
        assert!(changed
            .iter()
            .any(|event| matches!(event, AssetEvent::Modified { id } if *id == terrain)));
        assert!(changed
            .iter()
            .any(|event| matches!(event, AssetEvent::Modified { id } if *id == leaf)));
        assert_eq!(
            key,
            SurfaceKey::from(
                &app.world()
                    .resource::<Assets<SurfaceMaterial>>()
                    .get(leaf)
                    .unwrap()
                    .extension
            )
        );
    }
}
