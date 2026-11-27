//! One transparent batch for local weather and waterfall spray.
use crate::{
    performance::Performance,
    session::{Graphics, Player, Quality, Session},
    weather::Weather,
};
use bevy::{
    asset::RenderAssetUsages,
    camera::visibility::NoFrustumCulling,
    light::NotShadowCaster,
    mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use worldcraft_core::{generation::unit, river, Preset};

const CAPACITY: usize = 744;

#[derive(Component)]
pub struct WeatherParticles;

pub fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut pixels = Vec::with_capacity(32 * 32 * 4);
    for y in 0..32 {
        for x in 0..32 {
            let p = (Vec2::new(x as f32, y as f32) + Vec2::splat(0.5)) / 16.0 - Vec2::ONE;
            let alpha = (1.0 - p.length_squared()).max(0.0).powi(2);
            pixels.extend([255, 255, 255, (alpha * 255.0) as u8]);
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: 32,
            height: 32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = bevy::image::ImageSampler::linear();
    let mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.0f32; 3]; 4])
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, vec![[0.0f32; 4]; 4])
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0f32; 2]; 4])
    .with_inserted_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]));
    commands.spawn((
        Name::new("Weather and waterfall particles"),
        WeatherParticles,
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(images.add(image)),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            cull_mode: None,
            ..default()
        })),
        Transform::default(),
        Visibility::Hidden,
        NoFrustumCulling,
        NotShadowCaster,
        crate::foliage::NoRayProxy,
    ));
}

#[derive(Clone, Copy)]
pub struct Billboard {
    center: Vec3,
    right: Vec3,
    up: Vec3,
    color: [f32; 4],
}

fn volume(p: Vec3, eye: Vec3) -> Vec3 {
    let extent = Vec3::new(32.0, 20.0, 32.0);
    (p - eye + extent * 0.5).rem_euclid(extent) - extent * 0.5
}

pub fn update(
    session: Res<Session>,
    player: Res<Player>,
    graphics: Res<Graphics>,
    weather: Res<Weather>,
    limits: Res<Performance>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut emitter: Single<(&Mesh3d, &mut Transform, &mut Visibility), With<WeatherParticles>>,
    mut cards: Local<Vec<Billboard>>,
) {
    let (handle, transform, visibility) = &mut *emitter;
    let Some(g) = session.generator.as_ref() else {
        **visibility = Visibility::Hidden;
        return;
    };
    if limits.suspended {
        **visibility = Visibility::Hidden;
        return;
    }
    let (budget, spray) = match limits.quality(graphics.quality) {
        Quality::Low => (112.0, 16),
        Quality::Balanced => (340.0, 38),
        Quality::High => (680.0, 64),
    };
    let rotation = player.rotation();
    let right = rotation * Vec3::X;
    let up = rotation * Vec3::Y;
    let eye = Vec3::new(
        player.position.x.rem_euclid(32.0) as f32,
        player.position.y.rem_euclid(20.0) as f32,
        player.position.z.rem_euclid(32.0) as f32,
    );
    let day = 1.0 - 0.84 * crate::scene::night_visibility(graphics.hour);
    let wind = Vec3::new(weather.wind.x, 0.0, weather.wind.y);
    let drift = Vec3::new(weather.drift.x, 0.0, weather.drift.y);
    let rain_velocity = wind * 0.38 - Vec3::Y * 13.0;
    let projected_rain =
        Vec2::new(rain_velocity.dot(right), rain_velocity.dot(up)).normalize_or_zero();
    let streak = right * projected_rain.x + up * projected_rain.y;
    let across = right * projected_rain.y - up * projected_rain.x;
    cards.clear();
    cards.reserve(CAPACITY);
    let seed = g.settings().seed;
    let now = weather.elapsed;
    // Blended intensities share the same atmospheric budget during transitions.
    let strengths = weather.air.truncate();
    let split = strengths.element_sum().max(1.0);
    if player.position.y >= g.settings().sea_level {
        for (kind, amount) in strengths.to_array().into_iter().enumerate() {
            let count = (amount / split * budget) as i64;
            for i in 0..count {
                let noise = |axis| unit(seed ^ (kind as u64 * 313 + 910), i, axis) as f32;
                let start = Vec3::new(noise(0) * 32.0, noise(1) * 20.0, noise(2) * 32.0);
                let wobble =
                    ((now * f64::from(1.2 + noise(3)) + f64::from(noise(4)) * 8.0).sin()) as f32;
                let center = match kind {
                    0 => {
                        start + drift * 0.375
                            - Vec3::Y
                                * (now * f64::from(11.0 + noise(4) * 5.0)).rem_euclid(20.0) as f32
                    }
                    1 => {
                        start
                            + drift * 0.625
                            + Vec3::new(
                                wobble * 0.68,
                                -(now * f64::from(1.3 + noise(4) * 1.8)).rem_euclid(20.0) as f32,
                                wobble * 0.30,
                            )
                    }
                    _ => start + drift + Vec3::new(wobble * 0.60, wobble * 0.82, wobble * 0.48),
                };
                let p = volume(center, eye);
                let d = p.length();
                let edge = ((16.0 - p.x.abs().max(p.z.abs())) / 2.0).clamp(0.0, 1.0)
                    * ((10.0 - p.y.abs()) / 2.0).clamp(0.0, 1.0);
                let opacity = ((d - 1.0) * 0.6).clamp(0.0, 1.0) * edge * (0.5 + 0.5 * noise(5));
                if opacity <= 0.01 {
                    continue;
                }
                let size = noise(6);
                let (a, b, tint, alpha) = match kind {
                    0 => (
                        across * (0.015 + 0.021 * size),
                        streak * (0.34 + 0.38 * size),
                        [0.48, 0.65, 0.83],
                        0.51,
                    ),
                    1 => (
                        right * (0.026 + 0.055 * size),
                        up * (0.026 + 0.055 * size),
                        [0.88, 0.95, 1.0],
                        0.85,
                    ),
                    _ => (
                        right * (0.16 + 0.39 * size),
                        up * (0.10 + 0.26 * size),
                        [0.63, 0.34, 0.14],
                        0.28,
                    ),
                };
                cards.push(Billboard {
                    center: p,
                    right: a,
                    up: b,
                    color: [tint[0] * day, tint[1] * day, tint[2] * day, opacity * alpha],
                });
            }
        }
    }
    if g.settings().preset == Preset::RiverValley {
        let (_, target) = river::waterfall_view(g.settings());
        let fall = bevy::math::DVec3::from_array(target);
        let delta = (fall - player.position).as_vec3();
        if delta.length_squared() < 145.0_f32.powi(2) {
            let range_fade = ((145.0 - delta.length()) / 30.0).clamp(0.0, 1.0);
            let width = g.settings().terrain_scale as f32 * 14.0;
            let foot = g.settings().vertical_scale as f32 * 8.6;
            for i in 0..spray {
                let rand = |axis| unit(seed ^ 0x737072, i, axis) as f32;
                let age =
                    (now * f64::from(0.32 + 0.22 * rand(0)) + f64::from(rand(1))).fract() as f32;
                let p = delta
                    + Vec3::new(
                        (rand(2) * 2.0 - 1.0) * width,
                        -foot + age * (3.0 + 4.0 * rand(3)),
                        -4.0 - age * (5.0 + 7.0 * rand(4)),
                    )
                    + wind * age * 0.22;
                let size = (0.28 + age * 0.95) * (0.7 + rand(5));
                let alpha = 0.34 * (1.0 - age) * (age * 6.0).min(1.0) * range_fade;
                cards.push(Billboard {
                    center: p,
                    right: right * size,
                    up: up * size,
                    color: [0.77 * day, 0.88 * day, day, alpha],
                });
            }
        }
    }
    if cards.is_empty() {
        **visibility = Visibility::Hidden;
        return;
    }
    let Some(mut mesh) = meshes.get_mut(&handle.0) else {
        return;
    };
    let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
    else {
        unreachable!("Particle positions");
    };
    positions.clear();
    for c in cards.iter() {
        positions.extend([
            (c.center - c.right - c.up).to_array(),
            (c.center + c.right - c.up).to_array(),
            (c.center + c.right + c.up).to_array(),
            (c.center - c.right + c.up).to_array(),
        ]);
    }
    let Some(VertexAttributeValues::Float32x4(colors)) = mesh.attribute_mut(Mesh::ATTRIBUTE_COLOR)
    else {
        unreachable!("Particle colors");
    };
    colors.clear();
    for c in cards.iter() {
        colors.extend([c.color; 4]);
    }
    let Some(VertexAttributeValues::Float32x2(uv)) = mesh.attribute_mut(Mesh::ATTRIBUTE_UV_0)
    else {
        unreachable!("Particle UVs");
    };
    uv.resize(4 * cards.len(), [0.0; 2]);
    for chunk in uv.chunks_exact_mut(4) {
        chunk.copy_from_slice(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
    }
    let Some(Indices::U32(indices)) = mesh.indices_mut() else {
        unreachable!("Particle indices");
    };
    indices.clear();
    for card in 0..cards.len() as u32 {
        let i = card * 4;
        indices.extend([i, i + 1, i + 2, i, i + 2, i + 3]);
    }
    transform.translation = (player.position - session.origin).as_vec3();
    **visibility = Visibility::Visible;
}
