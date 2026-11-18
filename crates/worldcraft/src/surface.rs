//! Embedded, world-anchored procedural surface detail; no runtime texture downloads.
use crate::session::{Graphics, Player, Quality, Session};
use bevy::{
    asset::{load_internal_asset, uuid_handle, RenderAssetUsages},
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::{render_resource::AsBindGroup, storage::ShaderBuffer},
    shader::{Shader, ShaderRef},
};
pub const OUTER_CENTER: bevy::mesh::MeshVertexAttribute = bevy::mesh::MeshVertexAttribute::new(
    "BotanicalCenter",
    0x7a1004,
    bevy::render::render_resource::VertexFormat::Float32x3,
);
pub const SNOW_COVER: bevy::mesh::MeshVertexAttribute = bevy::mesh::MeshVertexAttribute::new(
    "WorldSnowCover",
    0x7a1005,
    bevy::render::render_resource::VertexFormat::Unorm8x4,
);
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

/// One fixed 80-byte GPU allocation shared by every surface and shader pass.
/// The extra two vectors retain the previous observer/wind state so temporal
/// reconstruction sees actual leaf and crown motion rather than static proxies.
/// Updating ShaderBuffer data at the same size reuses its GPU buffer and bind
/// groups, instead of preparing all materials each animation frame.
#[derive(Resource)]
pub struct SurfaceFrame {
    pub handle: Handle<ShaderBuffer>,
    values: [Vec4; 5],
}
impl SurfaceFrame {
    pub fn new(buffers: &mut Assets<ShaderBuffer>) -> Self {
        Self {
            handle: buffers.add(ShaderBuffer::new(
                vec![[0.0f32; 4]; 5],
                RenderAssetUsages::default(),
            )),
            values: [Vec4::ZERO; 5],
        }
    }
}
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
    /// Immutable kind (0 terrain, 1 water, 2 crowns, 3 ground, 4 rocks, 5 wood).
    #[uniform(100)]
    pub parameters: Vec4,
    #[storage(103, read_only)]
    pub frame: Handle<ShaderBuffer>,
    #[texture(110, dimension = "2d_array")]
    #[sampler(111)]
    pub terrain_color: Handle<Image>,
    #[texture(112, dimension = "2d_array")]
    pub terrain_normal: Handle<Image>,
    #[texture(113, dimension = "2d_array")]
    pub terrain_height: Handle<Image>,
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
            5 => "SURFACE_WOOD",
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
            (Mesh::ATTRIBUTE_TANGENT, 4),
        ] {
            if layout.0.contains(attribute.id) {
                attributes.push(attribute.at_shader_location(location));
            }
        }
        if layout.0.contains(OUTER_CENTER.id) {
            descriptor
                .vertex
                .shader_defs
                .push("BOTANICAL_CENTER".into());
            attributes.push(OUTER_CENTER.at_shader_location(11));
        }
        if layout.0.contains(SNOW_COVER.id) {
            attributes.push(SNOW_COVER.at_shader_location(12));
            descriptor.vertex.shader_defs.push("SNOW_CLIMATE".into());
            if let Some(fragment) = &mut descriptor.fragment {
                fragment.shader_defs.push("SNOW_CLIMATE".into());
            }
        }
        if layout.0.contains(MORPH.id) {
            if kind == "SURFACE_TERRAIN" {
                if let Some(fragment) = &mut descriptor.fragment {
                    fragment.shader_defs.push("HABITAT_TERRAIN".into());
                }
            }
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
    rays: Res<crate::raytrace::RayState>,
    mut frame: ResMut<SurfaceFrame>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    if limits.suspended {
        return;
    }
    // All noise periods divide 4096 m. Wrapping the f64 origin keeps detail
    // stable under origin rebases without uploading imprecise large f32 positions.
    let radius = limits.radius(graphics.radius);
    let outer_end = if session.generator.is_some() {
        crate::foliage::Foliage::outer_end(radius)
    } else {
        0.0
    };
    // xz are wrapped world coordinates, yw carry the previous/current end
    // distance for the temporal and primary vegetation passes.
    let origin = Vec4::new(
        session.origin.x.rem_euclid(4096.0) as f32,
        frame.values[0].w,
        session.origin.z.rem_euclid(4096.0) as f32,
        outer_end,
    );
    let observer =
        (player.position - session.origin)
            .as_vec3()
            .extend(crate::foliage::Foliage::crown_end(
                limits.radius(graphics.radius),
            ));
    // Secondary ray proxies are static, so pause primary foliage in ray mode.
    let breeze = if rays.active {
        0.0
    } else {
        graphics.breeze.clamp(0.0, 1.0)
    };
    let detail = if limits.quality(graphics.quality) == Quality::Low {
        0.0
    } else {
        limits.preferences.render.surface_relief.layers().max(1) as f32
    };
    let seconds = if breeze > 0.0 || detail > 0.0 {
        time.elapsed_secs()
    } else {
        0.0
    };
    // Eco hides its finite horizontal boundary, even from an elevated camera.
    // Underwater fog must retain its ordinary three-dimensional absorption.
    let horizon = if limits.mode() == crate::performance::Mode::Eco
        && !session
            .world
            .as_ref()
            .is_some_and(|w| player.position.y < w.settings.sea_level)
    {
        1.0
    } else {
        0.0
    };
    let next = [
        origin,
        observer,
        Vec4::new(seconds, breeze, detail, horizon),
        frame.values[1],
        Vec4::new(
            frame.values[2].x,
            frame.values[2].y,
            frame.values[0].x,
            frame.values[0].z,
        ),
    ];
    if frame.values != next {
        let mut buffer = buffers
            .get_mut(&frame.handle)
            .expect("Shared surface frame");
        buffer.clear();
        buffer.extend_from_slice(&next.map(|v| v.to_array()));
        frame.values = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::asset::{AssetEvent, AssetPlugin};

    #[test]
    fn animation_and_movement_reuse_one_frame_without_modifying_materials() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<SurfaceMaterial>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<Image>>()
            .init_resource::<Assets<ShaderBuffer>>()
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
        app.world_mut().resource_mut::<Graphics>().breeze = 0.45;
        for _ in 0..3 {
            app.update();
        }
        let leaf = app.world().resource::<crate::assets::Nature>().leaf.id();
        let shared = app.world().resource::<SurfaceFrame>().handle.id();
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
                assert!(
                    !matches!(event, AssetEvent::Modified { .. }),
                    "Animation re-prepared a material"
                );
            }
        }
        app.world_mut().resource_mut::<Player>().position.x += 12.0;
        app.update();
        let changed: Vec<_> = app
            .world_mut()
            .resource_mut::<Messages<AssetEvent<SurfaceMaterial>>>()
            .drain()
            .collect();
        assert!(!changed
            .iter()
            .any(|event| matches!(event, AssetEvent::Modified { .. })));
        let buffers = app.world().resource::<Assets<ShaderBuffer>>();
        assert_eq!(buffers.len(), 1);
        let buffer = buffers.get(shared).unwrap();
        assert_eq!(buffer.buffer_size(), 80);
        assert_eq!(
            buffer.cast_slice::<[f32; 4]>().unwrap()[1][0],
            app.world().resource::<Player>().position.x as f32
        );
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
        // Height changes and mode switches update only the shared frame. The
        // horizon mask must not turn underwater absorption into horizontal fog.
        app.insert_resource(crate::performance::Performance::load(false));
        app.world_mut().resource_mut::<Session>().world = Some(worldcraft_core::WorldFile::new(
            "Fog test".into(),
            worldcraft_core::GeneratorSettings::default(),
        ));
        for (mode, height, expected) in [
            (crate::performance::Mode::Eco, 1000.0, 1.0),
            (crate::performance::Mode::Eco, -1000.0, 0.0),
            (crate::performance::Mode::Performance, 1000.0, 0.0),
        ] {
            let mut limits = app
                .world_mut()
                .resource_mut::<crate::performance::Performance>();
            limits.preferences.mode = mode;
            limits.calibrating = false;
            app.world_mut().resource_mut::<Player>().position.y = height;
            app.update();
            let buffers = app.world().resource::<Assets<ShaderBuffer>>();
            let buffer = buffers.get(shared).unwrap();
            assert_eq!(buffer.buffer_size(), 80);
            assert_eq!(buffer.cast_slice::<[f32; 4]>().unwrap()[2][3], expected);
            assert!(!app
                .world_mut()
                .resource_mut::<Messages<AssetEvent<SurfaceMaterial>>>()
                .drain()
                .any(|event| matches!(event, AssetEvent::Modified { .. })));
        }
    }
}
