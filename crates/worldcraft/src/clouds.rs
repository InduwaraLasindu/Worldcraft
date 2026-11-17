//! Bounded textured cloud volume for High. Uses the existing volumetric integrator;
//! this is not a weather simulation or a self-shadowed production cloud renderer.
use crate::{performance::Performance, session::*};
use bevy::{
    asset::RenderAssetUsages,
    image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    light::FogVolume,
    math::DVec3,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use worldcraft_core::generation::unit;
const SIDE: usize = 64;
const WIDTH: f64 = 8192.0;
const DENSITY: f32 = 0.0003;
#[derive(Component)]
pub struct CloudLayer {
    tile: IVec2,
}
fn noise(x: f32, y: f32, z: f32, period: i64) -> f32 {
    let cell = [x.floor() as i64, y.floor() as i64, z.floor() as i64];
    let f = Vec3::new(x.fract(), y.fract(), z.fract());
    let u = f * f * (Vec3::splat(3.0) - f * 2.0);
    let sample = |dx: i64, dy: i64, dz: i64| {
        unit(
            0x636c_6f75_6473 ^ ((cell[1] + dy).rem_euclid(period) as u64).wrapping_mul(7919),
            (cell[0] + dx).rem_euclid(period),
            (cell[2] + dz).rem_euclid(period),
        ) as f32
    };
    let mix = |a: f32, b: f32, t: f32| a + (b - a) * t;
    mix(
        mix(
            mix(sample(0, 0, 0), sample(1, 0, 0), u.x),
            mix(sample(0, 1, 0), sample(1, 1, 0), u.x),
            u.y,
        ),
        mix(
            mix(sample(0, 0, 1), sample(1, 0, 1), u.x),
            mix(sample(0, 1, 1), sample(1, 1, 1), u.x),
            u.y,
        ),
        u.z,
    )
}
fn density(x: f32, y: f32, z: f32) -> f32 {
    let base = noise(x * 16.0, y * 4.0, z * 16.0, 16);
    let detail = noise(x * 32.0, y * 8.0, z * 32.0, 32);
    let envelope = (y / 0.18).clamp(0.0, 1.0) * ((1.0 - y) / 0.32).clamp(0.0, 1.0);
    ((base * 0.76 + detail * 0.24 - 0.49) * 4.0).clamp(0.0, 1.0) * envelope
}
fn texture() -> Image {
    let mut data = Vec::with_capacity(SIDE * SIDE * SIDE);
    for z in 0..SIDE {
        for y in 0..SIDE {
            for x in 0..SIDE {
                data.push(
                    (density(
                        x as f32 / SIDE as f32,
                        y as f32 / (SIDE - 1) as f32,
                        z as f32 / SIDE as f32,
                    ) * 255.0)
                        .round() as u8,
                );
            }
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: SIDE as u32,
            height: SIDE as u32,
            depth_or_array_layers: SIDE as u32,
        },
        TextureDimension::D3,
        data,
        TextureFormat::R8Unorm,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_w: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::ClampToEdge,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        ..default()
    });
    image
}
pub fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let density = images.add(texture());
    // The fog integrator estimates sunlight attenuation from each volume's
    // bounding radius. A single 32 km slab falsely extinguished almost all of
    // that light. Nine smaller, nonoverlapping cells share one density texture.
    // This is a bounded, single-scattering approximation, not cloud self-shadowing.
    for z in -1..=1 {
        for x in -1..=1 {
            commands.spawn((
                CloudLayer {
                    tile: IVec2::new(x, z),
                },
                FogVolume {
                    density_factor: 0.0,
                    density_texture: Some(density.clone()),
                    absorption: 0.008,
                    scattering: 0.70,
                    scattering_asymmetry: 0.15,
                    // Compensate for the inexpensive single-scattering model's
                    // missing sky/multiple scattering; intentionally an art control.
                    light_intensity: 4.0,
                    ..default()
                },
                Visibility::Hidden,
                Transform::from_xyz(x as f32 * WIDTH as f32, 2700.0, z as f32 * WIDTH as f32)
                    .with_scale(Vec3::new(WIDTH as f32, 1200.0, WIDTH as f32)),
            ));
        }
    }
}
fn offset(camera: DVec3, seconds: f64, breeze: f64) -> Vec3 {
    Vec3::new(
        ((camera.x + seconds * breeze * 4.0) / WIDTH).rem_euclid(1.0) as f32,
        0.0,
        (camera.z / WIDTH).rem_euclid(1.0) as f32,
    )
}
pub fn update(
    session: Res<Session>,
    player: Res<Player>,
    graphics: Res<Graphics>,
    limits: Res<Performance>,
    time: Res<Time>,
    mut clouds: Query<(&CloudLayer, &mut FogVolume, &mut Transform, &mut Visibility)>,
) {
    let active = !limits.suspended
        && graphics.clouds
        && graphics.volumetrics
        && limits.quality(graphics.quality) == Quality::High;
    let center = DVec3::new(
        (player.position.x / WIDTH).floor() * WIDTH,
        0.0,
        (player.position.z / WIDTH).floor() * WIDTH,
    );
    for (layer, mut fog, mut transform, mut visibility) in &mut clouds {
        let anchor = center
            + DVec3::new(
                layer.tile.x as f64 * WIDTH,
                0.0,
                layer.tile.y as f64 * WIDTH,
            );
        transform.translation.x = (anchor.x - session.origin.x) as f32;
        transform.translation.z = (anchor.z - session.origin.z) as f32;
        fog.density_texture_offset =
            offset(anchor, time.elapsed_secs_f64(), graphics.breeze as f64);
        fog.density_factor = if active { DENSITY } else { 0.0 };
        *visibility = if active {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cloud_volume_is_bounded_sparse_and_has_clear_vertical_boundaries() {
        let image = texture();
        let data = image.data.unwrap();
        assert_eq!(data.len(), 64 * 64 * 64);
        let coverage = data.iter().filter(|&&v| v > 16).count() as f32 / data.len() as f32;
        assert!((0.05..0.65).contains(&coverage));
        for x in 0..SIDE {
            for z in 0..SIDE {
                assert_eq!(data[z * SIDE * SIDE + x], 0);
                assert_eq!(data[z * SIDE * SIDE + (SIDE - 1) * SIDE + x], 0);
            }
        }
        assert!((density(0.23, 0.5, 0.67) - density(1.23, 0.5, 0.67)).abs() < 0.00001);
    }
    #[test]
    fn moving_volume_anchor_preserves_world_space_cloud_phase() {
        let a = DVec3::new(-999_999_900.0, 100.0, 450.0);
        let b = a + DVec3::new(128.0, 0.0, -256.0);
        let point = a + DVec3::new(450.0, 2700.0, 900.0);
        let sample = |camera: DVec3| {
            ((point - camera).as_vec3() / WIDTH as f32 + offset(camera, 100.0, 0.4))
                .map(|v| v.rem_euclid(1.0))
        };
        assert!((sample(a) - sample(b)).length() < 0.00001);
    }

    #[test]
    fn adjacent_cloud_cells_share_phase_and_reuse_one_bounded_texture() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Image>>()
            .add_systems(Startup, setup);
        app.update();
        assert_eq!(app.world().resource::<Assets<Image>>().len(), 1);
        let mut volumes = app.world_mut().query::<(&CloudLayer, &FogVolume)>();
        assert_eq!(volumes.iter(app.world()).len(), 9);
        for (layer, fog) in volumes.iter(app.world()) {
            let a = DVec3::new(
                layer.tile.x as f64 * WIDTH,
                0.0,
                layer.tile.y as f64 * WIDTH,
            );
            let b = a + DVec3::X * WIDTH;
            let point = a + DVec3::X * (WIDTH * 0.5);
            let phase = |anchor: DVec3| {
                (((point - anchor).x / WIDTH) + offset(anchor, 500.0, 0.4).x as f64).rem_euclid(1.0)
            };
            assert!((phase(a) - phase(b)).abs() < 0.00001);
            let radius = Vec3::new(WIDTH as f32, 1200.0, WIDTH as f32).length() * 0.5;
            let sunlight = (-DENSITY * radius * (fog.absorption + fog.scattering)).exp();
            assert!(
                sunlight > 0.25,
                "Volume dimensions incorrectly extinguished sunlight"
            );
        }
    }
}
