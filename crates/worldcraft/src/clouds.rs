//! Bounded world-anchored cloud raymarch in HDR, before tonemapping.
//! One shared 3D noise allocation replaces the old nine fog-box boundaries.
use crate::{performance::Performance, scene::MainCamera, session::*};
use bevy::{
    asset::{load_internal_asset, uuid_handle},
    core_pipeline::{
        prepass::ViewPrepassTextures,
        schedule::{Core3d, Core3dSystems},
        tonemapping::tonemapping,
        FullscreenShader,
    },
    prelude::*,
    render::{
        extract_component::{
            ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
            UniformComponentPlugin,
        },
        render_resource::{binding_types::*, *},
        renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery},
        view::{ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
        GpuResourceAppExt, Render, RenderApp, RenderStartup, RenderSystems,
    },
    shader::Shader,
};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use worldcraft_core::generation::unit;
const SIDE: usize = 64;
const SHADER: Handle<Shader> = uuid_handle!("2ba5ba5d-d6c6-4467-b507-f0951c5f3ef7");

#[derive(Component, Clone, Copy, ExtractComponent, ShaderType)]
#[extract_app(RenderApp)]
#[extract_component_filter(With<Camera>)]
pub struct SkySettings {
    anchor: Vec4,
    sun: Vec4,
    weather: Vec4,
    sky: Vec4,
    sampling: Vec4,
}

#[derive(Resource, Clone, Default)]
pub struct CloudGpuEvidence(pub Arc<AtomicU64>);
impl CloudGpuEvidence {
    pub fn draws(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

pub struct CloudPlugin;
impl Plugin for CloudPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, SHADER, "sky_post.wesl", Shader::from_wesl);
        app.add_plugins((
            ExtractComponentPlugin::<SkySettings>::default(),
            UniformComponentPlugin::<SkySettings>::default(),
        ));
        let evidence = CloudGpuEvidence::default();
        app.insert_resource(evidence.clone());
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render
                .insert_resource(evidence)
                .init_gpu_resource::<SpecializedRenderPipelines<SkyPipeline>>()
                .add_systems(RenderStartup, init_pipeline)
                .add_systems(Render, prepare_views.in_set(RenderSystems::Prepare))
                .add_systems(
                    Core3d,
                    render_sky
                        .in_set(Core3dSystems::PostProcess)
                        .before(tonemapping),
                );
        }
    }
}

pub fn update(
    mut commands: Commands,
    session: Res<Session>,
    graphics: Res<Graphics>,
    limits: Res<Performance>,
    time: Res<Time>,
    cameras: Query<(Entity, &Camera), With<MainCamera>>,
) {
    let active =
        !limits.suspended && graphics.clouds && limits.quality(graphics.quality) == Quality::High;
    let seed = session.world.as_ref().map_or(0, |w| w.settings.seed);
    let rain = session.world.as_ref().map_or(0.65, |w| w.settings.rainfall) as f32;
    let sea = session.world.as_ref().map_or(0.0, |w| w.settings.sea_level) as f32;
    let angle = (graphics.hour - 6.0) / 24.0 * std::f32::consts::TAU;
    let elevation = angle.sin();
    let sun = Vec3::new(angle.cos(), elevation, 0.35).normalize();
    for (entity, camera) in &cameras {
        if !active || !camera.is_active {
            commands.entity(entity).remove::<SkySettings>();
            continue;
        }
        commands.entity(entity).insert(SkySettings {
            anchor: Vec4::new(
                session.origin.x.rem_euclid(32768.0) as f32,
                0.0,
                session.origin.z.rem_euclid(32768.0) as f32,
                sea + 1600.0,
            ),
            sun: sun.extend(if elevation > 0.0 {
                100000.0 * elevation.sqrt()
            } else {
                0.05
            }),
            weather: Vec4::new(
                (time.elapsed_secs_f64() * graphics.breeze as f64 * 4.0).rem_euclid(32768.0) as f32,
                unit(seed ^ 0xc10d, 0, 0) as f32,
                unit(seed ^ 0xc10e, 0, 0) as f32,
                (time.elapsed_secs_f64() * graphics.breeze as f64 * 0.92).rem_euclid(32768.0)
                    as f32,
            ),
            sky: Vec4::new(elevation, angle, 0.65, 0.66 - rain * 0.12),
            sampling: Vec4::new(
                limits.preferences.render.cloud_steps as f32,
                limits.preferences.render.cloud_coverage,
                0.0,
                0.0,
            ),
        });
    }
}

fn noise(x: f32, y: f32, z: f32, period: i64) -> f32 {
    let p = Vec3::new(x, y, z);
    let cell = p.floor();
    let f = p - cell;
    let u = f * f * (Vec3::splat(3.0) - f * 2.0);
    let sample = |dx: i64, dy: i64, dz: i64| {
        unit(
            0xc10d ^ ((cell.y as i64 + dy).rem_euclid(period) as u64).wrapping_mul(7919),
            (cell.x as i64 + dx).rem_euclid(period),
            (cell.z as i64 + dz).rem_euclid(period),
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
fn noise_bytes() -> Vec<u8> {
    let mut level = Vec::with_capacity(SIDE * SIDE * SIDE);
    for z in 0..SIDE {
        for y in 0..SIDE {
            for x in 0..SIDE {
                let p = Vec3::new(x as f32, y as f32, z as f32) / SIDE as f32;
                let value = noise(p.x * 8.0, p.y * 8.0, p.z * 8.0, 8) * 0.62
                    + noise(p.x * 16.0, p.y * 16.0, p.z * 16.0, 16) * 0.26
                    + noise(p.x * 32.0, p.y * 32.0, p.z * 32.0, 32) * 0.12;
                level.push((value * 255.0).round() as u8);
            }
        }
    }
    let mut data = level.clone();
    let mut side = SIDE;
    while side > 1 {
        let next = side / 2;
        let mut mip = Vec::with_capacity(next * next * next);
        for z in 0..next {
            for y in 0..next {
                for x in 0..next {
                    let mut sum = 0_u32;
                    for dz in 0..2 {
                        for dy in 0..2 {
                            for dx in 0..2 {
                                sum += level
                                    [(z * 2 + dz) * side * side + (y * 2 + dy) * side + x * 2 + dx]
                                    as u32;
                            }
                        }
                    }
                    mip.push(((sum + 4) / 8) as u8);
                }
            }
        }
        data.extend_from_slice(&mip);
        level = mip;
        side = next;
    }
    data
}

#[derive(Resource)]
struct SkyPipeline {
    layout: BindGroupLayoutDescriptor,
    sampler: Sampler,
    noise_sampler: Sampler,
    noise_view: TextureView,
    _noise: Texture,
    fullscreen: FullscreenShader,
}
fn init_pipeline(
    mut commands: Commands,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    fullscreen: Res<FullscreenShader>,
) {
    let layout = BindGroupLayoutDescriptor::new(
        "worldcraft_sky_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                texture_2d(TextureSampleType::Depth),
                texture_3d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                uniform_buffer::<ViewUniform>(true),
                uniform_buffer::<SkySettings>(true),
            ),
        ),
    );
    let texture = device.create_texture_with_data(
        &queue,
        &TextureDescriptor {
            label: Some("worldcraft_cloud_noise"),
            size: Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 64,
            },
            mip_level_count: 7,
            sample_count: 1,
            dimension: TextureDimension::D3,
            format: TextureFormat::R8Unorm,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        },
        TextureDataOrder::MipMajor,
        &noise_bytes(),
    );
    let noise_view = texture.create_view(&TextureViewDescriptor::default());
    let sampler = device.create_sampler(&SamplerDescriptor {
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        ..default()
    });
    let noise_sampler = device.create_sampler(&SamplerDescriptor {
        address_mode_u: AddressMode::Repeat,
        address_mode_v: AddressMode::Repeat,
        address_mode_w: AddressMode::Repeat,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        mipmap_filter: MipmapFilterMode::Linear,
        ..default()
    });
    commands.insert_resource(SkyPipeline {
        layout,
        sampler,
        noise_sampler,
        noise_view,
        _noise: texture,
        fullscreen: fullscreen.clone(),
    });
}
impl SpecializedRenderPipeline for SkyPipeline {
    type Key = TextureFormat;
    fn specialize(&self, format: TextureFormat) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("worldcraft_cloud_sky".into()),
            layout: vec![self.layout.clone()],
            vertex: self.fullscreen.to_vertex_state(),
            fragment: Some(FragmentState {
                shader: SHADER,
                targets: vec![Some(ColorTargetState {
                    format,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            ..default()
        }
    }
}
#[derive(Component)]
struct SkyPipelineId(CachedRenderPipelineId);
fn prepare_views(
    mut commands: Commands,
    cache: Res<PipelineCache>,
    pipeline: Res<SkyPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<SkyPipeline>>,
    views: Query<(Entity, &ViewTarget), With<SkySettings>>,
) {
    for (entity, target) in &views {
        let id = pipelines.specialize(&cache, &pipeline, target.main_texture_format());
        commands.entity(entity).insert(SkyPipelineId(id));
    }
}
#[derive(PartialEq, Eq)]
struct GroupKey(TextureViewId, TextureViewId, BufferId, BufferId);
struct CachedGroup {
    key: GroupKey,
    group: BindGroup,
}
#[allow(clippy::too_many_arguments)]
fn render_sky(
    view: ViewQuery<(
        &ViewTarget,
        &SkyPipelineId,
        &ViewPrepassTextures,
        &ViewUniformOffset,
        &DynamicUniformIndex<SkySettings>,
    )>,
    pipeline: Res<SkyPipeline>,
    cache: Res<PipelineCache>,
    view_uniforms: Res<ViewUniforms>,
    settings: Res<ComponentUniforms<SkySettings>>,
    evidence: Res<CloudGpuEvidence>,
    mut ctx: RenderContext,
    mut groups: Local<Vec<CachedGroup>>,
) {
    let (target, id, depth, view_offset, index) = view.into_inner();
    let Some(compiled) = cache.get_render_pipeline(id.0) else {
        return;
    };
    let Some(depth) = depth.depth_only_view() else {
        return;
    };
    let Some(view_binding) = view_uniforms.uniforms.binding() else {
        return;
    };
    let Some(settings_binding) = settings.uniforms().binding() else {
        return;
    };
    let post = target.post_process_write();
    let key = GroupKey(
        post.source.id(),
        depth.id(),
        view_uniforms.uniforms.buffer().unwrap().id(),
        settings.uniforms().buffer().unwrap().id(),
    );
    let at = if let Some(at) = groups.iter().position(|g| g.key == key) {
        at
    } else {
        // Ping-pong source views need at most two cached groups. Resizes and
        // device recovery replace those entries instead of retaining old views.
        if groups.len() == 2 {
            groups.remove(0);
        }
        let group = ctx.render_device().create_bind_group(
            Some("worldcraft_cloud_group"),
            &cache.get_bind_group_layout(&pipeline.layout),
            &BindGroupEntries::sequential((
                post.source,
                &pipeline.sampler,
                depth,
                &pipeline.noise_view,
                &pipeline.noise_sampler,
                view_binding,
                settings_binding,
            )),
        );
        groups.push(CachedGroup { key, group });
        groups.len() - 1
    };
    let attachment = [Some(RenderPassColorAttachment {
        view: post.destination,
        depth_slice: None,
        resolve_target: None,
        ops: Operations::default(),
    })];
    let mut pass = ctx
        .command_encoder()
        .begin_render_pass(&RenderPassDescriptor {
            label: Some("worldcraft_cloud_sky"),
            color_attachments: &attachment,
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    pass.set_pipeline(compiled);
    pass.set_bind_group(0, &groups[at].group, &[view_offset.offset, index.index()]);
    pass.draw(0..3, 0..1);
    evidence.0.fetch_add(1, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn periodic_noise_has_bounded_complete_mips_and_stable_negative_phase() {
        let bytes = noise_bytes();
        assert_eq!(bytes.len(), 299593);
        assert!(bytes.iter().any(|&v| v < 80) && bytes.iter().any(|&v| v > 175));
        assert!((noise(-0.17, 0.31, 0.91, 8) - noise(7.83, 0.31, 0.91, 8)).abs() < 0.00001);
        assert_eq!(SkySettings::min_size().get(), 80);
    }
    #[test]
    fn cloud_pass_is_removed_when_the_camera_is_inactive_or_quality_is_low() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Session>()
            .init_resource::<Graphics>()
            .insert_resource(Performance::load(true))
            .add_systems(Update, update);
        let camera = app
            .world_mut()
            .spawn((
                MainCamera,
                Camera {
                    is_active: false,
                    ..default()
                },
            ))
            .id();
        {
            let mut g = app.world_mut().resource_mut::<Graphics>();
            g.quality = Quality::High;
            g.clouds = true;
            g.volumetrics = false;
        }
        app.update();
        assert!(app.world().get::<SkySettings>(camera).is_none());
        app.world_mut().get_mut::<Camera>(camera).unwrap().is_active = true;
        app.update();
        assert!(app.world().get::<SkySettings>(camera).is_some());
        app.world_mut().resource_mut::<Graphics>().quality = Quality::Low;
        app.update();
        assert!(app.world().get::<SkySettings>(camera).is_none());
    }
}
