//! Reconcile real camera effects and report the reconstruction method actually used.
use crate::{
    performance::Performance,
    render_options::*,
    scene::MainCamera,
    session::{Graphics, Player, Quality, Session},
};
use bevy::{
    anti_alias::{
        contrast_adaptive_sharpening::ContrastAdaptiveSharpening,
        fxaa::Fxaa,
        smaa::{Smaa, SmaaPreset},
        taa::TemporalAntiAliasing,
    },
    core_pipeline::{
        prepass::{DepthPrepass, MotionVectorPrepass, NormalPrepass},
        tonemapping::Tonemapping,
    },
    ecs::system::SystemParam,
    math::DVec3,
    pbr::{ScreenSpaceAmbientOcclusion, ScreenSpaceAmbientOcclusionQualityLevel},
    post_process::{
        bloom::Bloom,
        dof::{DepthOfField, DepthOfFieldMode},
    },
    prelude::*,
    render::{
        camera::{MipBias, TemporalJitter},
        view::ColorGrading,
    },
    window::{PresentMode, PrimaryWindow},
};

#[derive(Resource, Default)]
pub struct Reconstruction {
    pub dlss: bool,
    pub dlss_reason: String,
    pub active: Upscaler,
    pub status: String,
    pub history_resets: u64,
}

#[derive(SystemParam)]
pub struct UiInfo<'w> {
    pub state: Res<'w, Reconstruction>,
    pub fsr: Res<'w, crate::fsr::FsrState>,
    pub caps: Res<'w, crate::capabilities::Capabilities>,
    pub target: Res<'w, crate::scene::SceneTarget>,
    pub surfaces: Res<'w, crate::terrain_texture::ActiveSurfaces>,
}
#[derive(SystemParam)]
pub struct FrameInput<'w> {
    limits: Res<'w, Performance>,
    graphics: Res<'w, Graphics>,
    session: Res<'w, Session>,
    player: Res<'w, Player>,
    rays: Res<'w, crate::raytrace::RayState>,
    time: Res<'w, Time>,
    fsr: Res<'w, crate::fsr::FsrState>,
}

pub fn setup(
    mut state: ResMut<Reconstruction>,
    caps: Res<crate::capabilities::Capabilities>,
    #[cfg(feature = "nvidia-dlss")] dlss: Option<
        Res<bevy::anti_alias::dlss::DlssSuperResolutionSupported>,
    >,
) {
    #[cfg(feature = "nvidia-dlss")]
    {
        state.dlss = dlss.is_some() && !caps.software;
    }
    state.dlss_reason = if state.dlss {
        "NVIDIA DLSS / DLAA supported by the initialized Vulkan device.".into()
    } else if !cfg!(feature = "nvidia-dlss") {
        "This build does not include the NVIDIA runtime integration.".into()
    } else if caps.backend != "Vulkan" {
        "DLSS / DLAA requires Vulkan. Select Vulkan below, then restart.".into()
    } else {
        "The initialized device or NVIDIA driver does not report DLSS support.".into()
    };
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Applied {
    options: RenderOptions,
    quality: Quality,
    active: bool,
    rays: bool,
    software: bool,
    method: Upscaler,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct StatusKey {
    method: Upscaler,
    request: Upscaler,
    aa: AntiAliasing,
    quality: UpscaleQuality,
    revision: u64,
}
#[derive(Default)]
pub struct History {
    epoch: Option<(u64, u64)>,
    origin: DVec3,
    position: DVec3,
    rotation: Quat,
}
impl History {
    pub fn cut(&mut self, session: &Session, player: &Player) -> bool {
        let reset = self.epoch != Some((session.epoch, session.revision))
            || self.origin != session.origin
            || self.position.distance_squared(player.position) > 96.0 * 96.0
            || self.rotation.dot(player.rotation()).abs() < 0.65;
        self.epoch = Some((session.epoch, session.revision));
        self.origin = session.origin;
        self.position = player.position;
        self.rotation = player.rotation();
        reset
    }
}

#[allow(clippy::too_many_arguments)]
pub fn update(
    mut commands: Commands,
    input: FrameInput,
    mut state: ResMut<Reconstruction>,
    mut cameras: Query<(Entity, &Camera, &mut Projection, &mut ColorGrading), With<MainCamera>>,
    mut taa: Query<&mut TemporalAntiAliasing>,
    #[cfg(feature = "nvidia-dlss")] mut dlss: Query<&mut bevy::anti_alias::dlss::Dlss>,
    mut fsr_cameras: Query<&mut crate::fsr::FsrUpscaler>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut applied: Local<Option<Applied>>,
    mut history: Local<History>,
    mut status_key: Local<Option<StatusKey>>,
) {
    let limits = &input.limits;
    let graphics = &input.graphics;
    let session = &input.session;
    let player = &input.player;
    let rays = &input.rays;
    let time = &input.time;
    let fsr = &input.fsr;
    let options = limits.preferences.render;
    let quality = limits.quality(graphics.quality);
    let method = resolve(options.upscaler, state.dlss, fsr.available());
    let key = StatusKey {
        method,
        request: options.upscaler,
        aa: options.anti_aliasing,
        quality: options.upscale_quality,
        revision: fsr.0.revision.load(std::sync::atomic::Ordering::Acquire),
    };
    if *status_key != Some(key) {
        state.active = method;
        state.status = match method {
            Upscaler::Dlss => format!(
                "DLSS Super Resolution · {}",
                options.upscale_quality.label()
            ),
            Upscaler::Dlaa => "DLAA · one input pixel per output pixel".into(),
            Upscaler::Fsr41 => fsr.description(),
            _ if options.upscaler != Upscaler::Native && options.upscaler != Upscaler::Auto => {
                format!("Native fallback · {}", options.anti_aliasing.label())
            }
            _ => format!("Native rendering · {}", options.anti_aliasing.label()),
        };
        *status_key = Some(key);
    }
    let cut = history.cut(session, player);
    if cut {
        state.history_resets = state.history_resets.saturating_add(1);
        for mut v in &mut taa {
            v.reset = true;
        }
        #[cfg(feature = "nvidia-dlss")]
        for mut v in &mut dlss {
            v.reset = true;
        }
    }
    let pre_exposure = crate::scene::outdoor_exposure(graphics.hour, graphics.exposure).exposure();
    for mut v in &mut fsr_cameras {
        v.reset = cut;
        v.pre_exposure = pre_exposure;
        v.frame_ms = time.delta_secs() * 1000.0;
    }
    for (entity, camera, mut projection, mut grading) in &mut cameras {
        let next = Applied {
            options,
            quality,
            active: camera.is_active,
            rays: rays.active,
            software: limits.software_gpu,
            method,
        };
        if *applied == Some(next) {
            continue;
        }
        let target = &mut commands.entity(entity);
        // These components are owned here, including all their shared prepasses.
        // Rebuild the complete required set once rather than dropping another
        // effect's depth/motion dependencies while switching reconstruction.
        target.remove::<(
            Fxaa,
            Smaa,
            TemporalAntiAliasing,
            TemporalJitter,
            MipBias,
            DepthPrepass,
            MotionVectorPrepass,
            NormalPrepass,
        )>();
        target.remove::<(
            Bloom,
            ScreenSpaceAmbientOcclusion,
            DepthOfField,
            ContrastAdaptiveSharpening,
            crate::fsr::FsrUpscaler,
        )>();
        #[cfg(feature = "nvidia-dlss")]
        target.remove::<bevy::anti_alias::dlss::Dlss>();
        target.insert(Msaa::Off);
        target.insert(match options.tonemap {
            ToneMap::AgX => Tonemapping::AgX,
            ToneMap::TonyMcMapface => Tonemapping::TonyMcMapface,
            ToneMap::Aces => Tonemapping::AcesFitted,
            ToneMap::Reinhard => Tonemapping::Reinhard,
        });
        grading.global.post_saturation = options.saturation;
        grading.shadows.contrast = options.contrast;
        grading.midtones.contrast = options.contrast;
        grading.highlights.contrast = options.contrast;
        if let Projection::Perspective(p) = &mut *projection {
            p.fov = options.field_of_view.to_radians();
        }
        if camera.is_active {
            if rays.active {
                target.insert((DepthPrepass, MotionVectorPrepass));
            }
            match method {
                #[cfg(feature = "nvidia-dlss")]
                Upscaler::Dlss | Upscaler::Dlaa => {
                    use bevy::anti_alias::dlss::{Dlss, DlssPerfQualityMode};
                    let mode = if method == Upscaler::Dlaa {
                        DlssPerfQualityMode::Dlaa
                    } else {
                        match options.upscale_quality {
                            UpscaleQuality::Quality => DlssPerfQualityMode::Quality,
                            UpscaleQuality::Balanced => DlssPerfQualityMode::Balanced,
                            UpscaleQuality::Performance => DlssPerfQualityMode::Performance,
                            UpscaleQuality::UltraPerformance => {
                                DlssPerfQualityMode::UltraPerformance
                            }
                        }
                    };
                    target.insert(Dlss::<bevy::anti_alias::dlss::DlssSuperResolutionFeature> {
                        perf_quality_mode: mode,
                        reset: true,
                        ..default()
                    });
                }
                Upscaler::Fsr41 => {
                    target.insert(crate::fsr::FsrUpscaler {
                        quality: options.upscale_quality,
                        field_of_view: options.field_of_view.to_radians(),
                        reset: true,
                        pre_exposure,
                        frame_ms: time.delta_secs() * 1000.0,
                    });
                }
                _ => match options.anti_aliasing {
                    AntiAliasing::Off => {}
                    AntiAliasing::Fxaa => {
                        target.insert(Fxaa::default());
                    }
                    AntiAliasing::Smaa => {
                        target.insert(Smaa {
                            preset: SmaaPreset::High,
                        });
                    }
                    AntiAliasing::Taa => {
                        target.insert(TemporalAntiAliasing { reset: true });
                    }
                },
            }
            if options.sharpening > 0.0 {
                target.insert(ContrastAdaptiveSharpening {
                    sharpening_strength: options.sharpening,
                    ..default()
                });
            }
            if options.bloom && quality != Quality::Low {
                target.insert(Bloom {
                    intensity: options.bloom_strength,
                    ..Bloom::NATURAL
                });
            }
            let ao = match options.ambient_occlusion {
                EffectQuality::Off => None,
                EffectQuality::Automatic if quality != Quality::High => None,
                EffectQuality::Low => Some(ScreenSpaceAmbientOcclusionQualityLevel::Low),
                EffectQuality::Medium => Some(ScreenSpaceAmbientOcclusionQualityLevel::Medium),
                _ => Some(ScreenSpaceAmbientOcclusionQualityLevel::High),
            };
            if !rays.active && quality != Quality::Low {
                if let Some(level) = ao {
                    target.insert(ScreenSpaceAmbientOcclusion {
                        quality_level: level,
                        radius: options.ao_radius,
                        ..default()
                    });
                }
            }
            if options.depth_of_field && quality != Quality::Low {
                target.insert((
                    DepthPrepass,
                    DepthOfField {
                        mode: DepthOfFieldMode::Gaussian,
                        focal_distance: options.focus_distance,
                        aperture_f_stops: options.aperture,
                        max_circle_of_confusion_diameter: 16.0,
                        ..default()
                    },
                ));
            }
        }
        *applied = Some(next);
    }
    // The window is a full-resolution UI compositor; V-sync never changes the
    // adaptive/explicit frame cap or world-camera resolution.
    for mut w in &mut windows {
        let present = if options.vsync {
            PresentMode::AutoVsync
        } else {
            PresentMode::AutoNoVsync
        };
        if w.present_mode != present {
            w.present_mode = present;
        }
    }
}

pub fn controls(
    p: &mut bevy_egui::egui::Ui,
    options: &mut RenderOptions,
    state: &Reconstruction,
    fsr: &crate::fsr::FsrState,
    caps: &crate::capabilities::Capabilities,
    target: &crate::scene::SceneTarget,
    surfaces: &crate::terrain_texture::ActiveSurfaces,
) {
    use bevy_egui::egui;
    p.label(&caps.gpu);
    p.small(format!(
        "{} · output {} × {}",
        caps.backend, target.size.x, target.size.y
    ));
    p.label(&state.status);
    p.collapsing("Image reconstruction and anti-aliasing",|p|{
        egui::ComboBox::from_id_salt("upscaler").selected_text(options.upscaler.label()).show_ui(p,|p|{
            for mode in Upscaler::ALL {
                let supported=match mode {Upscaler::Dlss|Upscaler::Dlaa=>state.dlss,Upscaler::Fsr41=>fsr.available(),_=>true};
                p.add_enabled_ui(supported,|p|{p.selectable_value(&mut options.upscaler,*mode,mode.label());});
            }
        });
        p.small(&state.dlss_reason);
        p.small(fsr.description());
        if matches!(state.active,Upscaler::Dlss|Upscaler::Fsr41) {
            egui::ComboBox::from_id_salt("upscaler_quality").selected_text(options.upscale_quality.label()).show_ui(p,|p|{
                for q in UpscaleQuality::ALL {p.selectable_value(&mut options.upscale_quality,*q,q.label());}
            });
        }
        if state.active==Upscaler::Native {
            egui::ComboBox::from_id_salt("native_aa").selected_text(options.anti_aliasing.label()).show_ui(p,|p|{
                for aa in AntiAliasing::ALL {p.selectable_value(&mut options.anti_aliasing,*aa,aa.label());}
            });
        }
        p.add(egui::Slider::new(&mut options.resolution_scale,0.5..=1.0).text("Output resolution scale"));
        p.small("Multiplies the Performance budget, with a 50% floor. DLAA uses the current output resolution; choose 100% and a manual budget for native window pixels.");
        p.add(egui::Slider::new(&mut options.sharpening,0.0..=1.0).text("Adaptive sharpening"));
        p.small("DLSS, DLAA and FSR own temporal reconstruction. FXAA/SMAA/TAA are fallback choices, never stacked on top.");
    });
    p.collapsing("Lighting and shadows",|p|{
        quality_combo(p,"shadow_quality","Shadow maps",&mut options.shadows);
        p.add(egui::Slider::new(&mut options.shadow_distance,64.0..=768.0).text("Shadow distance (m)"));
        quality_combo(p,"ao_quality","Ambient occlusion",&mut options.ambient_occlusion);
        p.add(egui::Slider::new(&mut options.ao_radius,0.1..=2.0).text("AO radius (m)"));
        p.checkbox(&mut options.bloom,"Bloom");
        p.add_enabled(options.bloom,egui::Slider::new(&mut options.bloom_strength,0.0..=0.3).text("Bloom intensity"));
        p.small("Low graphics omits expensive lens/AO effects. Software shadow maps remain capped at 1024 pixels.");
    });
    p.collapsing("Terrain materials", |p| {
        p.radio_value(
            &mut options.procedural_surfaces,
            true,
            "Procedural litter, stone and turf",
        );
        p.radio_value(
            &mut options.procedural_surfaces,
            false,
            "Scanned surfaces (Poly Haven)",
        );
        if surfaces.1>0 {
            let storage=if surfaces.2 {"GPU block compression"} else {"compatibility format"};
            p.small(format!("Active maps: {} × {} · {storage}",surfaces.1,surfaces.1));
        } else {p.small("Active maps: 512 × 512");}
        p.add_enabled_ui(options.procedural_surfaces, |p| {
            egui::ComboBox::new("surface_resolution", "Texture size")
                .selected_text(options.surface_resolution.label())
                .show_ui(p, |p| {
                    for v in SurfaceResolution::ALL {
                        p.selectable_value(&mut options.surface_resolution, v, v.label());
                    }
                });
            p.horizontal_wrapped(|p| {
                p.label("Parallax occlusion");
                for (v, name) in [(SurfaceRelief::Off, "Off"), (SurfaceRelief::Balanced, "Balanced"), (SurfaceRelief::Ultra, "Ultra")] {
                    p.selectable_value(&mut options.surface_relief, v, name);
                }
            });
            p.small("4K uses 96 MiB of compressed detail maps. Low graphics and Eco use the base textures.");
        });
    });
    p.collapsing("Color and camera", |p| {
        egui::ComboBox::from_id_salt("tone_map")
            .selected_text(options.tonemap.label())
            .show_ui(p, |p| {
                for v in ToneMap::ALL {
                    p.selectable_value(&mut options.tonemap, *v, v.label());
                }
            });
        p.add(egui::Slider::new(&mut options.saturation, 0.0..=1.5).text("Saturation"));
        p.add(egui::Slider::new(&mut options.contrast, 0.5..=1.5).text("Contrast"));
        p.add(
            egui::Slider::new(&mut options.field_of_view, 45.0..=110.0)
                .text("Vertical field of view"),
        );
        p.checkbox(&mut options.depth_of_field, "Cinematic depth of field");
        p.add_enabled(
            options.depth_of_field,
            egui::Slider::new(&mut options.focus_distance, 0.5..=10000.0)
                .logarithmic(true)
                .text("Focus distance (m)"),
        );
        p.add_enabled(
            options.depth_of_field,
            egui::Slider::new(&mut options.aperture, 1.4..=22.0).text("Lens f-stop"),
        );
    });
    p.collapsing("Atmosphere quality",|p|{
        p.add(egui::Slider::new(&mut options.cloud_steps,16..=48).text("Cloud march steps"));
        p.add(egui::Slider::new(&mut options.cloud_coverage,0.0..=1.0).text("Cloud coverage"));
        p.add(egui::Slider::new(&mut options.fog_steps,12..=48).text("Light-shaft steps"));
        p.small("Clouds and shafts require High graphics and their environment toggles. More samples cost more GPU time.");
    });
    p.collapsing("Display",|p|{
        p.checkbox(&mut options.vsync,"Vertical synchronization");
        egui::ComboBox::from_id_salt("backend").selected_text(options.backend.label()).show_ui(p,|p|{
            for v in Backend::ALL {
                p.add_enabled_ui(*v!=Backend::Dx12 || cfg!(windows),|p|{p.selectable_value(&mut options.backend,*v,v.label());});
            }
        });
        p.small("Graphics API changes take effect after restarting WorldCraft. The active API is shown above.");
    });
    if p.button("Reset render settings").clicked() {
        *options = RenderOptions::default();
    }
    options.sanitize();
}
fn quality_combo(p: &mut bevy_egui::egui::Ui, id: &str, label: &str, value: &mut EffectQuality) {
    p.label(label);
    bevy_egui::egui::ComboBox::from_id_salt(id)
        .selected_text(value.label())
        .show_ui(p, |p| {
            for q in EffectQuality::ALL {
                p.selectable_value(value, *q, q.label());
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn switching_aa_keeps_required_prepasses_and_never_stacks_fallbacks() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(Performance::load(true))
            .init_resource::<Graphics>()
            .init_resource::<Session>()
            .init_resource::<Player>()
            .init_resource::<crate::raytrace::RayState>()
            .init_resource::<crate::fsr::FsrState>()
            .init_resource::<Reconstruction>()
            .add_systems(Update, update);
        let camera = app
            .world_mut()
            .spawn((
                MainCamera,
                Camera::default(),
                Projection::Perspective(default()),
                ColorGrading::default(),
            ))
            .id();
        for aa in [
            AntiAliasing::Taa,
            AntiAliasing::Smaa,
            AntiAliasing::Fxaa,
            AntiAliasing::Off,
        ] {
            app.world_mut()
                .resource_mut::<Performance>()
                .preferences
                .render
                .anti_aliasing = aa;
            app.update();
            let e = app.world().entity(camera);
            assert_eq!(
                e.contains::<TemporalAntiAliasing>(),
                aa == AntiAliasing::Taa
            );
            assert_eq!(e.contains::<Smaa>(), aa == AntiAliasing::Smaa);
            assert_eq!(e.contains::<Fxaa>(), aa == AntiAliasing::Fxaa);
            assert_eq!(e.contains::<MotionVectorPrepass>(), aa == AntiAliasing::Taa);
        }
        app.world_mut()
            .resource_mut::<Performance>()
            .preferences
            .render
            .depth_of_field = true;
        app.update();
        assert!(app.world().entity(camera).contains::<DepthPrepass>());
        app.world_mut()
            .resource_mut::<crate::raytrace::RayState>()
            .active = true;
        app.update();
        assert!(app.world().entity(camera).contains::<MotionVectorPrepass>());
        // A saved unsupported request must still have one working fallback.
        app.world_mut()
            .resource_mut::<Performance>()
            .preferences
            .render
            .upscaler = Upscaler::Dlaa;
        app.world_mut()
            .resource_mut::<Performance>()
            .preferences
            .render
            .anti_aliasing = AntiAliasing::Smaa;
        app.update();
        assert_eq!(
            app.world().resource::<Reconstruction>().active,
            Upscaler::Native
        );
        assert!(app.world().entity(camera).contains::<Smaa>());
        assert!(!app
            .world()
            .entity(camera)
            .contains::<TemporalAntiAliasing>());
    }
    #[test]
    fn history_resets_for_world_edits_rebases_teleports_and_camera_cuts() {
        let mut h = History::default();
        let mut s = Session::default();
        let mut p = Player::default();
        assert!(h.cut(&s, &p));
        assert!(!h.cut(&s, &p));
        p.position.x += 2.0;
        assert!(!h.cut(&s, &p));
        p.position.x += 1000.0;
        assert!(h.cut(&s, &p));
        s.origin.x += 256.0;
        assert!(h.cut(&s, &p));
        assert!(!h.cut(&s, &p));
        s.epoch += 1;
        assert!(h.cut(&s, &p));
        s.revision += 1;
        assert!(h.cut(&s, &p));
        p.yaw += std::f32::consts::PI;
        assert!(h.cut(&s, &p));
    }
}
