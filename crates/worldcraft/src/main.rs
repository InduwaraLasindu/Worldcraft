#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
mod assets;
mod calibration;
mod capabilities;
mod clouds;
mod control;
mod export;
mod foliage;
mod installation;
mod models;
mod nature_audit;
mod nature_texture;
mod performance;
mod raytrace;
mod scene;
mod session;
mod storage;
mod streaming;
mod surface;
mod terrain_texture;
mod tree;
mod ui;
mod verify;
mod vista;

use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, prelude::*, window::PresentMode};
use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};
use session::*;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(at) = args.iter().position(|a| a == "--validate-install") {
        let Some(report) = args.get(at + 1) else {
            std::process::exit(2);
        };
        std::process::exit(installation::run(std::path::Path::new(report)));
    }
    let ray_smoke = std::env::args().any(|a| a == "--ray-smoke-test");
    let nature_smoke = args.iter().any(|a| a == "--nature-smoke-test");
    let auto_smoke = args.iter().any(|a| a == "--auto-smoke-test");
    let eco_smoke = args.iter().any(|a| a == "--eco-smoke-test");
    let detailed_smoke = args.iter().any(|a| a == "--detailed-smoke-test");
    let learned_smoke = auto_smoke
        || eco_smoke
        || detailed_smoke
        || args.iter().any(|a| a == "--learned-smoke-test");
    let smoke =
        nature_smoke || ray_smoke || learned_smoke || std::env::args().any(|a| a == "--smoke-test");
    let mut limits = performance::Performance::load(smoke && !eco_smoke && !auto_smoke);
    if eco_smoke || args.iter().any(|a| a == "--eco") {
        limits.preferences.mode = performance::Mode::Eco;
        limits.calibrating = false;
        limits.calibration_status = "Manual Eco selected; no launch calibration.".into();
    }
    if auto_smoke {
        limits.preferences = performance::Preferences::default();
    }
    if smoke {
        limits.calibrating = false;
    }
    let pools = limits.task_pools();
    let mut app = App::new();
    app.insert_resource(limits);
    app.insert_resource(Launch { smoke, ray_smoke })
        .insert_resource(ClearColor(Color::srgb(0.10, 0.18, 0.24)))
        .insert_resource(GlobalAmbientLight {
            color: Color::srgb(0.65, 0.78, 1.0),
            brightness: 280.0,
            ..default()
        })
        .init_resource::<calibration::Probe>()
        .init_resource::<Session>()
        .init_resource::<Player>()
        .init_resource::<Controls>()
        .init_resource::<UiState>()
        .init_resource::<Graphics>()
        .init_resource::<storage::Disk>()
        .init_resource::<streaming::Stream>()
        .init_resource::<vista::Vista>()
        .init_resource::<raytrace::RayState>()
        .init_resource::<models::Models>()
        .init_resource::<export::Exporter>()
        .add_plugins(
            DefaultPlugins
                .set(bevy::app::TaskPoolPlugin {
                    task_pool_options: pools,
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: if ray_smoke {
                            "Worldcraft — Ray audit"
                        } else {
                            "Worldcraft — Explore your world"
                        }
                        .into(),
                        resolution: if ray_smoke {
                            (128_u32, 96_u32).into()
                        } else if args
                            .iter()
                            .any(|a| a == "--idle-audit" || a == "--calibration-audit")
                        {
                            (640_u32, 360_u32).into()
                        } else if nature_smoke {
                            if cfg!(target_os = "windows") {
                                (640_u32, 360_u32).into()
                            } else {
                                (960_u32, 640_u32).into()
                            }
                        } else if smoke {
                            if cfg!(target_os = "windows") {
                                if detailed_smoke {
                                    (640_u32, 360_u32).into()
                                } else {
                                    (400_u32, 300_u32).into()
                                }
                            } else {
                                (800_u32, 600_u32).into()
                            }
                        } else {
                            (1440_u32, 900_u32).into()
                        },
                        position: if ray_smoke {
                            bevy::window::WindowPosition::At(IVec2::new(900, 0))
                        } else {
                            bevy::window::WindowPosition::Automatic
                        },
                        present_mode: PresentMode::AutoVsync,
                        ..default()
                    }),
                    close_when_requested: false,
                    ..default()
                }),
        )
        .add_plugins((EguiPlugin::default(), FrameTimeDiagnosticsPlugin::default()))
        .insert_resource(bevy_egui::EguiGlobalSettings {
            auto_create_primary_context: false,
            ..default()
        })
        .add_systems(First, performance::frame_start)
        .add_systems(Last, performance::pace)
        .add_plugins((surface::SurfacePlugin, bevy::solari::prelude::SolariPlugins))
        .insert_resource(bevy::pbr::DefaultOpaqueRendererMethod::forward())
        .add_systems(
            Startup,
            (
                capabilities::setup,
                assets::setup,
                scene::setup,
                clouds::setup,
                calibration::setup,
            )
                .chain(),
        )
        .add_systems(EguiPrimaryContextPass, (ui::style, ui::draw).chain())
        .add_systems(
            Update,
            (
                export::poll,
                models::poll,
                storage::poll,
                performance::update,
                calibration::update,
                scene::resolution,
                control::input,
                streaming::update,
                foliage::update_ranges,
                vista::update,
                raytrace::update,
                scene::update,
                clouds::update,
                surface::update,
                storage::close,
            )
                .chain(),
        );
    if args.iter().any(|a| a == "--calibration-audit") {
        app.add_systems(Update, verify::calibration_check.after(calibration::update));
    }
    if nature_smoke {
        app.init_resource::<nature_audit::Audit>()
            .add_systems(Startup, nature_audit::start.after(scene::setup))
            .add_systems(
                Update,
                (nature_audit::rebase, nature_audit::check)
                    .chain()
                    .after(surface::update),
            );
    } else if ray_smoke {
        app.add_plugins(bevy::render::diagnostic::RenderDiagnosticsPlugin)
            .init_resource::<raytrace::QaRaySmoke>()
            .add_systems(Startup, raytrace::qa_setup.after(scene::setup))
            .add_systems(Update, raytrace::qa_check.after(raytrace::update));
    } else if learned_smoke {
        app.init_resource::<verify::LearnedSmoke>()
            .add_systems(Startup, verify::learned_start)
            .add_systems(Update, verify::learned_check);
    } else if smoke {
        app.init_resource::<verify::Smoke>()
            .add_systems(Startup, verify::start)
            .add_systems(Update, verify::check);
    }
    if smoke {
        app.insert_resource(bevy::winit::WinitSettings::continuous());
    }
    app.run();
}
