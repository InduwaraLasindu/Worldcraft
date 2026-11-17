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
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct SurfaceDetail {
    /// kind (0 terrain, 1 water, 2 crowns), time, detail enabled, reserved
    #[uniform(100)]
    pub parameters: Vec4,
    #[uniform(101)]
    pub origin: Vec4,
    #[uniform(102)]
    pub observer: Vec4,
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
        _key: bevy::pbr::MaterialExtensionKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
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
    ] {
        if let Some(mut m) = materials.get_mut(handle) {
            m.extension.origin = origin;
            m.extension.observer = (player.position - session.origin).as_vec3().extend(0.0);
            if m.extension.parameters.x == 2.0 {
                m.extension.observer.w =
                    crate::foliage::Foliage::crown_end(limits.radius(graphics.radius));
            }
            m.extension.parameters.y = time.elapsed_secs();
            m.extension.parameters.z = if limits.quality(graphics.quality) == Quality::Low {
                0.0
            } else {
                1.0
            };
        }
    }
}
