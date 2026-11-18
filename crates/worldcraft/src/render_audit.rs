//! Native reconstruction/lens regression route, with honest unsupported-device skips.
use crate::{render_options::*, session::*, streaming::Stream};
use bevy::{
    app::AppExit,
    math::DVec3,
    prelude::*,
    render::view::screenshot::{save_to_disk, Screenshot},
};
use std::sync::atomic::Ordering;
use worldcraft_core::{GeneratorSettings, Preset};

pub const LABELS: [&str; 9] = [
    "fxaa",
    "smaa",
    "taa",
    "cinematic",
    "lens",
    "scaled",
    "dlss",
    "dlaa",
    "fsr",
];
#[derive(Resource, Default)]
pub struct Audit {
    stage: usize,
    initialized: bool,
    frames: u32,
    ready: u32,
    capture: Option<u32>,
    rows: Vec<serde_json::Value>,
    fsr_failures: u64,
}
pub fn start(
    mut session: ResMut<Session>,
    mut player: ResMut<Player>,
    mut ui: ResMut<UiState>,
    mut graphics: ResMut<Graphics>,
) {
    session.create(
        "Renderer integration audit".into(),
        GeneratorSettings::for_preset(Preset::Temperate, 42),
        &mut player,
    );
    player.position = DVec3::new(-1750.0, 0.0, -1821.0);
    player.position.y = session
        .generator
        .as_ref()
        .unwrap()
        .height(player.position.x, player.position.z)
        + 1.8;
    player.yaw = 0.39479113;
    player.pitch = 0.09;
    player.captured = false;
    session.mode = Mode::Fly;
    ui.home = false;
    ui.photo = true;
    ui.inspector = false;
    graphics.radius = 2;
    graphics.hour = 10.0;
    graphics.breeze = 0.0;
    graphics.cycle = false;
    graphics.volumetrics = false;
}
#[allow(clippy::too_many_arguments)]
pub fn check(
    mut commands: Commands,
    mut audit: ResMut<Audit>,
    mut limits: ResMut<crate::performance::Performance>,
    mut graphics: ResMut<Graphics>,
    stream: Res<Stream>,
    state: Res<crate::render_quality::Reconstruction>,
    fsr: Res<crate::fsr::FsrState>,
    target: Res<crate::scene::SceneTarget>,
    mut exit: MessageWriter<AppExit>,
    cameras: Query<Entity, With<crate::scene::MainCamera>>,
    fxaa: Query<(), With<bevy::anti_alias::fxaa::Fxaa>>,
    smaa: Query<(), With<bevy::anti_alias::smaa::Smaa>>,
    taa: Query<(), With<bevy::anti_alias::taa::TemporalAntiAliasing>>,
) {
    audit.frames += 1;
    assert!(audit.frames < 1500, "Renderer route exceeded frame budget");
    let label = LABELS[audit.stage];
    if let Some(frame) = audit.capture {
        if audit.frames <= frame + 5
            || !std::path::Path::new(&format!("worldcraft-render-{label}.png")).is_file()
        {
            return;
        }
        audit.capture = None;
        audit.stage += 1;
        audit.ready = 0;
        audit.initialized = false;
        if audit.stage == LABELS.len() {
            assert_eq!(
                fsr.0.failures.load(Ordering::Relaxed),
                audit.fsr_failures,
                "A declared-supported FSR path failed"
            );
            let report = serde_json::json!({"status":"passed","source":std::env::var("GITHUB_SHA").unwrap_or("local-uncommitted".into()),
                "scope":"Native shader/state/image integration; unsupported vendor paths are explicit skips, not hardware certification",
                "version":env!("CARGO_PKG_VERSION"),"dlss_supported":state.dlss,"fsr_available":fsr.available(),
                "fsr_recorded_dispatches":fsr.0.dispatches.load(Ordering::Relaxed),"history_resets":state.history_resets,"cases":audit.rows});
            std::fs::write(
                "renderer-settings-audit.json",
                serde_json::to_vec_pretty(&report).unwrap(),
            )
            .unwrap();
            info!("RENDER AUDIT: native AA, lens/color controls, scaled output and capability fallbacks passed");
            exit.write(AppExit::Success);
            return;
        }
        return;
    }
    if !audit.initialized {
        let mut o = RenderOptions {
            upscaler: Upscaler::Native,
            ..Default::default()
        };
        graphics.quality = if (3..=5).contains(&audit.stage) {
            Quality::High
        } else {
            Quality::Balanced
        };
        match audit.stage {
            1 => o.anti_aliasing = AntiAliasing::Smaa,
            2 => o.anti_aliasing = AntiAliasing::Taa,
            3 => {
                o.anti_aliasing = AntiAliasing::Taa;
                o.sharpening = 0.25;
                o.cloud_steps = 48;
            }
            4 => {
                o.anti_aliasing = AntiAliasing::Taa;
                o.depth_of_field = true;
                o.focus_distance = 12.0;
                o.aperture = 2.8;
            }
            5 => {
                o.anti_aliasing = AntiAliasing::Taa;
                o.resolution_scale = 0.67;
                o.sharpening = 0.25;
            }
            6 => o.upscaler = Upscaler::Dlss,
            7 => o.upscaler = Upscaler::Dlaa,
            8 => o.upscaler = Upscaler::Fsr41,
            _ => {}
        }
        if audit.stage == 0 {
            audit.fsr_failures = fsr.0.failures.load(Ordering::Relaxed);
        }
        limits.preferences.render = o;
        audit.initialized = true;
        audit.ready = 0;
        return;
    }
    if stream.loaded.len() != stream.desired || stream.queue() != 0 {
        return;
    }
    audit.ready += 1;
    if audit.ready < 24 {
        return;
    }
    let entity = cameras.single().expect("One audited world view");
    let owned_aa = usize::from(fxaa.contains(entity))
        + usize::from(smaa.contains(entity))
        + usize::from(taa.contains(entity));
    let expected = resolve(
        limits.preferences.render.upscaler,
        state.dlss,
        fsr.available(),
    );
    assert_eq!(state.active, expected);
    if expected == Upscaler::Native {
        assert_eq!(
            owned_aa, 1,
            "Exactly one fallback AA effect must own the view"
        );
    } else {
        assert_eq!(
            owned_aa, 0,
            "Native AA must not stack with vendor reconstruction"
        );
    }
    match audit.stage {
        1 => assert!(smaa.contains(entity)),
        2..=5 => assert!(taa.contains(entity)),
        _ => {}
    }
    if audit.stage == 5 {
        assert!(limits.render_scale() < 1.0);
    }
    let vendor_execution_skipped = matches!(audit.stage, 6..=8) && expected == Upscaler::Native;
    audit.rows.push(serde_json::json!({"label":label,"requested":format!("{:?}",limits.preferences.render.upscaler),
        "active":format!("{:?}",state.active),"status":state.status,"output":[target.size.x,target.size.y],
        "fallback_aa_components":owned_aa,"resident_chunks":stream.loaded.len(),"queued_jobs":stream.queue(),
        "vendor_execution_skipped":vendor_execution_skipped}));
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(format!("worldcraft-render-{label}.png")));
    audit.capture = Some(audit.frames);
}
