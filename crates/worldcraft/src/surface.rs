//! Embedded, world-anchored procedural surface detail; no runtime texture downloads.
use crate::{
    assets::Nature,
    session::{Graphics, Quality, Session},
};
use bevy::{
    asset::{load_internal_asset, uuid_handle},
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::AsBindGroup,
    shader::{Shader, ShaderRef},
};
const SHADER: Handle<Shader> = weak_handle!("679ebd4f-5336-4664-92d3-c11b8c994132");
pub type SurfaceMaterial = ExtendedMaterial<StandardMaterial, SurfaceDetail>;
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct SurfaceDetail {
    /// kind (0 terrain, 1 water), time, detail enabled, reserved
    #[uniform(100)]
    pub parameters: Vec4,
    #[uniform(101)]
    pub origin: Vec4,
}
impl MaterialExtension for SurfaceDetail {
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
        app.add_plugins(MaterialPlugin::<SurfaceMaterial>::default());
    }
}
pub fn update(
    session: Res<Session>,
    graphics: Res<Graphics>,
    time: Res<Time>,
    nature: Res<Nature>,
    mut materials: ResMut<Assets<SurfaceMaterial>>,
) {
    // All noise periods divide 4096 m. Wrapping the f64 origin keeps detail
    // stable under origin rebases without uploading imprecise large f32 positions.
    let origin = Vec4::new(
        session.origin.x.rem_euclid(4096.0) as f32,
        0.0,
        session.origin.z.rem_euclid(4096.0) as f32,
        0.0,
    );
    for handle in [&nature.terrain, &nature.water] {
        if let Some(m) = materials.get_mut(handle) {
            m.extension.origin = origin;
            m.extension.parameters.y = time.elapsed_secs();
            m.extension.parameters.z = if graphics.quality == Quality::Low {
                0.0
            } else {
                1.0
            };
        }
    }
}
