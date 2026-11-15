//! Hosted software-rendered integration check. This is not a hardware benchmark.
use crate::{session::*, streaming::Stream};
#[cfg(not(target_os = "linux"))]
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy::{app::AppExit, math::DVec3, prelude::*};
use worldcraft_core::*;

#[derive(Resource, Default)]
pub struct Smoke {
    frames: u32,
    stage: u8,
    capture_frame: u32,
}
pub fn start(
    mut session: ResMut<Session>,
    mut player: ResMut<Player>,
    mut ui: ResMut<UiState>,
    mut graphics: ResMut<Graphics>,
) {
    session.create(
        "Cloud smoke test".into(),
        GeneratorSettings::for_preset(Preset::Temperate, 20261116),
        &mut player,
    );
    player.position += DVec3::new(0.0, 45.0, 70.0);
    player.pitch = -0.35;
    session.mode = Mode::Fly;
    ui.home = false;
    ui.inspector = false;
    graphics.radius = 2;
    graphics.quality = Quality::Low;
}
pub fn check(
    mut commands: Commands,
    mut state: ResMut<Smoke>,
    mut session: ResMut<Session>,
    player: Res<Player>,
    stream: Res<Stream>,
    mut ui: ResMut<UiState>,
    mut graphics: ResMut<Graphics>,
    mut exit: EventWriter<AppExit>,
) {
    state.frames += 1;
    assert!(
        state.frames < 5000,
        "Smoke test timed out before terrain finished streaming"
    );
    if state.stage == 0 && stream.loaded.len() >= 9 && state.frames > 100 {
        let x = player.position.x;
        let z = player.position.z;
        let before = session.generator.as_ref().unwrap().height(x, z);
        session
            .history
            .push(vec![Stroke {
                center: [x, z],
                brush: Brush::default(),
            }])
            .unwrap();
        session.rebuild(&[]);
        assert!((session.generator.as_ref().unwrap().height(x, z) - before - 5.0).abs() < 1e-6);
        assert!(session.history.undo());
        session.rebuild(&[]);
        assert!((session.generator.as_ref().unwrap().height(x, z) - before).abs() < 1e-6);
        assert!(session.history.redo());
        session.rebuild(&[]);
        let world = session.snapshot(&player).unwrap();
        let path = std::env::temp_dir().join("worldcraft-smoke.worldcraft.json");
        save_world(&path, &world).unwrap();
        assert_eq!(load_world(&path).unwrap(), world);
        state.stage = 1;
        info!("SMOKE: streaming, brush, undo/redo and save/load passed");
    }
    if state.stage == 1
        && stream.loaded.len() == stream.desired
        && stream.queue() == 0
        && stream
            .loaded
            .iter()
            .all(|(coord, l)| l.key.revision == *session.chunk_revisions.get(coord).unwrap_or(&0))
    {
        capture(&mut commands, "worldcraft-smoke.png");
        state.stage = 2;
        state.capture_frame = state.frames;
    }
    if state.stage == 2
        && state.frames > state.capture_frame + 20
        && std::path::Path::new("worldcraft-smoke.png").exists()
    {
        graphics.quality = Quality::Balanced;
        state.stage = 3;
        state.capture_frame = state.frames;
        info!("SMOKE: switched to Balanced atmosphere");
    }
    if state.stage == 3 && state.frames > state.capture_frame + 60 {
        capture(&mut commands, "worldcraft-balanced.png");
        state.stage = 4;
        state.capture_frame = state.frames;
    }
    if state.stage == 4
        && state.frames > state.capture_frame + 20
        && std::path::Path::new("worldcraft-balanced.png").exists()
    {
        graphics.quality = Quality::High;
        state.stage = 5;
        state.capture_frame = state.frames;
        info!("SMOKE: switched to High ambient occlusion");
    }
    if state.stage == 5 && state.frames > state.capture_frame + 60 {
        capture(&mut commands, "worldcraft-high.png");
        state.stage = 6;
        state.capture_frame = state.frames;
    }
    if state.stage == 6
        && state.frames > state.capture_frame + 20
        && std::path::Path::new("worldcraft-high.png").exists()
    {
        ui.home = true;
        ui.create = false;
        session.dirty = false;
        graphics.quality = Quality::Low;
        state.stage = 7;
        state.capture_frame = state.frames;
    }
    if state.stage == 7 && state.frames > state.capture_frame + 30 {
        capture(&mut commands, "worldcraft-home.png");
        state.stage = 8;
        state.capture_frame = state.frames;
    }
    if state.stage == 8
        && state.frames > state.capture_frame + 20
        && std::path::Path::new("worldcraft-home.png").exists()
    {
        info!("SMOKE: home and Low/Balanced/High captured; native application exited cleanly");
        exit.write(AppExit::Success);
    }
}

/// Egui 0.34 renders to the window swapchain; Bevy's redirected screenshot texture
/// omits that overlay. Hosted Linux QA captures the real X11 window instead.
fn capture(commands: &mut Commands, path: &str) {
    #[cfg(target_os = "linux")]
    {
        let _ = commands;
        let raw = std::path::Path::new(path).with_extension("xwd");
        let captured = std::process::Command::new("xwd")
            .args([
                "-silent",
                "-name",
                "Worldcraft — Explore your world",
                "-out",
            ])
            .arg(&raw)
            .status()
            .expect("Cloud QA requires x11-apps/xwd");
        assert!(captured.success(), "Native window capture failed");
        let converted = std::process::Command::new("convert")
            .arg(&raw)
            .arg(path)
            .status()
            .expect("Cloud QA requires ImageMagick");
        assert!(converted.success(), "Native capture conversion failed");
        let _ = std::fs::remove_file(raw);
    }
    #[cfg(not(target_os = "linux"))]
    {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path.to_owned()));
    }
}
