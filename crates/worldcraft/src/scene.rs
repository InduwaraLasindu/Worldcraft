use crate::{session::*, streaming::ChunkRoot};
use bevy::{
    anti_alias::fxaa::Fxaa,
    camera::{Exposure, Hdr},
    core_pipeline::tonemapping::Tonemapping,
    light::{
        atmosphere::ScatteringMedium, Atmosphere, AtmosphereEnvironmentMapLight,
        CascadeShadowConfigBuilder, DirectionalLightShadowMap, FogVolume, SunDisk, VolumetricFog,
        VolumetricLight,
    },
    pbr::{
        AtmosphereSettings, ScreenSpaceAmbientOcclusion, ScreenSpaceAmbientOcclusionQualityLevel,
    },
    post_process::bloom::Bloom,
    prelude::*,
};

#[derive(Component)]
pub struct MainCamera;
#[derive(Component)]
pub struct UiCamera;
#[derive(Component)]
pub struct Sun;
#[derive(Component)]
pub struct GroundMist;
#[derive(Resource)]
pub struct SceneTarget {
    pub image: Handle<Image>,
    pub size: UVec2,
}
pub fn setup(
    mut commands: Commands,
    mut media: ResMut<Assets<ScatteringMedium>>,
    mut images: ResMut<Assets<Image>>,
    launch: Res<Launch>,
) {
    let image = images.add(Image::new_target_texture(
        960,
        600,
        bevy::render::render_resource::TextureFormat::Rgba8Unorm,
        Some(bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb),
    ));
    commands.insert_resource(SceneTarget {
        image: image.clone(),
        size: UVec2::new(960, 600),
    });
    commands.spawn(Atmosphere::earth(
        media.add(ScatteringMedium::earth(128, 128)),
    ));
    commands.spawn((
        UiCamera,
        // UI paint uses its camera directly; exclude world meshes from its visibility work.
        bevy::camera::visibility::RenderLayers::layer(30),
        Camera2d,
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        Msaa::Off,
        bevy_egui::PrimaryEguiContext,
    ));
    let mut main = commands.spawn((
        MainCamera,
        // The window-facing camera is UI-only. Shadows must follow the world
        // observer rather than implicitly choosing that camera at the origin.
        bevy::camera::ShadowLodOrigin,
        Camera3d::default(),
        Camera {
            is_active: false,
            order: -1,
            ..default()
        },
        Hdr,
        Transform::from_xyz(0.0, 100.0, 0.0),
        Msaa::Off,
        Tonemapping::AgX,
        Projection::Perspective(PerspectiveProjection {
            fov: 70.0_f32.to_radians(),
            near: 0.1,
            far: 150_000.0,
            ..default()
        }),
        Exposure::SUNLIGHT,
        DistanceFog {
            color: Color::srgb(0.56, 0.68, 0.75),
            falloff: FogFalloff::Exponential { density: 0.0008 },
            ..default()
        },
    ));
    if !launch.ray_smoke {
        main.insert(bevy::camera::RenderTarget::Image(image.into()));
    }
    commands.spawn((
        Sun,
        VolumetricLight,
        SunDisk::EARTH,
        DirectionalLight {
            illuminance: 100_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        CascadeShadowConfigBuilder {
            first_cascade_far_bound: 30.0,
            maximum_distance: 320.0,
            ..default()
        }
        .build(),
        Transform::from_xyz(1.0, 1.0, 0.4).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        GroundMist,
        FogVolume {
            density_factor: 0.0002,
            absorption: 0.05,
            scattering: 0.8,
            ..default()
        },
        Transform::from_xyz(0.0, 150.0, 0.0).with_scale(Vec3::new(8192.0, 300.0, 8192.0)),
    ));
}
#[allow(clippy::too_many_arguments)]
pub fn update(
    mut commands: Commands,
    mut session: ResMut<Session>,
    player: Res<Player>,
    mut graphics: ResMut<Graphics>,
    time: Res<Time>,
    mut cameras: Query<
        (
            Entity,
            &Camera,
            &mut Transform,
            &mut DistanceFog,
            &mut Exposure,
        ),
        With<MainCamera>,
    >,
    mut suns: Query<(&mut Transform, &mut DirectionalLight), (With<Sun>, Without<MainCamera>)>,
    mut roots: Query<(&ChunkRoot, &mut Transform), (Without<Sun>, Without<MainCamera>)>,
    mut mist: Query<
        (&mut FogVolume, &mut Transform),
        (
            With<GroundMist>,
            Without<ChunkRoot>,
            Without<Sun>,
            Without<MainCamera>,
        ),
    >,
    mut shadow_map: ResMut<DirectionalLightShadowMap>,
    mut applied: Local<Option<(Quality, bool, bool, bool)>>,
    rays: Res<crate::raytrace::RayState>,
    limits: Res<crate::performance::Performance>,
) {
    let quality = limits.quality(graphics.quality);
    if graphics.cycle && !limits.suspended {
        graphics.hour = (graphics.hour + time.delta_secs() * 0.025) % 24.0;
    }
    if (player.position.x - session.origin.x).hypot(player.position.z - session.origin.z) > 256.0 {
        session.origin.x = (player.position.x / 256.0).floor() * 256.0;
        session.origin.z = (player.position.z / 256.0).floor() * 256.0;
    }
    let mut next_applied = None;
    for (e, camera, mut t, mut fog, mut exposure) in &mut cameras {
        let state = (quality, graphics.volumetrics, rays.active, camera.is_active);
        next_applied = Some(state);
        t.translation = (player.position - session.origin).as_vec3();
        t.rotation = player.rotation();
        let underwater = session
            .world
            .as_ref()
            .is_some_and(|w| player.position.y < w.settings.sea_level);
        fog.color = if underwater {
            Color::srgb(0.035, 0.19, 0.23)
        } else {
            Color::srgb(0.56, 0.68, 0.75)
        };
        fog.falloff = FogFalloff::Exponential {
            density: if underwater {
                0.055
            } else {
                0.00008 * graphics.fog
            },
        };
        exposure.ev100 = 13.0 + graphics.exposure;
        if *applied != Some(state) {
            commands.entity(e).remove::<(
                AtmosphereEnvironmentMapLight,
                AtmosphereSettings,
                Fxaa,
                Bloom,
                ScreenSpaceAmbientOcclusion,
                VolumetricFog,
            )>();
            // Bevy's environment probe requires an extracted active 3D view. Attaching
            // it on Home / while a world loads leaves its transform buffer unbound.
            // Reconcile on every camera activation change, including focus suspension.
            if camera.is_active && (quality != Quality::Low || rays.active) {
                commands.entity(e).insert((
                    Fxaa::default(),
                    AtmosphereSettings::default(),
                    AtmosphereEnvironmentMapLight::default(),
                    Bloom::NATURAL,
                ));
            }
            if camera.is_active && quality == Quality::High {
                if graphics.volumetrics {
                    commands.entity(e).insert(VolumetricFog {
                        ambient_intensity: 0.2,
                        ambient_color: Color::srgb(0.65, 0.75, 0.9),
                        step_count: 24,
                        jitter: 0.0,
                        ..default()
                    });
                }
                if !rays.active {
                    commands.entity(e).insert(ScreenSpaceAmbientOcclusion {
                        quality_level: ScreenSpaceAmbientOcclusionQualityLevel::High,
                        ..default()
                    });
                }
            }
        }
    }
    if *applied != next_applied {
        shadow_map.size = match quality {
            Quality::Low => 1024,
            Quality::Balanced => 2048,
            Quality::High => 4096,
        };
        *applied = next_applied;
    }
    let a = (graphics.hour - 6.0) / 24.0 * std::f32::consts::TAU;
    let elevation = a.sin();
    for (mut t, mut light) in &mut suns {
        *t = Transform::from_translation(Vec3::new(a.cos(), elevation, 0.35).normalize() * 100.0)
            .looking_at(Vec3::ZERO, Vec3::Y);
        light.illuminance = if elevation > 0.0 {
            100_000.0 * elevation.sqrt()
        } else {
            0.05
        };
        light.shadow_maps_enabled = quality != Quality::Low
            && (!rays.active || (quality == Quality::High && graphics.volumetrics));
        light.color = Color::srgb(
            1.0,
            0.72 + elevation.max(0.0) * 0.28,
            0.51 + elevation.max(0.0) * 0.49,
        );
    }
    let sea = session.world.as_ref().map_or(0.0, |w| w.settings.sea_level) as f32;
    for (mut volume, mut transform) in &mut mist {
        transform.translation = Vec3::new(
            (player.position.x - session.origin.x) as f32,
            sea + graphics.mist_height * 0.5,
            (player.position.z - session.origin.z) as f32,
        );
        transform.scale = Vec3::new(8192.0, graphics.mist_height.max(1.0), 8192.0);
        volume.density_factor = graphics.mist_density;
    }
    for (root, mut t) in &mut roots {
        let o = root.0.origin();
        t.translation = Vec3::new(
            (o[0] - session.origin.x) as f32,
            0.0,
            (o[1] - session.origin.z) as f32,
        );
    }
}

/// Render the full 3D pipeline into a smaller image, composited beneath full-resolution UI.
/// Unlike a main-pass viewport override this also reduces post-process texture sizes.
pub fn resolution(
    limits: Res<crate::performance::Performance>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut target: ResMut<SceneTarget>,
    mut images: ResMut<Assets<Image>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let scale = if limits.calibrating {
        let cap = if limits.software_gpu {
            256.0
        } else if worldcraft_core::hardware::recommend_eco(limits.cpu_threads, limits.memory) {
            640.0
        } else {
            1280.0
        };
        (cap / window.physical_width().max(1) as f32).min(1.0)
    } else {
        limits.render_scale()
    };
    let size = UVec2::new(
        (window.physical_width() as f32 * scale).round().max(1.0) as u32,
        (window.physical_height() as f32 * scale).round().max(1.0) as u32,
    );
    if size != target.size {
        if let Some(mut image) = images.get_mut(&target.image) {
            image.resize(bevy::render::render_resource::Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            });
            target.size = size;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atmosphere_follows_camera_lifecycle_without_changing_quality() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Session>()
            .init_resource::<Player>()
            .init_resource::<Graphics>()
            .init_resource::<crate::raytrace::RayState>()
            .init_resource::<DirectionalLightShadowMap>()
            .insert_resource(crate::performance::Performance::load(true))
            .add_systems(Update, update);
        app.world_mut().resource_mut::<Graphics>().quality = Quality::Balanced;
        let camera = app
            .world_mut()
            .spawn((
                MainCamera,
                Camera {
                    is_active: false,
                    ..default()
                },
                Transform::default(),
                DistanceFog::default(),
                Exposure::SUNLIGHT,
            ))
            .id();
        // Detailed worlds can spend many frames loading with an inactive main view.
        for active in [false, false, true, true, false, false, true] {
            app.world_mut().get_mut::<Camera>(camera).unwrap().is_active = active;
            app.update();
            let entity = app.world().entity(camera);
            assert_eq!(entity.contains::<AtmosphereEnvironmentMapLight>(), active);
            assert_eq!(entity.contains::<AtmosphereSettings>(), active);
            assert_eq!(entity.contains::<Bloom>(), active);
        }
    }
}
