//! Native art fixture: actual shared production assets on a flat saved-world canvas.
//! It is deliberately separate from ecology/placement and from hardware performance claims.
use crate::{
    assets::Nature,
    session::*,
    streaming::{ChunkRoot, Stream},
};
use bevy::{app::AppExit, math::DVec3, prelude::*};
use worldcraft_core::{scatter::Species, ChunkCoord, GeneratorSettings, Preset};

#[derive(Resource, Default)]
pub struct Audit {
    frames: u32,
    stage: usize,
    ready: u32,
    capture: Option<u32>,
    root: Option<Entity>,
    forest: bool,
    target: DVec3,
    cloud_draws: u64,
}

pub fn start(
    mut session: ResMut<Session>,
    mut player: ResMut<Player>,
    mut ui: ResMut<UiState>,
    mut graphics: ResMut<Graphics>,
) {
    session.create(
        "Nature material audit".into(),
        GeneratorSettings::for_preset(Preset::Empty, 20261117),
        &mut player,
    );
    player.position = DVec3::new(-320.0, 3.0, -168.0);
    session.mode = Mode::Fly;
    player.yaw = 0.0;
    player.pitch = 0.04;
    ui.home = false;
    ui.inspector = false;
    graphics.quality = Quality::Balanced;
    graphics.radius = 2;
    graphics.hour = 9.0;
    graphics.breeze = 0.0;
    graphics.cycle = false;
}

#[allow(clippy::too_many_arguments)]
pub fn check(
    mut commands: Commands,
    mut audit: ResMut<Audit>,
    nature: Res<Nature>,
    stream: Res<Stream>,
    mut session: ResMut<Session>,
    mut player: ResMut<Player>,
    mut graphics: ResMut<Graphics>,
    mut exit: MessageWriter<AppExit>,
    clouds: Res<crate::clouds::CloudGpuEvidence>,
) {
    audit.frames += 1;
    assert!(audit.frames < 2000, "Nature art fixture did not finish");
    let labels = [
        "broadleaf",
        "pine",
        "palm",
        "bark",
        "coarse",
        "rebase",
        "sunset",
        "forest-close",
        "forest-overlook",
        "forest-high",
        "clouds-clear",
        "clouds-high",
    ];
    if let Some(frame) = audit.capture {
        if audit.frames <= frame + 5
            || !std::path::Path::new(&format!("worldcraft-nature-{}.png", labels[audit.stage]))
                .is_file()
        {
            return;
        }
        audit.capture = None;
        audit.stage += 1;
        audit.ready = 0;
        if audit.stage == labels.len() {
            let report = serde_json::json!({"status":"passed","scope":"Native production-asset art fixture; not procedural ecosystem or gaming GPU certification",
                "seed":20261117,"world_presets":["Empty","Temperate"],"negative_coordinates":true,"capture_labels":labels,
                "cloud_draw_commands":clouds.draws()-audit.cloud_draws,
                "texture_mips":"explicit alpha-coverage-preserving chain","save_format_changed":false});
            std::fs::write(
                "nature-render-audit.json",
                serde_json::to_vec_pretty(&report).unwrap(),
            )
            .unwrap();
            info!("NATURE AUDIT: three species, textured bark, coarse endpoint, origin rebase and sunset rendered");
            exit.write(AppExit::Success);
            return;
        }
        if audit.stage < 3 {
            if let Some(root) = audit.root.take() {
                commands.entity(root).despawn();
            }
        }
    }
    if audit.stage < 3 && audit.root.is_none() {
        let species = [Species::Broadleaf, Species::Pine, Species::Palm][audit.stage];
        let root = commands
            .spawn((
                ChunkRoot(ChunkCoord { x: -3, z: -2 }),
                Transform::default(),
                Visibility::default(),
            ))
            .id();
        let height = session.generator.as_ref().unwrap().height(-320.0, -200.0) as f32;
        commands.entity(root).with_children(|p| {
            for (v, x, z) in [
                (0, 64.0, 56.0),
                (1, 52.0, 44.0),
                (2, 77.0, 40.0),
                (3, 46.0, 20.0),
                (0, 83.0, 12.0),
            ] {
                let transform = Transform::from_xyz(x, height, z)
                    .with_rotation(Quat::from_rotation_y(v as f32 * 0.9));
                p.spawn((
                    Mesh3d(nature.trunk(species, v).clone()),
                    MeshMaterial3d(nature.bark.clone()),
                    transform,
                ));
                p.spawn((
                    Mesh3d(nature.crown(species, v, false).clone()),
                    MeshMaterial3d(nature.foliage(species).clone()),
                    transform,
                    crate::foliage::Foliage::DetailedCrown,
                    crate::foliage::Foliage::DetailedCrown.range(2),
                ));
                p.spawn((
                    Mesh3d(nature.crown(species, v, true).clone()),
                    MeshMaterial3d(nature.foliage(species).clone()),
                    transform,
                    crate::foliage::Foliage::PairedCrown,
                    crate::foliage::Foliage::PairedCrown.range(2),
                    crate::foliage::NoRayProxy,
                ));
            }
        });
        audit.root = Some(root);
        player.position.y = height as f64 + 3.0;
    }
    if audit.stage == 3 {
        // Real, normal-mapped palm stem at walking distance; reset for the distant route.
        let height = session.generator.as_ref().unwrap().height(-320.0, -200.0);
        player.position = DVec3::new(-318.0, height + 2.0, -197.0);
        player.yaw = 0.3;
        player.pitch = 0.12;
    } else if audit.stage == 4 {
        let height = session.generator.as_ref().unwrap().height(-320.0, -200.0);
        player.position = DVec3::new(-320.0, height + 5.0, -64.0);
        player.yaw = 0.0;
        player.pitch = -0.02;
        // This view exercises the 128m crown endpoint. Normal terrain smoke separately
        // checks production near/far entity pairing, observer ranges and residency.
    } else if audit.stage == 5 {
        // Keep a stationary world observer and translate every render-space root.
        // scene::update handles roots and camera from the common session origin.
        // Origin is changed once by a separate system below to avoid a Res/ResMut alias.
    } else if audit.stage == 6 {
        graphics.hour = 17.0;
        graphics.quality = Quality::High;
        player.position = DVec3::new(
            -320.0,
            session.generator.as_ref().unwrap().height(-320.0, -200.0) + 3.0,
            -168.0,
        );
        player.pitch = 0.04;
    }
    if audit.stage == 7 && !audit.forest {
        if let Some(root) = audit.root.take() {
            commands.entity(root).despawn();
        }
        let mut settings = GeneratorSettings::for_preset(Preset::Temperate, 20261116);
        settings.vegetation = 1.25;
        settings.rainfall = 0.75;
        session.create(
            "Generated forest visual audit".into(),
            settings,
            &mut player,
        );
        let g = session.generator.as_ref().unwrap();
        let center = ChunkCoord::at(player.position.x, player.position.z);
        let target = worldcraft_core::streaming::wanted(center, 2)
            .into_iter()
            .flat_map(|c| worldcraft_core::scatter::scatter(g, c, 0))
            .find(|i| matches!(i.species, Species::Broadleaf | Species::Pine));
        audit.target = target.map_or(player.position, |i| DVec3::from_array(i.position));
        player.position = audit.target + DVec3::new(5.0, 0.0, 12.0);
        player.position.y = g.height(player.position.x, player.position.z) + 2.2;
        let delta = audit.target + DVec3::Y * 4.0 - player.position;
        player.yaw = (-delta.x).atan2(-delta.z) as f32;
        player.pitch = delta.y.atan2(delta.x.hypot(delta.z)) as f32;
        session.mode = Mode::Fly;
        graphics.hour = 9.0;
        graphics.breeze = 0.45;
        graphics.quality = Quality::Balanced;
        audit.forest = true;
    } else if audit.stage == 8 {
        player.position = audit.target + DVec3::new(0.0, 35.0, 60.0);
        let delta = audit.target - player.position;
        player.yaw = (-delta.x).atan2(-delta.z) as f32;
        player.pitch = delta.y.atan2(delta.x.hypot(delta.z)) as f32;
    } else if audit.stage == 9 {
        graphics.quality = Quality::High;
        graphics.hour = 11.0;
        player.position = audit.target + DVec3::new(5.0, 0.0, 12.0);
        player.position.y = session
            .generator
            .as_ref()
            .unwrap()
            .height(player.position.x, player.position.z)
            + 2.2;
        let delta = audit.target + DVec3::Y * 4.0 - player.position;
        player.yaw = (-delta.x).atan2(-delta.z) as f32;
        player.pitch = delta.y.atan2(delta.x.hypot(delta.z)) as f32;
    } else if audit.stage >= 10 {
        graphics.clouds = audit.stage == 11;
        graphics.volumetrics = true;
        graphics.breeze = 0.0;
        player.position = audit.target + DVec3::new(0.0, 35.0, 60.0);
        player.pitch = 0.25;
        if audit.stage == 10 {
            audit.cloud_draws = clouds.draws();
        }
    }
    if stream.loaded.len() != stream.desired || stream.queue() != 0 {
        return;
    }
    audit.ready += 1;
    if audit.ready >= 12 && (audit.stage != 11 || clouds.draws() > audit.cloud_draws + 4) {
        crate::verify::capture(
            &mut commands,
            &format!("worldcraft-nature-{}.png", labels[audit.stage]),
        );
        audit.capture = Some(audit.frames);
    }
}
pub fn rebase(audit: Res<Audit>, mut session: ResMut<Session>) {
    if audit.stage == 5 {
        session.origin = DVec3::new(-256.0, 0.0, -256.0);
    }
}
