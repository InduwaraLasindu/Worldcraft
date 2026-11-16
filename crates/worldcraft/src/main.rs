#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
mod assets;
mod capabilities;
mod control;
mod export;
mod installation;
mod models;
mod raytrace;
mod scene;
mod session;
mod storage;
mod streaming;
mod surface;
mod ui;
mod verify;
mod vista;

use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, prelude::*, window::PresentMode};
use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};
use session::*;

fn main() {
    let args:Vec<String>=std::env::args().collect();
    if let Some(at)=args.iter().position(|a|a=="--validate-install") {
        let Some(report)=args.get(at+1) else{std::process::exit(2);};
        std::process::exit(installation::run(std::path::Path::new(report)));
    }
    let ray_smoke = std::env::args().any(|a| a == "--ray-smoke-test");
    let learned_smoke = std::env::args().any(|a| a == "--learned-smoke-test");
    let smoke = ray_smoke || learned_smoke || std::env::args().any(|a| a == "--smoke-test");
    let mut app = App::new();
    app.insert_resource(Launch { smoke, ray_smoke })
        .insert_resource(ClearColor(Color::srgb(0.10, 0.18, 0.24)))
        .insert_resource(GlobalAmbientLight {
            color: Color::srgb(0.65, 0.78, 1.0),
            brightness: 280.0,
            ..default()
        })
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
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: if ray_smoke {
                        "Worldcraft — Ray audit"
                    } else {
                        "Worldcraft — Explore your world"
                    }
                    .into(),
                    resolution: if ray_smoke {
                        (128_u32, 96_u32).into()
                    } else if smoke {
                        (800_u32, 600_u32).into()
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
        .add_plugins((surface::SurfacePlugin, bevy::solari::prelude::SolariPlugins))
        .insert_resource(bevy::pbr::DefaultOpaqueRendererMethod::forward())
        .add_systems(
            Startup,
            (capabilities::setup, assets::setup, scene::setup).chain(),
        )
        .add_systems(EguiPrimaryContextPass, (ui::style, ui::draw).chain())
        .add_systems(
            Update,
            (
                export::poll,
                models::poll,
                storage::poll,
                control::input,
                streaming::update,
                vista::update,
                raytrace::update,
                scene::update,
                surface::update,
                storage::close,
            )
                .chain(),
        );
    if ray_smoke {
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
    if smoke {app.insert_resource(bevy::winit::WinitSettings::continuous());}
    app.run();
}
