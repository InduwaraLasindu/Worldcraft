#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
mod assets;
mod control;
mod scene;
mod session;
mod storage;
mod streaming;
mod surface;
mod ui;
mod verify;

use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, prelude::*, window::PresentMode};
use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};
use session::*;

fn main() {
    let smoke = std::env::args().any(|a| a == "--smoke-test");
    let mut app = App::new();
    app.insert_resource(Launch { smoke })
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
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Worldcraft — Explore your world".into(),
                resolution: if smoke {
                    (800_u32, 600_u32).into()
                } else {
                    (1440_u32, 900_u32).into()
                },
                present_mode: PresentMode::AutoVsync,
                ..default()
            }),
            close_when_requested: false,
            ..default()
        }))
        .add_plugins((EguiPlugin::default(), FrameTimeDiagnosticsPlugin::default()))
        .add_plugins((surface::SurfacePlugin, bevy::solari::prelude::SolariPlugins))
        .insert_resource(bevy::pbr::DefaultOpaqueRendererMethod::forward())
        .add_systems(Startup, (assets::setup, scene::setup).chain())
        .add_systems(EguiContextPass, (ui::style, ui::draw).chain())
        .add_systems(
            Update,
            (
                storage::poll,
                control::input,
                streaming::update,
                scene::update,
                surface::update,
                storage::close,
            )
                .chain(),
        );
    if smoke {
        app.init_resource::<verify::Smoke>()
            .add_systems(Startup, verify::start)
            .add_systems(Update, verify::check);
    }
    app.run();
}
