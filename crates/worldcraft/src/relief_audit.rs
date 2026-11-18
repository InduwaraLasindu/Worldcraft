//! Fixed views of an untouched seeded woodland. All surfaces come from the normal streamer.
use crate::{
    render_options::{AntiAliasing, EffectQuality, SurfaceRelief, SurfaceResolution, Upscaler},
    session::*,
    streaming::Stream,
    terrain_texture::ActiveSurfaces,
};
use bevy::{app::AppExit, math::DVec3, prelude::*};
use worldcraft_core::{
    scatter::{scatter, Species},
    Biome, ChunkCoord, GeneratorSettings, Preset,
};

const CASES: [(&str, SurfaceRelief); 9] = [
    ("rock-flat", SurfaceRelief::Off),
    ("rock-repeat", SurfaceRelief::Off),
    ("rock-pom", SurfaceRelief::Ultra),
    ("rock-travel", SurfaceRelief::Ultra),
    ("floor-flat", SurfaceRelief::Off),
    ("floor-pom", SurfaceRelief::Ultra),
    ("rock-1k", SurfaceRelief::Ultra),
    ("coast-sunset", SurfaceRelief::Ultra),
    ("snow-line", SurfaceRelief::Ultra),
];
#[derive(Resource, Default)]
pub struct Audit {
    frame: u32,
    case: usize,
    ready: u32,
    captured: Option<u32>,
    rock: DVec3,
    scale: f64,
    rows: Vec<serde_json::Value>,
}

pub fn start(
    mut audit: ResMut<Audit>,
    mut session: ResMut<Session>,
    mut player: ResMut<Player>,
    mut ui: ResMut<UiState>,
    mut graphics: ResMut<Graphics>,
    mut limits: ResMut<crate::performance::Performance>,
) {
    session.create(
        "Material relief audit".into(),
        GeneratorSettings::for_preset(Preset::Temperate, 42),
        &mut player,
    );
    let g = session.generator.as_ref().unwrap();
    let site = crate::biome_audit::site(g, Biome::Forest);
    let rock = scatter(g, ChunkCoord::at(site.x, site.z), 0)
        .into_iter()
        .filter(|i| i.species == Species::Rock && g.slope(i.position[0], i.position[2]) < 0.4)
        .max_by(|a, b| a.scale.total_cmp(&b.scale))
        .expect("Seeded woodland boulder");
    audit.rock = DVec3::from_array(rock.position);
    audit.scale = rock.scale as f64;
    session.mode = Mode::Fly;
    ui.home = false;
    ui.inspector = false;
    ui.photo = true;
    graphics.quality = Quality::Balanced;
    graphics.radius = 2;
    graphics.hour = 9.0;
    graphics.breeze = 0.0;
    graphics.cycle = false;
    graphics.clouds = false;
    graphics.volumetrics = false;
    let settings = &mut limits.preferences.render;
    settings.upscaler = Upscaler::Native;
    settings.anti_aliasing = AntiAliasing::Off;
    settings.bloom = false;
    settings.ambient_occlusion = EffectQuality::Off;
    settings.surface_resolution = SurfaceResolution::FourK;
    settings.surface_relief = SurfaceRelief::Off;
    place(&audit, &session, &mut player);
}

fn place(a: &Audit, s: &Session, p: &mut Player) {
    let floor = (4..=5).contains(&a.case);
    let side = if a.case == 3 { 0.38 } else { 0.0 };
    let target = if floor {
        a.rock + DVec3::new(2.1, 0.0, 3.6)
    } else {
        a.rock + DVec3::Y * (0.25 * a.scale)
    };
    let g = s.generator.as_ref().unwrap();
    let target = if floor {
        DVec3::new(target.x, g.height(target.x, target.z), target.z)
    } else {
        target
    };
    p.position = if floor {
        target + DVec3::new(0.32, 0.62, 0.70)
    } else {
        a.rock + DVec3::new(0.29 * a.scale + side, 0.83 * a.scale, 1.37 * a.scale)
    };
    p.position.y = p
        .position
        .y
        .max(g.height(p.position.x, p.position.z) + 0.55);
    let aim = target - p.position;
    p.yaw = (-aim.x).atan2(-aim.z) as f32;
    p.pitch = aim.y.atan2(aim.x.hypot(aim.z)) as f32;
}
/// Find actual water and snow from the terrain generator. The same scene path
/// remains available to a player with these two seeds.
fn landscape(coast: bool, session: &mut Session, p: &mut Player, graphics: &mut Graphics) {
    let (preset, seed) = if coast {
        (Preset::Islands, 20261116)
    } else {
        (Preset::Alpine, 324)
    };
    let settings = GeneratorSettings::for_preset(preset, seed);
    session.create("Landscape material audit".into(), settings.clone(), p);
    let g = session.generator.as_ref().unwrap();
    graphics.quality = Quality::High;
    graphics.clouds = true;
    graphics.hour = if coast { 17.53 } else { 13.4 };
    session.mode = Mode::Fly;
    if coast {
        use worldcraft_core::generation::unit;
        let cx = (0.15 + unit(seed ^ 151, 0, 0) * 0.7) * 4200.0 * settings.terrain_scale;
        let cz = (0.15 + unit(seed ^ 163, 0, 0) * 0.7) * 4200.0 * settings.terrain_scale;
        let sun = DVec3::new(-1.0, 0.0, 0.35).normalize();
        let point = |t: f64| DVec3::new(cx, 0.0, cz) + sun * t;
        let height = |t| {
            let v = point(t);
            g.height(v.x, v.z)
        };
        let edge = (1..850)
            .map(|i| i as f64 * 8.0)
            .find(|&t| {
                height(t - 8.0) >= settings.sea_level
                    && height(t) < settings.sea_level
                    && height(t + 48.0) < settings.sea_level
            })
            .expect("Archipelago sunset coastline");
        p.position = point(edge - 15.0);
        p.position.y = height(edge - 15.0).max(settings.sea_level) + 2.2;
        p.yaw = (-sun.x).atan2(-sun.z) as f32;
        p.pitch = 0.033;
    } else {
        // Seed 324 has a chain of snow-covered peaks east of spawn. Use an
        // actual summit and verify its biome so a camera cannot silently land
        // in a wooded foothill when the terrain changes.
        let (x, z) = (3465.0, 203.0);
        let summit = g.sample(x, z);
        assert!(
            summit.biome == Biome::Snow && g.biome_blend(x, z, &summit).weight(Biome::Snow) > 0.85
        );
        let peak = DVec3::new(x, summit.height, z);
        let (cx, cz) = (x + 275.0, z + 210.0);
        let eye = DVec3::new(cx, (g.height(cx, cz) + 14.0).max(peak.y - 105.0), cz);
        p.position = eye;
        let aim = peak - eye;
        p.yaw = (-aim.x).atan2(-aim.z) as f32;
        p.pitch = aim.y.atan2(aim.x.hypot(aim.z)) as f32 - 0.05;
    }
}
pub fn check(
    mut commands: Commands,
    mut audit: ResMut<Audit>,
    mut player: ResMut<Player>,
    mut session: ResMut<Session>,
    mut graphics: ResMut<Graphics>,
    stream: Res<Stream>,
    active: Res<ActiveSurfaces>,
    mut limits: ResMut<crate::performance::Performance>,
    #[cfg(target_os = "windows")] target: Res<crate::scene::SceneTarget>,
    mut exit: MessageWriter<AppExit>,
) {
    audit.frame += 1;
    assert!(audit.frame < 1400, "Material route failed to settle");
    let (label, pom) = CASES[audit.case];
    let filename = format!("worldcraft-relief-{label}.png");
    if let Some(frame) = audit.captured {
        if audit.frame < frame + 5 || !std::path::Path::new(&filename).is_file() {
            return;
        }
        audit.captured = None;
        audit.ready = 0;
        audit.case += 1;
        if audit.case == CASES.len() || (cfg!(target_os = "windows") && audit.case == 3) {
            let report = serde_json::json!({"status":"passed","source":std::env::var("GITHUB_SHA").unwrap_or_else(|_|"local".into()),"seed":42,"authored_instances":0,"rock":audit.rock.to_array(),"cases":audit.rows});
            std::fs::write(
                "relief-render-audit.json",
                serde_json::to_vec_pretty(&report).unwrap(),
            )
            .unwrap();
            info!("RELIEF AUDIT: captured fixed-camera procedural surface comparisons");
            exit.write(AppExit::Success);
            return;
        }
        let (_, next) = CASES[audit.case];
        limits.preferences.render.surface_relief = next;
        limits.preferences.render.surface_resolution = if audit.case >= 6 {
            SurfaceResolution::OneK
        } else {
            SurfaceResolution::FourK
        };
        if audit.case >= 7 {
            limits.preferences.render.cloud_coverage = 0.73;
            landscape(audit.case == 7, &mut session, &mut player, &mut graphics);
        } else {
            place(&audit, &session, &mut player);
        }
        return;
    }
    if stream.loaded.len() != stream.desired || stream.queue() != 0 || stream.far_triangles == 0 {
        return;
    }
    audit.ready += 1;
    if audit.ready < 10 {
        return;
    }
    assert!(active.0);
    assert_eq!(
        active.1,
        if active.2 && audit.case < 6 {
            4096
        } else {
            1024
        }
    );
    assert_eq!(limits.preferences.render.surface_relief, pom);
    let snow_weight = (label == "snow-line").then(|| {
        let g = session.generator.as_ref().unwrap();
        let s = g.sample(3465.0, 203.0);
        g.biome_blend(3465.0, 203.0, &s).weight(Biome::Snow)
    });
    audit.rows.push(serde_json::json!({"summit_snow_weight":snow_weight,"label":label,"image":filename,"side":active.1,"bc":active.2,"pom_layers":pom.layers(),"seed":session.world.as_ref().unwrap().settings.seed,"hour":graphics.hour,"camera":player.position.to_array(),"yaw":player.yaw,"pitch":player.pitch}));
    #[cfg(not(target_os = "windows"))]
    crate::verify::capture(&mut commands, &filename);
    #[cfg(target_os = "windows")]
    commands
        .spawn(bevy::render::view::screenshot::Screenshot::image(
            target.image.clone(),
        ))
        .observe(bevy::render::view::screenshot::save_to_disk(filename));
    audit.captured = Some(audit.frame);
}
