use std::{path::PathBuf, sync::Arc};
use bevy::{math::DVec3, prelude::*};
use worldcraft_core::*;

#[derive(Resource)] pub struct Launch { pub smoke: bool }
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode { #[default] Walk, Fly, Edit }
#[derive(Resource)]
pub struct Player {
    pub position: DVec3,
    pub yaw: f32,
    pub pitch: f32,
    pub velocity: f64,
    pub grounded: bool,
    pub captured: bool,
}
impl Default for Player {
    fn default() -> Self { Self { position: DVec3::new(0.0,100.0,0.0), yaw: 0.0,
        pitch: -0.15, velocity: 0.0, grounded: false, captured: false } }
}
impl Player {
    pub fn rotation(&self) -> Quat { Quat::from_euler(EulerRot::YXZ, self.yaw, self.pitch, 0.0) }
}
#[derive(Resource)]
pub struct Session {
    pub world: Option<WorldFile>,
    pub generator: Option<Arc<Generator>>,
    pub history: EditHistory,
    pub epoch: u64,
    pub revision: u64,
    pub dirty: bool,
    pub path: Option<PathBuf>,
    pub origin: DVec3,
    pub mode: Mode,
}
impl Default for Session {
    fn default() -> Self { Self { world: None, generator: None, history: EditHistory::default(),
        epoch: 0, revision: 0, dirty: false, path: None, origin: DVec3::ZERO, mode: Mode::Walk } }
}
impl Session {
    pub fn open(&mut self, world: WorldFile, path: Option<PathBuf>, player: &mut Player) {
        player.position = DVec3::from_array(world.camera);
        player.yaw = world.yaw; player.pitch = world.pitch; player.velocity = 0.0;
        player.captured = false;
        self.history = EditHistory::from_transactions(world.edits.clone());
        self.generator = Some(Arc::new(Generator::new(world.settings.clone(), self.history.strokes())));
        self.world = Some(world);
        self.path = path; self.dirty = false;
        self.epoch = self.epoch.wrapping_add(1); self.revision = 0;
        self.origin = DVec3::new((player.position.x / 256.0).floor() * 256.0, 0.0,
            (player.position.z / 256.0).floor() * 256.0);
        self.mode = Mode::Walk;
    }
    pub fn create(&mut self, name: String, settings: GeneratorSettings, player: &mut Player) {
        let g = Generator::new(settings.clone(), Vec::new());
        let mut w = WorldFile::new(name, settings);
        w.camera = g.spawn_position();
        self.open(w, None, player);
        self.dirty = true;
    }
    pub fn rebuild(&mut self, live: &[Stroke]) {
        if let Some(w) = &self.world {
            let mut edits = self.history.strokes(); edits.extend_from_slice(live);
            self.generator = Some(Arc::new(Generator::new(w.settings.clone(), edits)));
            self.revision = self.revision.wrapping_add(1);
            self.dirty = true;
        }
    }
    pub fn snapshot(&self, p: &Player) -> Option<WorldFile> {
        self.world.as_ref().map(|w| {
            let mut w = w.clone(); w.edits = self.history.transactions.clone();
            w.camera = p.position.to_array(); w.yaw = p.yaw; w.pitch = p.pitch; w
        })
    }
}
#[derive(Resource)]
pub struct Controls {
    pub brush: Brush,
    pub live: Vec<Stroke>,
    pub hit: Option<[f64; 3]>,
    pub last_dab: Option<[f64; 2]>,
    pub last_time: f64,
    pub speed: f64,
}
impl Default for Controls {
    fn default() -> Self { Self { brush: Brush::default(), live: Vec::new(), hit: None,
        last_dab: None, last_time: 0.0, speed: 30.0 } }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Quality { Low, #[default] Balanced, High }
#[derive(Resource)]
pub struct Graphics {
    pub quality: Quality,
    pub radius: i64,
    pub hour: f32,
    pub cycle: bool,
    pub fog: f32,
    pub exposure: f32,
    pub map: bool,
}
impl Default for Graphics {
    fn default() -> Self { Self { quality: Quality::Balanced, radius: 6,
        hour: 9.0, cycle: false, fog: 1.0, exposure: 0.0, map: false } }
}
#[derive(Resource)]
pub struct UiState {
    pub home: bool,
    pub create: bool,
    pub advanced: bool,
    pub name: String,
    pub seed: String,
    pub settings: GeneratorSettings,
    pub message: String,
    pub help: bool,
    pub inspector: bool,
    pub map_layer: usize,
    pub map_zoom: f64,
    pub recents: Vec<PathBuf>,
}
impl Default for UiState {
    fn default() -> Self { Self { home: true, create: false, advanced: false,
        name: "My wilderness".into(), seed: "20261116".into(),
        settings: GeneratorSettings::default(), message: String::new(),
        help: false, inspector: true, map_layer: 0, map_zoom: 1200.0, recents: Vec::new() } }
}
