//! AMD's signed FSR API integration. No vendor-name-based support inference.
use crate::render_options::UpscaleQuality;
use bevy::{
    core_pipeline::prepass::{DepthPrepass, MotionVectorPrepass},
    prelude::*,
    render::camera::{MipBias, TemporalJitter},
    render::{extract_component::ExtractComponent, RenderApp},
};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex,
};

#[derive(Component, Clone, ExtractComponent)]
#[extract_app(RenderApp)]
#[extract_component_filter(With<Camera>)]
#[require(
    TemporalJitter,
    MipBias,
    DepthPrepass,
    MotionVectorPrepass,
    bevy::camera::Hdr
)]
pub struct FsrUpscaler {
    pub quality: UpscaleQuality,
    pub field_of_view: f32,
    pub reset: bool,
    pub pre_exposure: f32,
    pub frame_ms: f32,
}

pub struct Shared {
    pub available: AtomicBool,
    pub description: Mutex<String>,
    pub dispatches: AtomicU64,
    pub failures: AtomicU64,
    pub revision: AtomicU64,
}
#[derive(Resource, Clone)]
pub struct FsrState(pub Arc<Shared>);
impl Default for FsrState {
    fn default() -> Self {
        Self(Arc::new(Shared { available:AtomicBool::new(false),description:Mutex::new(
            if cfg!(windows) {"Checking the AMD FSR runtime and graphics device…"}
            else {"AMD FSR 4.1's signed runtime requires Windows and Vulkan. Native AA remains available."}.into()),
            dispatches:AtomicU64::new(0),failures:AtomicU64::new(0),revision:AtomicU64::new(0), }))
    }
}
impl FsrState {
    pub fn available(&self) -> bool {
        self.0.available.load(Ordering::Acquire)
    }
    pub fn description(&self) -> String {
        self.0
            .description
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }
    pub fn status(&self, available: bool, message: String) {
        *self.0.description.lock().unwrap_or_else(|p| p.into_inner()) = message;
        self.0.available.store(available, Ordering::Release);
        self.0.revision.fetch_add(1, Ordering::Release);
    }
}

pub struct FsrPlugin;
impl Plugin for FsrPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FsrState>();
        crate::fsr_native::install(app);
    }
    fn finish(&self, app: &mut App) {
        crate::fsr_native::finish(app);
    }
}
