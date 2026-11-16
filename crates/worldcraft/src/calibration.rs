//! A bounded native launch probe. A small rendered fixture, never a user's world.
use crate::{
    assets::Nature,
    performance::{Mode, Performance},
    session::*,
};
use bevy::{
    camera::{visibility::RenderLayers, RenderTarget},
    prelude::*,
    tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task},
    window::PrimaryWindow,
};
use std::time::Instant;
#[derive(Component)]
pub struct ProbeRoot;
#[derive(Component)]
pub struct ProbeCamera;
#[derive(Component)]
pub struct CullingWitness(pub bool);
#[derive(Resource, Default)]
pub struct Probe {
    started: Option<Instant>,
    measuring: Option<Instant>,
    warmup_frames: u32,
    last: Option<Instant>,
    samples: Vec<f64>,
    cpu: Option<Task<f64>>,
    cpu_ms: Option<f64>,
    pub done: bool,
}
pub fn setup(
    mut commands: Commands,
    nature: Res<Nature>,
    limits: Res<Performance>,
    target: Res<crate::scene::SceneTarget>,
) {
    if !limits.calibrating {
        return;
    }
    let root = commands
        .spawn((ProbeRoot, Transform::default(), Visibility::default()))
        .id();
    commands.entity(root).with_children(|p| {
        p.spawn((
            Camera3d::default(),
            ProbeCamera,
            Camera {
                order: -1,
                ..default()
            },
            RenderTarget::Image(target.image.clone().into()),
            Transform::from_xyz(32.0, 24.0, 32.0).looking_at(Vec3::new(0.0, 4.0, 0.0), Vec3::Y),
            RenderLayers::layer(31),
            Msaa::Off,
        ));
        p.spawn((
            DirectionalLight {
                illuminance: 30_000.0,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_xyz(1.0, 3.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
            RenderLayers::layer(31),
        ));
        for (inside, position) in [(true, Vec3::ZERO), (false, Vec3::new(120.0, 0.0, 120.0))] {
            p.spawn((
                CullingWitness(inside),
                Mesh3d(nature.trunks[0].clone()),
                MeshMaterial3d(nature.bark.clone()),
                Transform::from_translation(position),
                RenderLayers::layer(31),
            ));
        }
        for z in -4..4 {
            for x in -4..4 {
                let v = ((x + 4) + (z + 4) * 8) as usize % 4;
                let transform = Transform::from_xyz(x as f32 * 6.0, 0.0, z as f32 * 6.0);
                p.spawn((
                    Mesh3d(nature.crowns[v].clone()),
                    MeshMaterial3d(nature.leaf.clone()),
                    transform,
                    RenderLayers::layer(31),
                ));
                p.spawn((
                    Mesh3d(nature.trunks[v].clone()),
                    MeshMaterial3d(nature.bark.clone()),
                    transform,
                    RenderLayers::layer(31),
                ));
            }
        }
    });
    let cpu = AsyncComputeTaskPool::get().spawn(async {
        let start = Instant::now();
        let g =
            worldcraft_core::Generator::new(worldcraft_core::GeneratorSettings::default(), vec![]);
        let mut checksum = 0.0;
        for z in 0..8 {
            for x in 0..8 {
                checksum += g.height(x as f64 * 8.0, z as f64 * 8.0);
            }
        }
        std::hint::black_box(checksum);
        start.elapsed().as_secs_f64() * 1000.0
    });
    commands.insert_resource(Probe {
        cpu: Some(cpu),
        ..default()
    });
}
#[allow(clippy::too_many_arguments)]
pub fn update(
    mut commands: Commands,
    mut probe: ResMut<Probe>,
    mut limits: ResMut<Performance>,
    ui: Res<UiState>,
    windows: Query<&Window, With<PrimaryWindow>>,
    roots: Query<Entity, With<ProbeRoot>>,
    witnesses: Query<(&CullingWitness, &ViewVisibility)>,
    mut cameras: Query<&mut Camera, With<ProbeCamera>>,
    target: Res<crate::scene::SceneTarget>,
) {
    if !limits.calibrating || probe.done {
        return;
    }
    if limits.preferences.mode != Mode::Auto || !ui.home {
        limits.calibrating = false;
        probe.done = true;
        limits.calibration_status =
            "Launch probe skipped; live adaptation remains available in Automatic.".into();
        for e in &roots {
            commands.entity(e).despawn();
        }
        return;
    }
    let focused = limits.audit
        || windows
            .iter()
            .any(|w| w.focused && w.physical_width() > 0 && w.physical_height() > 0);
    for mut camera in &mut cameras {
        camera.is_active = focused;
    }
    if !limits.audit
        && !windows
            .iter()
            .any(|w| w.focused && w.physical_width() > 0 && w.physical_height() > 0)
    {
        probe.started = None;
        probe.measuring = None;
        probe.warmup_frames = 0;
        probe.last = None;
        probe.samples.clear();
        return;
    }
    if let Some(cpu) = probe
        .cpu
        .as_mut()
        .and_then(|t| block_on(future::poll_once(t)))
    {
        probe.cpu_ms = Some(cpu);
        probe.cpu = None;
    }
    let now = Instant::now();
    let start = *probe.started.get_or_insert(now);
    if probe.measuring.is_some() {
        if let Some(last) = probe.last {
            if probe.samples.len() < 256 {
                probe
                    .samples
                    .push(now.duration_since(last).as_secs_f64() * 1000.0);
            }
        }
    } else {
        probe.warmup_frames = probe.warmup_frames.saturating_add(1);
        if start.elapsed().as_secs_f64() >= 1.0 && probe.warmup_frames >= 8 {
            // First-use shader/pipeline compilation can consume the entire fixed
            // launch interval. Measure after several rendered frames, not inside it.
            probe.measuring = Some(now);
        }
    }
    probe.last = Some(now);
    let enough_time = probe
        .measuring
        .is_some_and(|at| at.elapsed().as_secs_f64() >= 2.0);
    if !enough_time && start.elapsed().as_secs_f64() < 8.0 {
        return;
    }
    let p90 = worldcraft_core::adaptive::percentile90(&mut probe.samples);
    let selection_ms = if probe.samples.len() < 12 {
        p90.max(36.0)
    } else {
        p90
    };
    let cpu = probe.cpu_ms.unwrap_or(200.0);
    let m = limits.memory.unwrap_or_default();
    limits.controller.ceiling = if limits.software_gpu {
        0
    } else if limits.cpu_threads <= 4
        || m.total < 6 * worldcraft_core::hardware::GIB
        || m.available < 2 * worldcraft_core::hardware::GIB
    {
        1
    } else {
        4
    };
    let tier = worldcraft_core::adaptive::initial_tier(
        selection_ms,
        cpu,
        m.total as f64 / 1073741824.0,
        m.available as f64 / 1073741824.0,
        limits.software_gpu,
    );
    limits.controller.tier = tier.min(limits.controller.ceiling);
    limits.controller.reset_window();
    limits.calibration_status = format!(
        "Launch probe: p90 {p90:.1} ms · {} samples{} · terrain sampling {}. Settings continue adapting toward 30 FPS.",
        probe.samples.len(),
        if probe.samples.len() < 12 { " (limited; conservative settings)" } else { "" },
        probe.cpu_ms.map_or_else(|| "not ready".into(), |ms| format!("{ms:.1} ms")),
    );
    if limits.audit {
        let visible = witnesses.iter().any(|(w, v)| w.0 && v.get());
        let culled = witnesses.iter().any(|(w, v)| !w.0 && !v.get());
        assert!(
            visible && culled,
            "Launch fixture frustum culling did not distinguish the front and rear witnesses"
        );
        let report = serde_json::json!({"status":"passed","scope":"small software-renderable launch probe; not gaming-GPU certification","samples":probe.samples.len(),"warmup_frames":probe.warmup_frames,"probe_seconds":start.elapsed().as_secs_f64(),"measurement_seconds":probe.measuring.map_or(0.0,|at|at.elapsed().as_secs_f64()),"frustum_inside_visible":visible,"frustum_behind_culled":culled,"p90_frame_ms":p90,"probe_width":target.size.x,"probe_height":target.size.y,"cpu_terrain_ms":probe.cpu_ms,"automatic_tier":limits.controller.tier,"render_scale":limits.render_scale()});
        std::fs::write(
            "calibration-audit.json",
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .expect("write probe report");
    }
    limits.calibrating = false;
    probe.done = true;
    for e in &roots {
        commands.entity(e).despawn();
    }
    info!(
        "{} Tier {}",
        limits.calibration_status, limits.controller.tier
    );
}
