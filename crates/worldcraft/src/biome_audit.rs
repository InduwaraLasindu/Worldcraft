//! Procedural-world visual route. No authored plants, altered biome paint, or image effects.
use crate::{clouds::CloudGpuEvidence, session::*, streaming::Stream};
use bevy::{app::AppExit, math::DVec3, prelude::*};
use worldcraft_core::{scatter::scatter, Biome, ChunkCoord, Generator, GeneratorSettings, Preset};

#[derive(Clone, Copy)]
struct Case {
    label: &'static str,
    seed: u64,
    preset: Preset,
    biome: Biome,
    hour: f32,
    overlook: bool,
}
const CASES: [Case; 6] = [
    Case {
        label: "woodland",
        seed: 42,
        preset: Preset::Temperate,
        biome: Biome::Forest,
        hour: 10.0,
        overlook: false,
    },
    Case {
        label: "birch",
        seed: 20261118,
        preset: Preset::Temperate,
        biome: Biome::BirchForest,
        hour: 9.0,
        overlook: false,
    },
    Case {
        label: "savanna",
        seed: 761,
        preset: Preset::Savanna,
        biome: Biome::Savanna,
        hour: 16.0,
        overlook: false,
    },
    Case {
        label: "tropical",
        seed: 279,
        preset: Preset::Tropical,
        biome: Biome::Rainforest,
        hour: 10.0,
        overlook: false,
    },
    Case {
        label: "alpine",
        seed: 324,
        preset: Preset::Alpine,
        biome: Biome::Conifer,
        hour: 15.0,
        overlook: true,
    },
    Case {
        label: "meadow",
        seed: 42,
        preset: Preset::Temperate,
        biome: Biome::Meadow,
        hour: 11.0,
        overlook: false,
    },
];

#[derive(Resource, Default)]
pub struct Audit {
    case: usize,
    initialized: bool,
    ready: u32,
    frames: u32,
    capture: Option<u32>,
    rows: Vec<serde_json::Value>,
    single: bool,
    start_draws: u64,
}

pub fn start(mut audit: ResMut<Audit>, mut ui: ResMut<UiState>) {
    if let Ok(label) = std::env::var("WORLDCRAFT_BIOME_CASE") {
        audit.case = CASES
            .iter()
            .position(|c| c.label == label)
            .expect("Unknown biome audit case");
        audit.single = true;
    }
    ui.home = false;
    ui.inspector = false;
    ui.photo = true;
}

// A bounded search, independent of chunk residency. Preview only rejects climate
// candidates; the final camera always uses the exact production height/slope.
fn site(g: &Generator, biome: Biome) -> DVec3 {
    for ring in 0_i32..=24 {
        for z in -ring..=ring {
            for x in -ring..=ring {
                if x.abs().max(z.abs()) != ring {
                    continue;
                }
                let (wx, wz) = (x as f64 * 256.0 + 37.0, z as f64 * 256.0 - 41.0);
                let sample = g.preview_sample(wx, wz);
                if sample.biome != biome || sample.height < g.settings().sea_level + 5.0 {
                    continue;
                }
                let exact = g.sample(wx, wz);
                if exact.biome == biome && g.slope(wx, wz) < 0.55 {
                    return DVec3::new(wx, exact.height, wz);
                }
            }
        }
    }
    panic!("No walkable {biome:?} site in the bounded audit search");
}

#[allow(clippy::too_many_arguments)]
pub fn check(
    mut commands: Commands,
    mut audit: ResMut<Audit>,
    mut session: ResMut<Session>,
    mut player: ResMut<Player>,
    mut graphics: ResMut<Graphics>,
    stream: Res<Stream>,
    evidence: Res<CloudGpuEvidence>,
    meshes: Res<Assets<Mesh>>,
    mesh_entities: Query<&Mesh3d>,
    mut exit: MessageWriter<AppExit>,
) {
    audit.frames += 1;
    assert!(audit.frames < 2400, "Procedural biome route did not settle");
    if let Some(frame) = audit.capture {
        let path = format!("worldcraft-biome-{}.png", CASES[audit.case].label);
        if audit.frames <= frame + 3 || !std::path::Path::new(&path).is_file() {
            return;
        }
        audit.capture = None;
        info!("BIOME AUDIT: captured {}", CASES[audit.case].label);
        audit.case += 1;
        audit.initialized = false;
        if audit.single || audit.case == CASES.len() {
            std::fs::write("biome-render-audit.json", serde_json::to_vec_pretty(&serde_json::json!({
                "status":"passed", "scope":"Untouched procedural worlds, software/native rendering; visual quality and gaming performance require review",
                "generator_version":worldcraft_core::GENERATOR_VERSION, "authored_instances":0,
                "sites":audit.rows,
            })).unwrap()).unwrap();
            info!(
                "BIOME AUDIT: {} actual generated landscapes rendered",
                audit.rows.len()
            );
            exit.write(AppExit::Success);
            return;
        }
    }
    let case = CASES[audit.case];
    if !audit.initialized {
        session.create(
            format!("Procedural {} audit", case.label),
            GeneratorSettings::for_preset(case.preset, case.seed),
            &mut player,
        );
        let g = session.generator.as_ref().unwrap();
        let target = site(g, case.biome);
        player.position = if case.overlook {
            target + DVec3::new(0.0, 32.0, 55.0)
        } else {
            target + DVec3::new(5.0, 0.0, 12.0)
        };
        player.position.y = player
            .position
            .y
            .max(g.height(player.position.x, player.position.z) + 1.8);
        let delta = target + DVec3::Y * if case.overlook { 0.0 } else { 3.0 } - player.position;
        player.yaw = (-delta.x).atan2(-delta.z) as f32;
        player.pitch = delta.y.atan2(delta.x.hypot(delta.z)) as f32;
        player.captured = false;
        session.mode = Mode::Fly;
        graphics.quality = Quality::High;
        graphics.radius = 2;
        graphics.hour = case.hour;
        graphics.breeze = 0.0;
        graphics.cycle = false;
        graphics.clouds = true;
        graphics.volumetrics = false;
        audit.ready = 0;
        audit.start_draws = evidence.draws();
        audit.initialized = true;
        info!(
            "BIOME AUDIT: {} seed={} site={target:?}",
            case.label, case.seed
        );
        return;
    }
    if stream.loaded.len() != stream.desired || stream.queue() != 0 || stream.far_triangles == 0 {
        return;
    }
    audit.ready += 1;
    if audit.ready < 12 || evidence.draws() <= audit.start_draws + 4 {
        return;
    }
    let g = session.generator.as_ref().unwrap();
    let center = ChunkCoord::at(player.position.x, player.position.z);
    let instances = scatter(g, center, 0);
    let mut species = [0_usize; 11];
    for i in &instances {
        species[i.species as usize] += 1;
    }
    let triangles: usize = mesh_entities
        .iter()
        .filter_map(|h| meshes.get(&h.0))
        .map(|m| m.indices().map_or(m.count_vertices(), |i| i.len()) / 3)
        .sum();
    let cloud_draws = evidence.draws() - audit.start_draws;
    let sample = g.sample(player.position.x, player.position.z);
    audit.rows.push(serde_json::json!({
        "label":case.label,"seed":case.seed,"preset":format!("{:?}",case.preset),
        "biome":format!("{:?}",case.biome),"camera":player.position.to_array(),
        "yaw":player.yaw,"pitch":player.pitch,"hour":graphics.hour,
        "camera_terrain_palette":g.color(player.position.x, player.position.z),
        "camera_climate":[sample.temperature,sample.moisture],
        "camera_biome_weights":g.biome_blend(player.position.x, player.position.z, &sample).weights.to_vec(),
        "resident_chunks":stream.loaded.len(),"queued_jobs":stream.queue(),
        "center_species_counts":species,"resident_mesh_triangles":triangles,
        "far_triangles":stream.far_triangles,"cloud_draw_commands":cloud_draws,
        "image":format!("worldcraft-biome-{}.png",case.label),
    }));
    info!(
        "BIOME AUDIT: {} settled; {} triangles; {} cloud draws",
        case.label, triangles, cloud_draws
    );
    crate::verify::capture(
        &mut commands,
        &format!("worldcraft-biome-{}.png", case.label),
    );
    audit.capture = Some(audit.frames);
}
