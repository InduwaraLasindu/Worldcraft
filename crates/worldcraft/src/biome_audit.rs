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
const CASES: [Case; 7] = [
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
    Case {
        label: "estuary",
        seed: 42,
        preset: Preset::Estuary,
        biome: Biome::Beach,
        hour: 8.0,
        overlook: true,
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
    spawn_view: bool,
    scanned_comparison: bool,
    start_draws: u64,
}

pub fn start(mut audit: ResMut<Audit>, mut ui: ResMut<UiState>) {
    if let Ok(label) = std::env::var("WORLDCRAFT_BIOME_CASE") {
        audit.spawn_view = label == "estuary-spawn";
        let label = if audit.spawn_view { "estuary" } else { &label };
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
pub(super) fn site(g: &Generator, biome: Biome) -> DVec3 {
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
    ocean: Query<&Mesh3d, With<crate::vista::OceanSurface>>,
    active: Res<crate::terrain_texture::ActiveSurfaces>,
    mut limits: ResMut<crate::performance::Performance>,
    mut exit: MessageWriter<AppExit>,
) {
    audit.frames += 1;
    assert!(audit.frames < 2400, "Procedural biome route did not settle");
    if let Some(frame) = audit.capture {
        let path = if audit.scanned_comparison {
            "worldcraft-material-savanna-scanned.png".to_owned()
        } else {
            format!("worldcraft-biome-{}.png", CASES[audit.case].label)
        };
        if audit.frames <= frame + 3 || !std::path::Path::new(&path).is_file() {
            return;
        }
        audit.capture = None;
        if CASES[audit.case].label == "savanna" && !audit.scanned_comparison {
            audit.scanned_comparison = true;
            limits.preferences.render.procedural_surfaces = false;
            audit.ready = 0;
            audit.start_draws = evidence.draws();
            return;
        }
        limits.preferences.render.procedural_surfaces = true;
        audit.scanned_comparison = false;
        info!("BIOME AUDIT: captured {}", CASES[audit.case].label);
        audit.case += 1;
        audit.initialized = false;
        if audit.single || audit.case == CASES.len() {
            std::fs::write("biome-render-audit.json", serde_json::to_vec_pretty(&serde_json::json!({
                "source":std::env::var("GITHUB_SHA").unwrap_or_else(|_| "local".into()),
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
        limits.preferences.render.procedural_surfaces = true;
        session.create(
            format!("Procedural {} audit", case.label),
            GeneratorSettings::for_preset(case.preset, case.seed),
            &mut player,
        );
        let g = session.generator.as_ref().unwrap();
        let target = if case.preset == Preset::Estuary {
            let z = 165.0;
            DVec3::new(
                worldcraft_core::estuary::channel_x(g.settings(), z),
                g.settings().sea_level,
                z,
            )
        } else {
            site(g, case.biome)
        };
        if !audit.spawn_view {
            player.position = if case.preset == Preset::Estuary {
                target + DVec3::new(-95.0, 48.0, 125.0)
            } else if case.overlook {
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
        }
        player.captured = false;
        session.mode = Mode::Fly;
        if std::env::var_os("WORLDCRAFT_AUDIT_4K").is_some() {
            limits.preferences.render.surface_resolution =
                crate::render_options::SurfaceResolution::FourK;
        }
        graphics.quality = Quality::High;
        graphics.radius = 2;
        graphics.hour = case.hour;
        graphics.breeze = 0.0;
        graphics.cycle = false;
        graphics.clouds = true;
        graphics.volumetrics = case.preset == Preset::Estuary
            && std::env::var_os("WORLDCRAFT_AUDIT_CLEAR_AIR").is_none();
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
    if active.0 == audit.scanned_comparison
        || (std::env::var_os("WORLDCRAFT_AUDIT_4K").is_some()
            && active.3 != if active.2 { 4096 } else { 1024 })
    {
        audit.ready = 0;
        return;
    }
    if audit.scanned_comparison {
        crate::verify::capture(&mut commands, "worldcraft-material-savanna-scanned.png");
        audit.capture = Some(audit.frames);
        return;
    }
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
    let estuary_depth_range = if case.preset == Preset::Estuary {
        let id = ocean.single().expect("Exactly one active water surface");
        let water = meshes.get(&id.0).expect("Resident estuary water");
        assert_eq!(water.count_vertices(), 2601);
        let Some(bevy::mesh::VertexAttributeValues::Float32x2(uv)) =
            water.attribute(Mesh::ATTRIBUTE_UV_0)
        else {
            panic!("Missing water depth samples");
        };
        let (lo, hi) = uv
            .iter()
            .filter(|v| v[1] > 0.0)
            .map(|v| v[0])
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), d| {
                (lo.min(d), hi.max(d))
            });
        assert!(lo < 0.0 && hi > 1.0);
        Some([lo, hi])
    } else {
        None
    };
    let cloud_draws = evidence.draws() - audit.start_draws;
    let sample = g.sample(player.position.x, player.position.z);
    let spawn_view = audit.spawn_view;
    audit.rows.push(serde_json::json!({
        "label":case.label,"ordinary_spawn_camera":spawn_view,"estuary_depth_range_m":estuary_depth_range,"seed":case.seed,"preset":format!("{:?}",case.preset),
        "procedural_surfaces":active.0,"surface_texture_side":active.3,"volumetrics":graphics.volumetrics, "biome":format!("{:?}",case.biome),"camera":player.position.to_array(),
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

// Follow one real tree through the detailed-crown and outer-visibility bands.
// Small frame-by-frame steps also exercise tile streaming and temporal history.
#[derive(Resource, Default)]
pub struct Travel {
    tree: DVec3,
    direction: DVec3,
    distance: u32,
    hold: u32,
    frames: u32,
    moved_at: Option<std::time::Instant>,
    moving_ms: Vec<f64>,
    worst_queue: usize,
    peak_tiles: usize,
    captures: Vec<serde_json::Value>,
}
const TRAVEL_STOPS: [u32; 8] = [92, 126, 130, 160, 192, 218, 226, 248];

pub fn start_travel(
    mut route: ResMut<Travel>,
    mut session: ResMut<Session>,
    mut player: ResMut<Player>,
    mut graphics: ResMut<Graphics>,
    mut ui: ResMut<UiState>,
) {
    let settings = GeneratorSettings::for_preset(Preset::Temperate, 42);
    session.create("Woodland travel audit".into(), settings, &mut player);
    let g = session.generator.as_ref().unwrap();
    let site = site(g, Biome::Forest);
    let tree = scatter(g, ChunkCoord::at(site.x, site.z), 1)
        .into_iter()
        .filter(|p| p.species.is_tree())
        .min_by(|a, b| {
            DVec3::from_array(a.position)
                .distance_squared(site)
                .total_cmp(&DVec3::from_array(b.position).distance_squared(site))
        })
        .expect("The selected woodland must contain a natural tree");
    route.tree = DVec3::from_array(tree.position);
    route.direction = DVec3::new(0.32, 0.34, 0.90).normalize();
    route.distance = TRAVEL_STOPS[0];
    set_travel_pose(&route, &mut player);
    graphics.quality = Quality::High;
    graphics.radius = 2;
    graphics.hour = 10.0;
    graphics.cycle = false;
    graphics.breeze = 0.0;
    graphics.clouds = true;
    graphics.volumetrics = false;
    session.mode = Mode::Fly;
    ui.home = false;
    ui.inspector = false;
    ui.photo = true;
}
fn set_travel_pose(route: &Travel, player: &mut Player) {
    player.position = route.tree + route.direction * f64::from(route.distance);
    let aim = route.tree + DVec3::Y * 7.0 - player.position;
    player.yaw = (-aim.x).atan2(-aim.z) as f32;
    player.pitch = aim.y.atan2(aim.x.hypot(aim.z)) as f32;
    player.captured = false;
}

pub fn travel(
    mut commands: Commands,
    mut route: ResMut<Travel>,
    stream: Res<Stream>,
    session: Res<Session>,
    caps: Res<crate::capabilities::Capabilities>,
    target: Res<crate::scene::SceneTarget>,
    mut player: ResMut<Player>,
    mut exit: MessageWriter<AppExit>,
) {
    let frame = std::time::Instant::now();
    if let Some(previous) = route.moved_at.take() {
        route
            .moving_ms
            .push(frame.duration_since(previous).as_secs_f64() * 1000.0);
    }
    route.frames += 1;
    assert!(route.frames < 1800, "Woodland travel route did not finish");
    route.worst_queue = route.worst_queue.max(stream.queue());
    route.peak_tiles = route.peak_tiles.max(stream.loaded.len());
    assert!(route.worst_queue <= 4 && route.peak_tiles <= 25);
    let distance = route.distance;
    if TRAVEL_STOPS.contains(&distance) {
        if stream.loaded.len() != 25 || stream.queue() != 0 || stream.far_triangles == 0 {
            route.hold = 0;
            return;
        }
        // X11 captures the last presented image. Hold the real camera long
        // enough for presentation and for a native-resolution temporal history.
        route.hold += 1;
        if route.hold < 8 {
            return;
        }
        let name = format!("worldcraft-travel-{distance:03}.png");
        if route.hold == 8 {
            assert!((player.position.distance(route.tree) - f64::from(distance)).abs() < 0.001);
            crate::verify::capture(&mut commands, &name);
            let position = player.position.to_array();
            route
                .captures
                .push(serde_json::json!({"distance_m":distance,"eye":position,
                "origin":session.origin.to_array(),"resident_chunks":stream.loaded.len(),
                "queued_jobs":stream.queue(),"image":name}));
            return;
        }
        if !std::path::Path::new(&name).is_file() {
            return;
        }
        route.hold = 0;
        if distance == TRAVEL_STOPS[TRAVEL_STOPS.len() - 1] {
            let mut times = route.moving_ms.clone();
            times.sort_by(f64::total_cmp);
            assert_eq!(
                times.len(),
                ((TRAVEL_STOPS.last().unwrap() - TRAVEL_STOPS[0]) / 2) as usize
            );
            let percentile = |p: usize| times[(times.len() * p).div_ceil(100) - 1];
            let report = serde_json::json!({"source":std::env::var("GITHUB_SHA").unwrap_or_else(|_| "local".into()),
                "seed":42,"authored_instances":0,"tree":route.tree.to_array(),
                "step_m":2,"crown_switch_m":crate::foliage::Foliage::crown_end(2),
                "tree_cutoff_m":crate::foliage::Foliage::outer_end(2),
                "adapter":caps.gpu,"backend":caps.backend,"software":caps.software,
                "render_size":[target.size.x,target.size.y],
                "moving_frames":{"samples":times.len(),"p50_ms":percentile(50),"p95_ms":percentile(95),
                "p99_ms":percentile(99),"intervals_ms":route.moving_ms},
                "peak_jobs":route.worst_queue,"peak_resident_chunks":route.peak_tiles,"captures":route.captures});
            std::fs::write(
                "forest-travel-audit.json",
                serde_json::to_vec_pretty(&report).unwrap(),
            )
            .unwrap();
            exit.write(AppExit::Success);
            return;
        }
    }
    route.moved_at = Some(frame);
    route.distance += 2;
    set_travel_pose(&route, &mut player);
}
