//! Report the actual initialized adapter; never infer ray tracing from a vendor name.
use bevy::{
    prelude::*,
    render::{
        batching::gpu_preprocessing::{GpuPreprocessingMode, GpuPreprocessingSupport},
        renderer::{RenderAdapterInfo, RenderDevice},
        RenderApp, RenderStartup,
    },
    solari::prelude::SolariPlugins,
};

/// Must finish after RenderPlugin initializes the actual adapter and before
/// PbrPlugin's mesh plugins select their extraction/preparation systems.
pub struct SoftwareRasterPlugin;
fn force_cpu_preparation(render_app: &mut bevy::app::SubApp) {
    render_app
        .remove_systems_in_set(
            RenderStartup,
            bevy::render::init_gpu_resource::<GpuPreprocessingSupport>,
            bevy::ecs::schedule::ScheduleCleanupPolicy::RemoveSystemsOnly,
        )
        .expect("GPU preparation initialization schedule");
    render_app.insert_resource(GpuPreprocessingSupport {
        max_supported_mode: GpuPreprocessingMode::None,
    });
}
impl Plugin for SoftwareRasterPlugin {
    fn build(&self, _app: &mut App) {}
    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        let Some(info) = render_app.world().get_resource::<RenderAdapterInfo>() else {
            return;
        };
        if format!("{:?}", info.device_type) == "Cpu" {
            // The generic GPU-resource initializer would restore advertised
            // compute/culling support at RenderStartup. Remove only that
            // initializer so the CPU extraction/batching choice stays coherent.
            force_cpu_preparation(render_app);
            info!("GRAPHICS: software adapter uses bounded CPU mesh preparation; GPU/ray compute remains available");
        }
    }
}

#[cfg(test)]
mod preparation_tests {
    use super::*;
    #[test]
    fn software_cpu_choice_survives_render_startup_and_recovery() {
        let mut render = bevy::app::SubApp::new();
        render.add_schedule(Schedule::new(RenderStartup));
        // If this initializer survives, running the schedule will try to use
        // nonexistent GPU resources. The actual fallback must remove it, not
        // just write a temporary capability value before startup overwrites it.
        render.add_systems(
            RenderStartup,
            bevy::render::init_gpu_resource::<GpuPreprocessingSupport>,
        );
        force_cpu_preparation(&mut render);
        for _ in 0..2 {
            render.world_mut().run_schedule(RenderStartup);
            assert!(!render
                .world()
                .resource::<GpuPreprocessingSupport>()
                .is_available());
        }
    }
}
#[derive(Resource, Clone, Debug)]
pub struct Capabilities {
    pub gpu: String,
    pub backend: String,
    pub ray_api: bool,
    pub hardware_rays: bool,
    pub software: bool,
    pub cpu_threads: usize,
    pub ray_reason: String,
}
pub fn setup(
    mut commands: Commands,
    device: Res<RenderDevice>,
    adapter: Res<RenderAdapterInfo>,
    mut limits: ResMut<crate::performance::Performance>,
) {
    let ray_api = device
        .features()
        .contains(SolariPlugins::required_wgpu_features());
    let software = format!("{:?}", adapter.device_type) == "Cpu";
    limits.software_gpu = software;
    if software {
        limits.controller.tier = 0;
        limits.controller.ceiling = 0;
    } else if worldcraft_core::hardware::recommend_eco(limits.cpu_threads, limits.memory) {
        limits.controller.ceiling = 2;
    }
    let hardware_rays = ray_api && !software;
    let ray_reason = if hardware_rays {
        "Hardware ray queries are available".into()
    } else if ray_api {
        "Software ray queries are available for validation; no hardware ray-tracing adapter".into()
    } else {
        format!(
            "Required ray-query/bindless features are unavailable: {:?}",
            SolariPlugins::required_wgpu_features().difference(device.features())
        )
    };
    let caps = Capabilities {
        gpu: adapter.name.clone(),
        backend: format!("{:?}", adapter.backend),
        ray_api,
        hardware_rays,
        software,
        cpu_threads: std::thread::available_parallelism().map_or(1, usize::from),
        ray_reason,
    };
    info!(
        "GRAPHICS: {} / {} / ray API={} / hardware={}",
        caps.gpu, caps.backend, caps.ray_api, caps.hardware_rays
    );
    commands.insert_resource(caps);
}
