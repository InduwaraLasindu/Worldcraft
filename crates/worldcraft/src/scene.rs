use crate::{session::*, streaming::ChunkRoot};
use bevy::{
    camera::{Exposure, Hdr},
    core_pipeline::tonemapping::Tonemapping,
    light::{
        atmosphere::ScatteringMedium, Atmosphere, AtmosphereEnvironmentMapLight,
        CascadeShadowConfigBuilder, DirectionalLightShadowMap, FogVolume, SunDisk, VolumetricFog,
        VolumetricLight,
    },
    pbr::AtmosphereSettings,
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

/// Keep presentation exposure and vendor pre-exposure in the same space.
pub fn outdoor_exposure(hour: f32, compensation: f32) -> bevy::camera::Exposure {
    let elevation = ((hour - 6.0) * std::f32::consts::TAU / 24.0).sin();
    let dusk = (1.0 - elevation.max(0.0)).powi(3);
    bevy::camera::Exposure {
        ev100: bevy::camera::Exposure::EV100_SUNLIGHT + compensation - 2.6 * dusk,
    }
}
fn shadow_size(quality: Quality, software: bool) -> usize {
    let requested = match quality {
        Quality::Low => 1024,
        Quality::Balanced => 2048,
        Quality::High => 4096,
    };
    // Software adapters share process RAM with render targets, scene buffers and
    // driver compilation. Keep manual High usable without allocating four 4K
    // shadow cascades there. Real GPUs retain the requested shadow resolution.
    if software {
        requested.min(1024)
    } else {
        requested
    }
}
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
            &mut Camera,
            &mut Transform,
            &mut DistanceFog,
            &mut Exposure,
        ),
        With<MainCamera>,
    >,
    mut suns: Query<
        (Entity, &mut Transform, &mut DirectionalLight),
        (With<Sun>, Without<MainCamera>),
    >,
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
    mut applied: Local<Option<(Quality, bool, bool, bool, u32)>>,
    mut shadow_applied: Local<Option<(usize, f32)>>,
    rays: Res<crate::raytrace::RayState>,
    limits: Res<crate::performance::Performance>,
    ambient: Option<ResMut<GlobalAmbientLight>>,
) {
    let quality = limits.quality(graphics.quality);
    let options = limits.preferences.render;
    if graphics.cycle && !limits.suspended {
        graphics.hour = (graphics.hour + time.delta_secs() * 0.025) % 24.0;
    }
    if (player.position.x - session.origin.x).hypot(player.position.z - session.origin.z) > 256.0 {
        session.origin.x = (player.position.x / 256.0).floor() * 256.0;
        session.origin.z = (player.position.z / 256.0).floor() * 256.0;
    }
    let a = (graphics.hour - 6.0) / 24.0 * std::f32::consts::TAU;
    let elevation = a.sin();
    // Dawn condensation disperses as the sun climbs. At the same elevation
    // evening haze is weaker. Evaluate once for both distance and volume fog.
    let low_sun = (1.0 - elevation.abs() * 2.3).clamp(0.0, 1.0);
    let dawn_mist = low_sun * low_sun * (0.75 + 0.45 * a.cos());
    let mut next_applied = None;
    for (e, mut camera, mut t, mut fog, mut exposure) in &mut cameras {
        let state = (
            quality,
            graphics.volumetrics,
            rays.active,
            camera.is_active,
            options.fog_steps,
        );
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
            let day = ((elevation + 0.12) * 3.6).clamp(0.0, 1.0);
            let twilight = low_sun * day;
            Color::srgb(
                0.08 + 0.48 * day + 0.16 * twilight,
                0.11 + 0.57 * day - 0.025 * twilight,
                0.16 + 0.59 * day - 0.11 * twilight,
            )
        };
        fog.falloff = if underwater {
            FogFalloff::Exponential { density: 0.055 }
        } else if limits.mode() == crate::performance::Mode::Eco {
            // Fade to the matching sky before the nearest resident tile edge.
            // The surface shader uses horizontal distance so camera altitude
            // does not swallow the foreground. No extra mesh/job work is added.
            let end =
                limits.radius(graphics.radius) as f32 * worldcraft_core::CHUNK_SIZE as f32 - 32.0;
            FogFalloff::Linear {
                start: end * 0.35,
                end,
            }
        } else {
            FogFalloff::Exponential {
                density: 0.00008 * graphics.fog * (1.0 + 0.65 * dawn_mist),
            }
        };
        let clear = if quality == Quality::Low {
            ClearColorConfig::Custom(fog.color)
        } else {
            ClearColorConfig::Default
        };
        let same_clear = match (&camera.clear_color, &clear) {
            (ClearColorConfig::Default, ClearColorConfig::Default) => true,
            (ClearColorConfig::Custom(a), ClearColorConfig::Custom(b)) => a == b,
            _ => false,
        };
        if !same_clear {
            camera.clear_color = clear;
        }
        exposure.ev100 = outdoor_exposure(graphics.hour, graphics.exposure).ev100;
        if *applied != Some(state) {
            commands.entity(e).remove::<(
                AtmosphereEnvironmentMapLight,
                AtmosphereSettings,
                VolumetricFog,
            )>();
            // Bevy's environment probe requires an extracted active 3D view. Attaching
            // it on Home / while a world loads leaves its transform buffer unbound.
            // Reconcile on every camera activation change, including focus suspension.
            if camera.is_active && (quality != Quality::Low || rays.active) {
                commands.entity(e).insert((
                    AtmosphereSettings::default(),
                    AtmosphereEnvironmentMapLight::default(),
                ));
            }
            if camera.is_active && quality == Quality::High {
                if graphics.volumetrics {
                    commands.entity(e).insert(VolumetricFog {
                        ambient_intensity: 0.2,
                        ambient_color: Color::srgb(0.65, 0.75, 0.9),
                        step_count: options.fog_steps,
                        jitter: 0.0,
                        ..default()
                    });
                }
            }
        }
    }
    if *applied != next_applied {
        *applied = next_applied;
    }
    let shadow_pixels = if options.shadows == crate::render_options::EffectQuality::Automatic {
        shadow_size(quality, limits.software_gpu)
    } else {
        options
            .shadow_resolution(
                quality == Quality::High,
                quality == Quality::Low,
                limits.software_gpu,
            )
            .max(512)
    };
    let shadow_distance =
        options
            .shadow_distance
            .min(if limits.software_gpu { 320.0 } else { 768.0 });
    let shadow_changed = *shadow_applied != Some((shadow_pixels, shadow_distance));
    if shadow_changed {
        shadow_map.size = shadow_pixels;
        *shadow_applied = Some((shadow_pixels, shadow_distance));
    }
    if let Some(mut ambient) = ambient {
        // Diffuse sky fill falls with daylight. This restores shaded surface
        // detail while preserving directional shadows and twilight falloff.
        let brightness = 2.0 + 1450.0 * elevation.max(0.0).sqrt();
        if ambient.brightness != brightness {
            ambient.brightness = brightness;
        }
    }
    for (entity, mut t, mut light) in &mut suns {
        if shadow_changed {
            commands.entity(entity).insert(
                CascadeShadowConfigBuilder {
                    first_cascade_far_bound: 30.0,
                    maximum_distance: shadow_distance,
                    ..default()
                }
                .build(),
            );
        }
        let transform =
            Transform::from_translation(Vec3::new(a.cos(), elevation, 0.35).normalize() * 100.0)
                .looking_at(Vec3::ZERO, Vec3::Y);
        if *t != transform {
            *t = transform;
        }
        let illuminance = if elevation > 0.0 {
            100_000.0 * elevation.sqrt()
        } else {
            0.05
        };
        if light.illuminance != illuminance {
            light.illuminance = illuminance;
        }
        let shadows = quality != Quality::Low
            && options.shadows != crate::render_options::EffectQuality::Off
            && (!rays.active || (quality == Quality::High && graphics.volumetrics));
        if light.shadow_maps_enabled != shadows {
            light.shadow_maps_enabled = shadows;
        }
        let color = Color::srgb(
            1.0,
            0.72 + elevation.max(0.0) * 0.28,
            0.51 + elevation.max(0.0) * 0.49,
        );
        if light.color != color {
            light.color = color;
        }
    }
    let sea = session.world.as_ref().map_or(0.0, |w| w.settings.sea_level) as f32;
    for (mut volume, mut transform) in &mut mist {
        transform.translation = Vec3::new(
            (player.position.x - session.origin.x) as f32,
            sea + graphics.mist_height * 0.5,
            (player.position.z - session.origin.z) as f32,
        );
        transform.scale = Vec3::new(8192.0, graphics.mist_height.max(1.0), 8192.0);
        volume.density_factor = graphics.mist_density * (1.0 + 1.35 * dawn_mist);
    }
    for (root, mut t) in &mut roots {
        let o = root.0.origin();
        let translation = Vec3::new(
            (o[0] - session.origin.x) as f32,
            0.0,
            (o[1] - session.origin.z) as f32,
        );
        // A stationary root must not dirty every descendant's global transform.
        // Dense stands share immutable instance transforms until an origin rebase.
        if t.translation != translation {
            t.translation = translation;
        }
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
    fn stationary_roots_and_sun_keep_their_change_ticks_until_they_move() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Session>()
            .init_resource::<Player>()
            .init_resource::<Graphics>()
            .init_resource::<crate::raytrace::RayState>()
            .init_resource::<DirectionalLightShadowMap>()
            .insert_resource(crate::performance::Performance::load(true))
            .add_systems(Update, update);
        let root = app
            .world_mut()
            .spawn((
                ChunkRoot(worldcraft_core::ChunkCoord { x: 0, z: 0 }),
                Transform::default(),
            ))
            .id();
        let sun = app
            .world_mut()
            .spawn((Sun, Transform::default(), DirectionalLight::default()))
            .id();
        app.update();
        let ticks = (
            app.world()
                .entity(root)
                .get_ref::<Transform>()
                .unwrap()
                .last_changed(),
            app.world()
                .entity(sun)
                .get_ref::<Transform>()
                .unwrap()
                .last_changed(),
            app.world()
                .entity(sun)
                .get_ref::<DirectionalLight>()
                .unwrap()
                .last_changed(),
        );
        app.update();
        assert_eq!(
            ticks,
            (
                app.world()
                    .entity(root)
                    .get_ref::<Transform>()
                    .unwrap()
                    .last_changed(),
                app.world()
                    .entity(sun)
                    .get_ref::<Transform>()
                    .unwrap()
                    .last_changed(),
                app.world()
                    .entity(sun)
                    .get_ref::<DirectionalLight>()
                    .unwrap()
                    .last_changed(),
            )
        );
        app.world_mut().resource_mut::<Player>().position.x = 300.0;
        app.update();
        assert_eq!(
            app.world().get::<Transform>(root).unwrap().translation.x,
            -256.0
        );
        assert_ne!(
            ticks.0,
            app.world()
                .entity(root)
                .get_ref::<Transform>()
                .unwrap()
                .last_changed()
        );
    }

    #[test]
    fn software_shadow_allocations_stay_bounded_when_manually_selecting_high() {
        assert_eq!(shadow_size(Quality::High, true), 1024);
        assert_eq!(shadow_size(Quality::Balanced, true), 1024);
        assert_eq!(shadow_size(Quality::High, false), 4096);
        assert_eq!(shadow_size(Quality::Balanced, false), 2048);
    }

    #[test]
    fn eco_haze_covers_the_resident_edge_and_restores_manual_graphics() {
        let mut limits = crate::performance::Performance::load(false);
        limits.preferences.mode = crate::performance::Mode::Eco;
        limits.calibrating = false;
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Session>()
            .init_resource::<Player>()
            .init_resource::<Graphics>()
            .init_resource::<crate::raytrace::RayState>()
            .init_resource::<DirectionalLightShadowMap>()
            .insert_resource(limits)
            .add_systems(Update, update);
        let camera = app
            .world_mut()
            .spawn((
                MainCamera,
                Camera::default(),
                Transform::default(),
                DistanceFog::default(),
                Exposure::SUNLIGHT,
            ))
            .id();
        app.world_mut().resource_mut::<Graphics>().quality = Quality::High;
        app.update();
        let fog = app.world().get::<DistanceFog>(camera).unwrap();
        let FogFalloff::Linear { start, end } = fog.falloff else {
            panic!("Eco must blend its short horizon");
        };
        assert!(end <= 224.0);
        assert!(start >= 50.0);
        assert!(
            matches!(app.world().get::<Camera>(camera).unwrap().clear_color,
            ClearColorConfig::Custom(color) if color == fog.color)
        );
        app.world_mut()
            .resource_mut::<crate::performance::Performance>()
            .preferences
            .mode = crate::performance::Mode::Performance;
        app.update();
        assert!(matches!(
            app.world().get::<Camera>(camera).unwrap().clear_color,
            ClearColorConfig::Default
        ));
        let fog = app.world().get::<DistanceFog>(camera).unwrap();
        assert!(matches!(fog.falloff, FogFalloff::Exponential { density } if density < 0.001));
    }

    #[test]
    fn atmosphere_follows_camera_lifecycle_without_changing_quality() {
        use bevy::{post_process::bloom::Bloom, render::view::ColorGrading};
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Session>()
            .init_resource::<Player>()
            .init_resource::<Graphics>()
            .init_resource::<crate::render_quality::Reconstruction>()
            .init_resource::<crate::fsr::FsrState>()
            .init_resource::<crate::raytrace::RayState>()
            .init_resource::<DirectionalLightShadowMap>()
            .insert_resource(crate::performance::Performance::load(true))
            .add_systems(Update, (update, crate::render_quality::update).chain());
        app.world_mut().resource_mut::<Graphics>().quality = Quality::Balanced;
        let camera = app
            .world_mut()
            .spawn((
                MainCamera,
                Projection::default(),
                ColorGrading::default(),
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
