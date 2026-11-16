//! Hosted software-rendered integration check. This is not a hardware benchmark.
use crate::{
    session::*,
    storage::{self, Disk},
    streaming::Stream,
};
#[cfg(not(target_os = "linux"))]
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy::{app::AppExit, math::DVec3, prelude::*};
use worldcraft_core::*;

#[derive(Resource, Default)]
pub struct Smoke {
    frames: u32,
    stage: u8,
    capture_frame: u32,
    positioned: bool,
}
pub fn start(mut disk: ResMut<Disk>, mut ui: ResMut<UiState>, mut graphics: ResMut<Graphics>) {
    storage::create(
        &mut disk,
        "Cloud smoke test".into(),
        GeneratorSettings::for_preset(Preset::Temperate, 20261116),
    );
    ui.home = true;
    ui.inspector = false;
    graphics.radius = 2;
    graphics.quality = Quality::Low;
}
pub fn check(
    mut commands: Commands,
    mut state: ResMut<Smoke>,
    mut session: ResMut<Session>,
    mut player: ResMut<Player>,
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
    if !state.positioned {
        if session.world.is_none() {
            return;
        }
        player.position += DVec3::new(0.0, 45.0, 70.0);
        player.position.y = player.position.y.max(
            session
                .generator
                .as_ref()
                .unwrap()
                .height(player.position.x, player.position.z)
                + 15.0,
        );
        player.pitch = -0.35;
        session.mode = Mode::Fly;
        state.positioned = true;
        info!("SMOKE: asynchronous world creation and safe spawn passed");
    }
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
        // Rebase every render-space object while keeping the world camera fixed.
        session.origin = DVec3::new(player.position.x, 0.0, player.position.z + 128.0);
        state.stage = 13;
        state.capture_frame = state.frames;
        info!("SMOKE: testing world-anchored detail under a floating-origin shift");
    }
    if state.stage == 13 && state.frames > state.capture_frame + 45 {
        capture(&mut commands, "worldcraft-rebased.png");
        state.stage = 14;
        state.capture_frame = state.frames;
    }
    if state.stage == 14
        && state.frames > state.capture_frame + 20
        && std::path::Path::new("worldcraft-rebased.png").exists()
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
        session.create(
            "Alpine verification".into(),
            GeneratorSettings::for_preset(Preset::Alpine, 20261116),
            &mut player,
        );
        player.position += DVec3::new(0.0, 65.0, 120.0);
        player.position.y = player.position.y.max(
            session
                .generator
                .as_ref()
                .unwrap()
                .height(player.position.x, player.position.z)
                + 15.0,
        );
        player.pitch = -0.28;
        session.mode = Mode::Fly;
        ui.home = false;
        ui.inspector = false;
        graphics.quality = Quality::Balanced;
        graphics.radius = 3;
        state.stage = 9;
        state.capture_frame = state.frames;
        info!("SMOKE: switched worlds to Alpine, rejecting previous epoch jobs");
    }
    if state.stage == 9
        && stream.loaded.len() == stream.desired
        && stream.queue() == 0
        && state.frames > state.capture_frame + 60
    {
        capture(&mut commands, "worldcraft-alpine.png");
        state.stage = 10;
        state.capture_frame = state.frames;
    }
    if state.stage == 10
        && state.frames > state.capture_frame + 20
        && std::path::Path::new("worldcraft-alpine.png").exists()
    {
        let seed = 20261116;
        let settings = GeneratorSettings::for_preset(Preset::Islands, seed);
        session.create("Coast verification".into(), settings.clone(), &mut player);
        let cx =
            (0.15 + generation::unit(seed ^ 151, 0, 0) * 0.7) * 4200.0 * settings.terrain_scale;
        let cz =
            (0.15 + generation::unit(seed ^ 163, 0, 0) * 0.7) * 4200.0 * settings.terrain_scale;
        let radius = (1250.0 + generation::unit(seed ^ 179, 0, 0) * 850.0) * settings.terrain_scale;
        let x = cx;
        let z = cz + radius * 0.86;
        let h = session
            .generator
            .as_ref()
            .unwrap()
            .height(x, z)
            .max(settings.sea_level);
        player.position = DVec3::new(x, h + 80.0, z);
        player.yaw = 0.0;
        player.pitch = -0.22;
        session.mode = Mode::Fly;
        graphics.radius = 4;
        state.stage = 11;
        state.capture_frame = state.frames;
        info!("SMOKE: switched worlds to coast for water shader verification");
    }
    if state.stage == 11
        && stream.loaded.len() == stream.desired
        && stream.queue() == 0
        && state.frames > state.capture_frame + 60
    {
        capture(&mut commands, "worldcraft-coast.png");
        state.stage = 12;
        state.capture_frame = state.frames;
    }
    if state.stage == 12
        && state.frames > state.capture_frame + 20
        && std::path::Path::new("worldcraft-coast.png").exists()
    {
        graphics.map = true;
        graphics.quality = Quality::Low;
        ui.map_zoom = 8000.0;
        state.stage = 15;
        state.capture_frame = state.frames;
        info!("SMOKE: opening an 8 km regional preview");
    }
    if state.stage == 15 && state.frames > state.capture_frame + 60 {
        capture(&mut commands, "worldcraft-map.png");
        state.stage = 16;
        state.capture_frame = state.frames;
    }
    if state.stage == 16
        && state.frames > state.capture_frame + 20
        && std::path::Path::new("worldcraft-map.png").exists()
    {
        info!("SMOKE: quality modes, home, Alpine, coast, map and epoch changes verified; exiting");
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
