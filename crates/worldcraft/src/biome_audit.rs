//! Procedural-world visual route. No authored plants, altered biome paint, or image effects.
use crate::{clouds::CloudGpuEvidence, session::*, streaming::Stream};
use bevy::{app::AppExit, math::DVec3, prelude::*};
use worldcraft_core::{scatter::scatter, Biome, ChunkCoord, Generator, GeneratorSettings, Preset};

#[derive(Clone, Copy, serde::Deserialize)]
struct Case<'a> {
    label: &'a str,
    seed: u64,
    preset: Preset,
    biome: Biome,
    hour: f32,
    #[serde(default)]
    overlook: bool,
    #[serde(default)]
    center: Option<[f64; 2]>,
}
const CASES: [Case<'static>; 7] = [
    Case {
        label: "woodland",
        seed: 42,
        preset: Preset::Temperate,
        biome: Biome::Forest,
        hour: 10.0,
        overlook: false,
        center: None,
    },
    Case {
        label: "birch",
        seed: 20261118,
        preset: Preset::Temperate,
        biome: Biome::BirchForest,
        hour: 9.0,
        overlook: false,
        center: None,
    },
    Case {
        label: "savanna",
        seed: 761,
        preset: Preset::Savanna,
        biome: Biome::Savanna,
        hour: 16.0,
        overlook: false,
        center: None,
    },
    Case {
        label: "tropical",
        seed: 279,
        preset: Preset::Tropical,
        biome: Biome::Rainforest,
        hour: 10.0,
        overlook: false,
        center: None,
    },
    Case {
        label: "alpine",
        seed: 324,
        preset: Preset::Alpine,
        biome: Biome::Conifer,
        hour: 15.0,
        overlook: true,
        center: None,
    },
    Case {
        label: "meadow",
        seed: 42,
        preset: Preset::Temperate,
        biome: Biome::Meadow,
        hour: 11.0,
        overlook: false,
        center: None,
    },
    Case {
        label: "estuary",
        seed: 42,
        preset: Preset::Estuary,
        biome: Biome::Beach,
        hour: 8.0,
        overlook: true,
        center: None,
    },
];

const RIVER_CASE: Case<'static> = Case {
    label: "river-valley",
    seed: 42,
    preset: Preset::RiverValley,
    biome: Biome::Forest,
    hour: 9.5,
    overlook: true,
    center: None,
};
#[derive(serde::Deserialize)]
struct FixtureManifest<'a> {
    version: u32,
    route_seconds: f64,
    views: std::collections::BTreeMap<String, [f64; 3]>,
    #[serde(borrow)]
    fixtures: Vec<Case<'a>>,
}

fn fixtures() -> FixtureManifest<'static> {
    let manifest: FixtureManifest<'static> =
        serde_json::from_str(include_str!("../../../docs/fixtures/photorealism-v1.json"))
            .expect("Photorealism fixture manifest");
    assert_eq!(manifest.version, 1);
    manifest
}

struct PhotorealFixture {
    case: Case<'static>,
    view: String,
    offset: DVec3,
    route_seconds: f64,
    target: DVec3,
    start: Option<(f64, DVec3)>,
    frame_ms: Vec<f64>,
}

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
    sun_view: bool,
    cloud_descent: bool,
    river_view: bool,
    settled_frame_ms: Vec<f64>,
    scanned_comparison: bool,
    start_draws: u64,
    photoreal: Option<PhotorealFixture>,
}

pub fn start(mut audit: ResMut<Audit>, mut ui: ResMut<UiState>) {
    if let Ok(label) = std::env::var("WORLDCRAFT_PHOTOREAL_CASE") {
        let manifest = fixtures();
        let view = std::env::var("WORLDCRAFT_PHOTOREAL_VIEW").unwrap_or_else(|_| "mid".into());
        let offset = manifest
            .views
            .get(if view == "route" { "close" } else { &view })
            .expect("Unknown photorealism fixture view");
        audit.photoreal = Some(PhotorealFixture {
            case: *manifest
                .fixtures
                .iter()
                .find(|case| case.label == label)
                .expect("Unknown photorealism fixture"),
            view,
            offset: DVec3::from_array(*offset),
            route_seconds: manifest.route_seconds,
            target: DVec3::ZERO,
            start: None,
            frame_ms: Vec::new(),
        });
        audit.single = true;
    } else if let Ok(label) = std::env::var("WORLDCRAFT_BIOME_CASE") {
        audit.river_view = label == RIVER_CASE.label;
        audit.spawn_view = label == "estuary-spawn";
        audit.sun_view = label == "woodland-sun";
        audit.cloud_descent = label == "estuary-cloud-descent";
        let label = if audit.spawn_view || audit.cloud_descent {
            "estuary"
        } else if audit.sun_view {
            "woodland"
        } else {
            &label
        };
        if !audit.river_view {
            audit.case = CASES
                .iter()
                .position(|c| c.label == label)
                .expect("Unknown biome audit case");
        }
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

fn photoreal_site(g: &Generator, case: Case<'_>) -> DVec3 {
    let [x, z] = case
        .center
        .expect("Photorealism fixture requires a fixed site");
    let sample = g.sample(x, z);
    assert_eq!(sample.biome, case.biome, "Fixed fixture changed biome");
    let height = if case.biome == Biome::Ocean {
        g.settings().sea_level
    } else {
        sample.height
    };
    DVec3::new(x, height, z)
}

fn aim_at(player: &mut Player, target: DVec3) {
    let delta = target - player.position;
    player.yaw = (-delta.x).atan2(-delta.z) as f32;
    player.pitch = delta.y.atan2(delta.x.hypot(delta.z)) as f32;
}

fn photoreal_route(
    fixture: &mut PhotorealFixture,
    player: &mut Player,
    generator: &Generator,
    time: &Time<Real>,
) -> Option<serde_json::Value> {
    let now = time.elapsed_secs_f64();
    let (started, origin) = *fixture.start.get_or_insert((now, player.position));
    let elapsed = now - started;
    if elapsed > 0.0 {
        fixture.frame_ms.push(time.delta_secs_f64() * 1000.0);
    }
    let distance = elapsed.min(fixture.route_seconds) * 6.0;
    let along = DVec3::new(0.6, 0.0, 0.8);
    player.position = origin + along * distance;
    player.position.y = player
        .position
        .y
        .max(generator.height(player.position.x, player.position.z) + 2.0);
    aim_at(player, fixture.target + along * distance);
    if elapsed < fixture.route_seconds {
        return None;
    }
    let mut sorted = fixture.frame_ms.clone();
    sorted.sort_by(f64::total_cmp);
    let percentile = |p: usize| sorted[(sorted.len() * p).div_ceil(100) - 1];
    Some(serde_json::json!({
        "elapsed_seconds": elapsed,
        "frames": sorted.len(),
        "p50_ms": percentile(50),
        "p95_ms": percentile(95),
        "p99_ms": percentile(99),
        "intervals_ms": fixture.frame_ms,
    }))
}

#[allow(clippy::too_many_arguments)]
fn woodland_opening(g: &Generator, site: DVec3, yaw: f32) -> DVec3 {
    let center = ChunkCoord::at(site.x, site.z);
    let mut trunks = Vec::new();
    for dz in -1..=1 {
        for dx in -1..=1 {
            trunks.extend(
                scatter(
                    g,
                    ChunkCoord {
                        x: center.x + dx,
                        z: center.z + dz,
                    },
                    0,
                )
                .into_iter()
                .filter(|p| {
                    p.species.is_tree() || p.species == worldcraft_core::scatter::Species::Rock
                }),
            );
        }
    }
    let forward = DVec3::new(-f64::from(yaw.sin()), 0.0, -f64::from(yaw.cos()));
    let mut best = (f64::NEG_INFINITY, site);
    for iz in -4..=4 {
        for ix in -4..=4 {
            let mut p = site + DVec3::new(f64::from(ix) * 3.0, 0.0, f64::from(iz) * 3.0);
            p.y = g.height(p.x, p.z) + 1.75;
            if g.slope(p.x, p.z) > 0.4 {
                continue;
            }
            let mut clearance = 10.0f64;
            for trunk in &trunks {
                let mut rel = DVec3::from_array(trunk.position) - p;
                rel.y = 0.0;
                let along = rel.dot(forward).clamp(0.0, 18.0);
                let width = (rel - forward * along).length() - f64::from(trunk.scale) * 0.7;
                clearance = clearance.min(width);
            }
            let score = clearance - 0.014 * p.distance(site);
            if score > best.0 {
                best = (score, p);
            }
        }
    }
    best.1
}

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
    time: Res<Time<Real>>,
    (caps, reconstruction, target, river, weather, particles): (
        Res<crate::capabilities::Capabilities>,
        Res<crate::render_quality::Reconstruction>,
        Res<crate::scene::SceneTarget>,
        Query<&Mesh3d, With<crate::vista::RiverSurface>>,
        Res<crate::weather::Weather>,
        Query<(&Mesh3d, &Visibility), With<crate::particles::WeatherParticles>>,
    ),
    mut exit: MessageWriter<AppExit>,
) {
    audit.frames += 1;
    let case = if audit.river_view {
        RIVER_CASE
    } else {
        audit
            .photoreal
            .as_ref()
            .map_or(CASES[audit.case], |fixture| fixture.case)
    };
    let image = format!("worldcraft-biome-{}.png", case.label);
    let frame_limit = if audit.photoreal.is_some() {
        20_000
    } else {
        2400
    };
    assert!(
        audit.frames < frame_limit,
        "Procedural biome route did not finish"
    );
    if let Some(frame) = audit.capture {
        let path = if audit.scanned_comparison {
            "worldcraft-material-savanna-scanned.png".to_owned()
        } else {
            image.clone()
        };
        if audit.frames <= frame + 3 || !std::path::Path::new(&path).is_file() {
            return;
        }
        audit.capture = None;
        if audit.photoreal.is_none()
            && case.label == "savanna"
            && !audit.scanned_comparison
            && crate::terrain_texture::scanned_sources()
        {
            audit.scanned_comparison = true;
            limits.preferences.render.procedural_surfaces = false;
            audit.ready = 0;
            audit.start_draws = evidence.draws();
            return;
        }
        limits.preferences.render.procedural_surfaces = true;
        audit.scanned_comparison = false;
        info!("BIOME AUDIT: captured {}", case.label);
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
    if !audit.initialized {
        limits.preferences.render.procedural_surfaces = true;
        session.create(
            format!("Procedural {} audit", case.label),
            GeneratorSettings::for_preset(case.preset, case.seed),
            &mut player,
        );
        let g = session.generator.as_ref().unwrap();
        let target = if audit.river_view {
            let (eye, aim) = worldcraft_core::river::waterfall_view(g.settings());
            player.position = DVec3::from_array(eye);
            player.position.y = player
                .position
                .y
                .max(g.height(player.position.x, player.position.z) + 2.0);
            aim_at(&mut player, DVec3::from_array(aim));
            DVec3::from_array(aim)
        } else if case.preset == Preset::Estuary && audit.photoreal.is_none() {
            let z = 165.0;
            DVec3::new(
                worldcraft_core::estuary::channel_x(g.settings(), z),
                g.settings().sea_level,
                z,
            )
        } else if audit.photoreal.is_some() {
            photoreal_site(g, case)
        } else {
            site(g, case.biome)
        };
        if let Some(fixture) = audit.photoreal.as_mut() {
            fixture.target = target;
            player.position = target + fixture.offset;
            player.position.y = player
                .position
                .y
                .max(g.height(player.position.x, player.position.z) + 2.0);
            aim_at(&mut player, target);
        } else if !audit.spawn_view && !audit.river_view {
            player.position = if audit.cloud_descent {
                target + DVec3::new(-780.0, 2700.0, 1530.0)
            } else if case.preset == Preset::Estuary {
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
        if std::env::var_os("WORLDCRAFT_AUDIT_4K").is_some() {
            limits.preferences.render.surface_resolution =
                crate::render_options::SurfaceResolution::FourK;
        }
        graphics.quality = Quality::High;
        graphics.radius = if audit.photoreal.is_some()
            && (std::env::var_os("WORLDCRAFT_PHOTOREAL_GPU").is_some()
                || std::env::var_os("WORLDCRAFT_PHOTOREAL_REFERENCE").is_some())
        {
            Graphics::default().radius
        } else {
            2
        };
        graphics.hour = if audit.sun_view { 7.25 } else { case.hour };
        if let Ok(hour) = std::env::var("WORLDCRAFT_AUDIT_HOUR") {
            graphics.hour = hour
                .parse::<f32>()
                .expect("Time of day in hours")
                .clamp(0.0, 23.99);
            if crate::scene::night_visibility(graphics.hour) > 0.9 {
                let moon =
                    -crate::scene::sky_direction(crate::scene::solar_angle(graphics.hour) + 0.30);
                player.yaw = (-moon.x).atan2(-moon.z);
                player.pitch = 0.15;
            }
        }
        if let Ok(label) = std::env::var("WORLDCRAFT_AUDIT_WEATHER") {
            graphics.weather = Some(
                crate::weather::Kind::ALL
                    .into_iter()
                    .find(|kind| label.eq_ignore_ascii_case(kind.label()))
                    .expect("Known weather condition"),
            );
        }
        if audit.sun_view {
            let a = (graphics.hour - 6.0) * std::f32::consts::TAU / 24.0;
            player.yaw = (-a.cos()).atan2(-0.35) - 0.18;
            player.pitch = 0.13;
            player.position = woodland_opening(g, target, player.yaw);
        }
        session.mode = Mode::Fly;
        graphics.breeze = 0.0;
        graphics.cycle = false;
        graphics.clouds = true;
        if audit.cloud_descent {
            graphics.mist_height = 1200.0;
        }
        graphics.volumetrics =
            (case.preset == Preset::Estuary || audit.sun_view || audit.photoreal.is_some())
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
    let route_started = audit
        .photoreal
        .as_ref()
        .is_some_and(|fixture| fixture.start.is_some());
    if audit.photoreal.is_some() && !route_started && audit.frames % 30 == 0 {
        info!(
            "PHOTOREAL AUDIT: frame {} tiles {}/{} queued {} far triangles {} material side {} cloud draws {}",
            audit.frames,
            stream.loaded.len(),
            stream.desired,
            stream.queue(),
            stream.far_triangles,
            active.3,
            evidence.draws() - audit.start_draws,
        );
    }
    if !route_started
        && (stream.loaded.len() != stream.desired
            || stream.queue() != 0
            || stream.far_triangles == 0)
    {
        return;
    }
    audit.ready += 1;
    if audit.ready < 12 || evidence.draws() <= audit.start_draws + 4 {
        return;
    }
    let g = session.generator.as_ref().unwrap();
    let requires_4k = limits.preferences.render.surface_resolution
        == crate::render_options::SurfaceResolution::FourK;
    if active.0 == audit.scanned_comparison
        || (requires_4k && active.3 != if active.2 { 4096 } else { 1024 })
    {
        audit.ready = 0;
        return;
    }
    if audit.scanned_comparison {
        crate::verify::capture(&mut commands, "worldcraft-material-savanna-scanned.png");
        audit.capture = Some(audit.frames);
        return;
    }
    let cloud_descent_frame_ms = if audit.cloud_descent {
        audit.settled_frame_ms.push(time.delta_secs_f64() * 1000.0);
        if audit.settled_frame_ms.len() < 24 {
            return;
        }
        audit.settled_frame_ms.sort_by(f64::total_cmp);
        let samples = &audit.settled_frame_ms;
        let middle = samples.len() / 2;
        let p95 = (samples.len() * 95).div_ceil(100) - 1;
        Some(serde_json::json!({
            "frames": samples.len(),
            "median": (samples[middle - 1] + samples[middle]) * 0.5,
            "p95": samples[p95],
        }))
    } else {
        None
    };
    let photoreal = if let Some(fixture) = audit.photoreal.as_mut() {
        let timing = if fixture.view == "route" {
            let Some(timing) = photoreal_route(fixture, &mut player, g, &time) else {
                return;
            };
            Some(timing)
        } else {
            None
        };
        Some(serde_json::json!({
            "manifest_version": 1,
            "view": fixture.view,
            "site": fixture.target.to_array(),
            "timing": timing,
            "adapter": caps.gpu,
            "backend": caps.backend,
            "software": caps.software,
            "active_upscaler": reconstruction.active,
            "scene_target": target.size.to_array(),
            "view_radius_tiles": limits.radius(graphics.radius),
            "render_options": limits.preferences.render,
        }))
    } else {
        None
    };
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
    let estuary_depth_range =
        if case.preset == Preset::Estuary && !audit.cloud_descent && audit.photoreal.is_none() {
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
    let river_triangles = if audit.river_view {
        let mesh = meshes
            .get(&river.single().expect("One active river mesh").0)
            .unwrap();
        let Some(bevy::mesh::VertexAttributeValues::Float32x2(uv)) =
            mesh.attribute(Mesh::ATTRIBUTE_UV_0)
        else {
            panic!("Missing river flow markers")
        };
        assert!(uv.iter().any(|uv| uv[1] < 0.0), "Missing waterfall section");
        Some(mesh.indices().unwrap().len() / 3)
    } else {
        None
    };
    let cloud_draws = evidence.draws() - audit.start_draws;
    let weather_quads = particles
        .single()
        .ok()
        .filter(|(_, visibility)| **visibility == Visibility::Visible)
        .and_then(|(mesh, _)| meshes.get(&mesh.0))
        .map_or(0, |mesh| mesh.count_vertices() / 4);
    if graphics
        .weather
        .is_some_and(|kind| kind != crate::weather::Kind::Clear)
    {
        assert!(
            weather_quads > 0,
            "Weather preview must have resident particles"
        );
    }
    let sample = g.sample(player.position.x, player.position.z);
    let spawn_view = audit.spawn_view;
    let sun_view = audit.sun_view;
    let cloud_descent = audit.cloud_descent;
    audit.rows.push(serde_json::json!({
        "label":case.label,"ordinary_spawn_camera":spawn_view,"sun_facing_woodland":sun_view,"estuary_depth_range_m":estuary_depth_range,"seed":case.seed,"preset":format!("{:?}",case.preset),
        "cloud_descent":cloud_descent,"mist_height_m":graphics.mist_height,"settled_frame_ms":cloud_descent_frame_ms,
        "procedural_surfaces":active.0,"photographic_materials":active.4,"surface_texture_side":active.3,"volumetrics":graphics.volumetrics, "biome":format!("{:?}",case.biome),"camera":player.position.to_array(),
        "yaw":player.yaw,"pitch":player.pitch,"hour":graphics.hour,
        "weather":weather.kind.label(),"weather_quads":weather_quads,"ground_wetness":weather.wet,
        "fresh_snow":weather.snow,"lightning_strikes":weather.strike,
        "camera_terrain_palette":g.color(player.position.x, player.position.z),
        "camera_climate":[sample.temperature,sample.moisture],
        "camera_biome_weights":g.biome_blend(player.position.x, player.position.z, &sample).weights.to_vec(),
        "resident_chunks":stream.loaded.len(),"queued_jobs":stream.queue(),
        "center_species_counts":species,"resident_mesh_triangles":triangles,
        "far_triangles":stream.far_triangles,"cloud_draw_commands":cloud_draws,
        "image":image,"photoreal":photoreal,"river_triangles":river_triangles,
    }));
    info!(
        "BIOME AUDIT: {} settled; {} triangles; {} cloud draws",
        case.label, triangles, cloud_draws
    );
    crate::verify::capture(&mut commands, &image);
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

#[cfg(test)]
mod photoreal_tests {
    use super::*;

    #[test]
    fn manifest_covers_every_biome_with_a_distinct_exact_site() {
        let manifest = fixtures();
        assert_eq!(manifest.fixtures.len(), Biome::ALL.len());
        assert_eq!(manifest.route_seconds, 60.0);
        for biome in Biome::ALL {
            let mut cases = manifest.fixtures.iter().filter(|case| case.biome == biome);
            let case = cases.next().unwrap();
            assert!(cases.next().is_none());
            assert_eq!(
                manifest
                    .fixtures
                    .iter()
                    .filter(|other| other.label == case.label)
                    .count(),
                1
            );
            let generator = Generator::new(
                GeneratorSettings::for_preset(case.preset, case.seed),
                vec![],
            );
            let site = photoreal_site(&generator, *case);
            assert_eq!(generator.sample(site.x, site.z).biome, biome);
        }
    }
}
