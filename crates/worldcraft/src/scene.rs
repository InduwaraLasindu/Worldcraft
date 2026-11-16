use crate::{session::*, streaming::ChunkRoot};
use bevy::{
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
pub struct Sun;
#[derive(Component)]
pub struct GroundMist;
pub fn setup(mut commands: Commands, mut media: ResMut<Assets<ScatteringMedium>>) {
    commands.spawn(Atmosphere::earth(
        media.add(ScatteringMedium::earth(128, 128)),
    ));
    commands.spawn((
        MainCamera,
        Camera3d::default(),
        Camera::default(),
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
    mut cameras: Query<(Entity, &mut Transform, &mut DistanceFog, &mut Exposure), With<MainCamera>>,
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
    mut applied: Local<Option<(Quality, bool, bool)>>,
    rays: Res<crate::raytrace::RayState>,
) {
    if graphics.cycle {
        graphics.hour = (graphics.hour + time.delta_secs() * 0.025) % 24.0;
    }
    if (player.position.x - session.origin.x).hypot(player.position.z - session.origin.z) > 256.0 {
        session.origin.x = (player.position.x / 256.0).floor() * 256.0;
        session.origin.z = (player.position.z / 256.0).floor() * 256.0;
    }
    for (e, mut t, mut fog, mut exposure) in &mut cameras {
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
        if *applied != Some((graphics.quality, graphics.volumetrics, rays.active)) {
            commands.entity(e).remove::<(
                AtmosphereEnvironmentMapLight,
                AtmosphereSettings,
                Bloom,
                ScreenSpaceAmbientOcclusion,
                VolumetricFog,
            )>();
            if graphics.quality != Quality::Low || rays.active {
                commands.entity(e).insert((
                    AtmosphereSettings::default(),
                    AtmosphereEnvironmentMapLight::default(),
                    Bloom::NATURAL,
                ));
            }
            if graphics.quality == Quality::High {
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
    if *applied != Some((graphics.quality, graphics.volumetrics, rays.active)) {
        shadow_map.size = match graphics.quality {
            Quality::Low => 1024,
            Quality::Balanced => 2048,
            Quality::High => 4096,
        };
        *applied = Some((graphics.quality, graphics.volumetrics, rays.active));
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
        light.shadow_maps_enabled = graphics.quality != Quality::Low
            && (!rays.active || (graphics.quality == Quality::High && graphics.volumetrics));
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
