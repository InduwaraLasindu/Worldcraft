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
    detail_target: Option<(ChunkCoord, ChunkKey, bevy::asset::AssetId<Mesh>)>,
    original_position: DVec3,
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
    mut exit: MessageWriter<AppExit>,
    foliage: Query<(
        &crate::foliage::Foliage,
        &Transform,
        &bevy::camera::visibility::VisibilityRange,
        &ChildOf,
        Has<crate::foliage::NoRayProxy>,
    )>,
    shadow_observer: Query<Has<bevy::camera::ShadowLodOrigin>, With<crate::scene::MainCamera>>,
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
        let center = ChunkCoord::at(player.position.x, player.position.z);
        let coord = ChunkCoord {
            x: center.x + 2,
            z: center.z,
        };
        let loaded = stream
            .loaded
            .get(&coord)
            .expect("Initial presentation regression tile");
        assert_eq!(loaded.key.lod, 0);
        assert_eq!(loaded.detail, 1);
        state.detail_target = Some((coord, loaded.key, loaded.terrain_mesh.id()));
        state.original_position = player.position;
        player.position.x += CHUNK_SIZE;
        state.stage = 17;
        state.capture_frame = state.frames;
    }
    if state.stage == 17
        && state.frames > state.capture_frame + 2
        && stream.loaded.len() == stream.desired
        && stream.queue() == 0
    {
        let (coord, key, mesh) = state.detail_target.unwrap();
        let loaded = stream.loaded.get(&coord).unwrap();
        assert_eq!(
            loaded.key, key,
            "Terrain topology should not change during this foliage refinement"
        );
        assert_eq!(
            loaded.terrain_mesh.id(),
            mesh,
            "Foliage-only refresh reuploaded unchanged terrain"
        );
        assert_eq!(
            loaded.detail, 0,
            "Foliage kept its distant detail after approaching"
        );
        player.position = state.original_position;
        state.stage = 18;
        state.capture_frame = state.frames;
    }
    if state.stage == 18
        && state.frames > state.capture_frame + 2
        && stream.loaded.len() == stream.desired
        && stream.queue() == 0
    {
        let (coord, key, mesh) = state.detail_target.unwrap();
        let loaded = stream.loaded.get(&coord).unwrap();
        assert_eq!(loaded.key, key);
        assert_eq!(loaded.detail, 1);
        assert_eq!(
            loaded.terrain_mesh.id(),
            mesh,
            "Returning foliage band reuploaded terrain"
        );
        info!("SMOKE: vegetation refined and coarsened independently of terrain topology");
        audit_foliage(&foliage);
        assert!(shadow_observer.single().unwrap());
        capture(&mut commands, "worldcraft-smoke.png");
        state.stage = 2;
        state.capture_frame = state.frames;
    }
    if state.stage == 2
        && state.frames > state.capture_frame + settle_frames(20)
        && std::path::Path::new("worldcraft-smoke.png").exists()
    {
        graphics.quality = Quality::Balanced;
        state.stage = 3;
        state.capture_frame = state.frames;
        info!("SMOKE: switched to Balanced atmosphere");
    }
    if state.stage == 3 && state.frames > state.capture_frame + settle_frames(60) {
        capture(&mut commands, "worldcraft-balanced.png");
        state.stage = 4;
        state.capture_frame = state.frames;
    }
    if state.stage == 4
        && state.frames > state.capture_frame + settle_frames(20)
        && std::path::Path::new("worldcraft-balanced.png").exists()
    {
        // Rebase every render-space object while keeping the world camera fixed.
        session.origin = DVec3::new(player.position.x, 0.0, player.position.z + 128.0);
        state.stage = 13;
        state.capture_frame = state.frames;
        info!("SMOKE: testing world-anchored detail under a floating-origin shift");
    }
    if state.stage == 13 && state.frames > state.capture_frame + settle_frames(45) {
        capture(&mut commands, "worldcraft-rebased.png");
        state.stage = 14;
        state.capture_frame = state.frames;
    }
    if state.stage == 14
        && state.frames > state.capture_frame + settle_frames(20)
        && std::path::Path::new("worldcraft-rebased.png").exists()
    {
        graphics.quality = Quality::High;
        state.stage = 5;
        state.capture_frame = state.frames;
        info!("SMOKE: switched to High ambient occlusion");
    }
    if state.stage == 5 && state.frames > state.capture_frame + settle_frames(60) {
        capture(&mut commands, "worldcraft-high.png");
        state.stage = 6;
        state.capture_frame = state.frames;
    }
    if state.stage == 6
        && state.frames > state.capture_frame + settle_frames(20)
        && std::path::Path::new("worldcraft-high.png").exists()
    {
        ui.home = true;
        ui.create = false;
        session.dirty = false;
        graphics.quality = Quality::Low;
        state.stage = 7;
        state.capture_frame = state.frames;
    }
    if state.stage == 7 && state.frames > state.capture_frame + settle_frames(30) {
        capture(&mut commands, "worldcraft-home.png");
        state.stage = 8;
        state.capture_frame = state.frames;
    }
    if state.stage == 8
        && state.frames > state.capture_frame + settle_frames(20)
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
        && state.frames > state.capture_frame + settle_frames(60)
    {
        capture(&mut commands, "worldcraft-alpine.png");
        state.stage = 10;
        state.capture_frame = state.frames;
    }
    if state.stage == 10
        && state.frames > state.capture_frame + settle_frames(20)
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
        && state.frames > state.capture_frame + settle_frames(60)
    {
        capture(&mut commands, "worldcraft-coast.png");
        state.stage = 12;
        state.capture_frame = state.frames;
    }
    if state.stage == 12
        && state.frames > state.capture_frame + settle_frames(20)
        && std::path::Path::new("worldcraft-coast.png").exists()
    {
        graphics.map = true;
        graphics.quality = Quality::Low;
        ui.map_zoom = 8000.0;
        state.stage = 15;
        state.capture_frame = state.frames;
        info!("SMOKE: opening an 8 km regional preview");
    }
    if state.stage == 15 && state.frames > state.capture_frame + settle_frames(60) {
        capture(&mut commands, "worldcraft-map.png");
        state.stage = 16;
        state.capture_frame = state.frames;
    }
    if state.stage == 16
        && state.frames > state.capture_frame + settle_frames(20)
        && std::path::Path::new("worldcraft-map.png").exists()
    {
        info!("SMOKE: quality modes, home, Alpine, coast, map and epoch changes verified");
        #[cfg(target_os = "linux")]
        {
            let mut child = std::process::Command::new(std::env::current_exe().unwrap())
                .arg("--ray-smoke-test")
                .spawn()
                .expect("Start separate native ray audit");
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(180);
            loop {
                if let Some(result) = child.try_wait().unwrap() {
                    assert!(result.success(), "Native ray-query audit failed");
                    let mut learned = std::process::Command::new("xvfb-run")
                        .arg("-a")
                        .arg(std::env::current_exe().unwrap())
                        .arg("--learned-smoke-test")
                        .spawn()
                        .expect("Start learned-world integration audit");
                    let until = std::time::Instant::now() + std::time::Duration::from_secs(150);
                    loop {
                        if let Some(result) = learned.try_wait().unwrap() {
                            assert!(result.success(), "Learned-world integration audit failed");
                            break;
                        }
                        if std::time::Instant::now() > until {
                            let _ = learned.kill();
                            let _ = learned.wait();
                            panic!("Learned-world integration audit exceeded its bounded timeout");
                        }
                        std::thread::sleep(std::time::Duration::from_millis(200));
                    }
                    break;
                }
                if std::time::Instant::now() > deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    panic!("Native ray-query audit exceeded its bounded timeout");
                }
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
        }
        #[cfg(target_os = "linux")]
        if std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true")
            && std::path::Path::new("detail-example.worldcraft").is_file()
        {
            let result = std::process::Command::new("timeout")
                .args(["180s", "xvfb-run", "-a"])
                .arg(std::env::current_exe().unwrap())
                .arg("--detailed-smoke-test")
                .status()
                .expect("Start detailed-world render audit");
            assert!(result.success(), "Detailed-world render audit failed");
            let result = std::process::Command::new("python3")
                .arg("tools/ci/publish_detail_qa.py")
                .status()
                .expect("Publish detailed-world evidence");
            assert!(
                result.success(),
                "Detailed-world evidence publication failed"
            );
        }
        exit.write(AppExit::Success);
    }
}

fn audit_foliage(
    foliage: &Query<(
        &crate::foliage::Foliage,
        &Transform,
        &bevy::camera::visibility::VisibilityRange,
        &ChildOf,
        Has<crate::foliage::NoRayProxy>,
    )>,
) {
    use crate::foliage::Foliage;
    let key = |parent: &ChildOf, transform: &Transform| {
        (
            parent.parent(),
            transform.translation.to_array().map(f32::to_bits),
        )
    };
    let mut near = std::collections::HashMap::new();
    let mut far = Vec::new();
    let mut ground = 0;
    for (kind, transform, range, parent, no_ray_proxy) in foliage {
        assert!(transform.translation.is_finite());
        assert!(!range.use_aabb);
        match kind {
            Foliage::DetailedCrown => {
                assert!(!no_ray_proxy);
                assert!(near
                    .insert(key(parent, transform), (transform, range))
                    .is_none());
            }
            Foliage::PairedCrown => {
                assert!(no_ray_proxy, "A paired crown duplicated its ray occluder");
                far.push((key(parent, transform), transform, range));
            }
            Foliage::Ground => ground += 1,
            _ => {}
        }
    }
    assert!(
        !near.is_empty(),
        "Native foliage fixture contained no crown pairs"
    );
    assert_eq!(near.len(), far.len());
    for (key, transform, range) in far {
        let (near_transform, near_range) = near.get(&key).expect("Unpaired crown");
        assert_eq!(
            *near_transform, transform,
            "Crossfade origins must match exactly"
        );
        assert_eq!(near_range.end_margin, range.start_margin);
        assert_eq!(
            near_range.end_margin.start, near_range.end_margin.end,
            "Crown topology switch should use the geometric endpoint, not alpha dithering"
        );
        assert!(range.start_margin.end <= range.end_margin.start);
    }
    assert!(
        ground > 0,
        "Native foliage fixture contained no ground batches"
    );
    std::fs::write(
        "foliage-render-audit.json",
        serde_json::to_vec_pretty(&serde_json::json!({
            "status": "passed",
            "scope": "Native scene structure and shader execution; temporal appearance still needs route captures",
            "paired_crowns": near.len(), "ground_batches": ground,
            "paired_origins_and_margins_match": true, "duplicate_ray_proxies_excluded": true,
            "crown_transition": "geometric morph to species-specific target; no crown crossfade dithering"
        })).unwrap(),
    ).expect("Write foliage audit");
    info!("SMOKE: distance-faded foliage pairs, shadows and ground anchors passed");
}

/// Egui 0.34 renders to the window swapchain; Bevy's redirected screenshot texture
/// omits that overlay. Hosted Linux QA captures the real X11 window instead.
pub(crate) fn capture(commands: &mut Commands, path: &str) {
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

#[derive(Resource, Default)]
pub struct LearnedSmoke {
    frames: u32,
    positioned: bool,
    captured: Option<u32>,
    detailed: bool,
}
pub fn learned_start(
    mut smoke: ResMut<LearnedSmoke>,
    mut disk: ResMut<Disk>,
    mut graphics: ResMut<Graphics>,
    mut ui: ResMut<UiState>,
) {
    let args: Vec<String> = std::env::args().collect();
    if let Some(at) = args.iter().position(|a| a == "--detailed-smoke-test") {
        smoke.detailed = true;
        if let Some(path) = args.get(at + 1) {
            storage::open(&mut disk, Some(path.into()));
        } else {
            storage::detailed_example(&mut disk);
        }
    } else {
        storage::example(&mut disk);
    }
    graphics.quality = Quality::Balanced;
    graphics.radius = 3;
    ui.inspector = false;
}
pub fn learned_check(
    mut commands: Commands,
    mut smoke: ResMut<LearnedSmoke>,
    mut session: ResMut<Session>,
    mut player: ResMut<Player>,
    stream: Res<Stream>,
    mut exit: MessageWriter<AppExit>,
    limits: Res<crate::performance::Performance>,
    target: Res<crate::scene::SceneTarget>,
    cameras: Query<(&Camera, &GlobalTransform), With<crate::scene::MainCamera>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    let auto = std::env::args().any(|a| a == "--auto-smoke-test");
    if std::env::args().any(|a| a == "--eco-smoke-test") || auto {
        assert!(stream.queue() <= 1, "Eco exceeded its job budget");
        assert!(
            stream.loaded.len() <= 25,
            "Eco exceeded its residency budget"
        );
        assert_eq!(stream.far_triangles, 0, "Eco scheduled distant terrain");
    }
    smoke.frames += 1;
    assert!(smoke.frames < 2500, "Learned terrain render timed out");
    if let Some(frame) = smoke.captured {
        if smoke.frames > frame + 20
            && std::path::Path::new(if smoke.detailed {
                "worldcraft-detailed.png"
            } else {
                "worldcraft-learned.png"
            })
            .exists()
        {
            exit.write(AppExit::Success);
        }
        return;
    }
    let Some(g) = session.generator.clone() else {
        return;
    };
    if !smoke.positioned {
        let learned = session.world.as_ref().unwrap().learned.as_ref().unwrap();
        if smoke.detailed {
            assert!(learned.detail.is_some());
            player.position.y = g.height(player.position.x, player.position.z) + 80.0;
        }
        session.mode = Mode::Fly;
        player.pitch = -0.28;
        smoke.positioned = true;
    }
    if smoke.frames > 100
        && stream.loaded.len() == stream.desired
        && stream.queue() == 0
        && (auto || std::env::args().any(|a| a == "--eco-smoke-test") || stream.far_triangles > 0)
    {
        let (camera, transform) = cameras.single().unwrap();
        let window = windows.single().unwrap();
        // Exercise the same coordinate path as editing against the initialized
        // native image camera, including Auto's 50% target. This checks projection,
        // not physical mouse/egui event delivery.
        for fraction in [
            Vec2::splat(0.5),
            Vec2::new(0.15, 0.25),
            Vec2::new(0.85, 0.75),
        ] {
            let mapped = crate::control::viewport_cursor(camera, window, fraction * window.size());
            let ray = camera.viewport_to_world(transform, mapped).unwrap();
            let projected = camera
                .world_to_viewport(transform, ray.origin + *ray.direction * 100.0)
                .unwrap();
            assert!(
                (mapped - projected).length() < 0.02,
                "Scaled cursor projection drifted"
            );
            if fraction == Vec2::splat(0.5) {
                assert!(ray.direction.dot(*transform.forward()) > 0.99999);
            }
        }
        let (x, z) = (player.position.x, player.position.z);
        let before = g.height(x, z);
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
        let world = session.snapshot(&player).unwrap();
        let path = std::env::temp_dir().join("worldcraft-learned-audit.json");
        save_world(&path, &world).unwrap();
        let loaded = load_world(&path).unwrap();
        assert_eq!(world, loaded);
        assert_eq!(before, Generator::for_world(&loaded).unwrap().height(x, z));
        let image = if smoke.detailed {
            "worldcraft-detailed.png"
        } else {
            "worldcraft-learned.png"
        };
        capture(&mut commands, image);
        if auto {
            assert_eq!(limits.preferences.mode, crate::performance::Mode::Auto);
            assert_eq!(
                limits.controller.tier, 0,
                "Automatic software floor was not preserved"
            );
            assert_eq!(limits.render_scale(), 0.5);
            assert_eq!(limits.quality(Quality::High), Quality::Low);
            std::fs::write("automatic-render-audit.json",serde_json::to_vec_pretty(&serde_json::json!({"status":"passed","scope":"actual software adapter automatic floor; not gaming FPS certification","tier":limits.controller.tier,"scale":limits.render_scale(),"width":target.size.x,"height":target.size.y,"resident_chunks":stream.loaded.len(),"jobs":stream.queue(),"saved_world_unchanged":world==loaded,"scaled_cursor_projection_checked":true})).unwrap()).expect("write automatic audit");
        }
        if smoke.detailed {
            let detail = loaded.learned.as_ref().unwrap().detail.as_ref().unwrap();
            let report = serde_json::json!({"status":"passed","scope":"Native rendered detailed saved world; software graphics, not a hardware benchmark","sculpt_undo_reopen":true,"exact_round_trip":world==loaded,"detail_side":detail.side,"detail_spacing_metres":detail.spacing,"loaded_chunks":stream.loaded.len(),"far_triangles":stream.far_triangles,"image":image});
            std::fs::write(
                "detail-render-audit.json",
                serde_json::to_vec_pretty(&report).unwrap(),
            )
            .unwrap();
        }
        info!("LEARNED AUDIT: native learned relief, distant rings, sculpt/undo and exact save/reopen passed");
        smoke.captured = Some(smoke.frames);
    }
}

fn settle_frames(normal: u32) -> u32 {
    if cfg!(target_os = "windows") {
        (normal / 3).max(3)
    } else {
        normal
    }
}

pub fn calibration_check(
    limits: Res<crate::performance::Performance>,
    probe: Res<crate::calibration::Probe>,
    cameras: Query<&Camera, With<crate::scene::MainCamera>>,
    mut settled: Local<u8>,
    mut exit: MessageWriter<AppExit>,
) {
    if !limits.calibrating && probe.done {
        *settled += 1;
        if *settled >= 8 {
            assert!(
                cameras.iter().all(|c| !c.is_active),
                "Home camera still rendering after calibration"
            );
            assert!(
                std::path::Path::new("calibration-audit.json").exists(),
                "Missing measured calibration report"
            );
            exit.write(AppExit::Success);
        }
    }
}
