//! Report the actual initialized adapter; never infer ray tracing from a vendor name.
use bevy::{
    prelude::*,
    render::renderer::{RenderAdapterInfo, RenderDevice},
    solari::prelude::SolariPlugins,
};
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
    limits.weak_gpu = matches!(
        format!("{:?}", adapter.device_type).as_str(),
        "Cpu" | "IntegratedGpu"
    );
    let ray_api = device
        .features()
        .contains(SolariPlugins::required_wgpu_features());
    let software = format!("{:?}", adapter.device_type) == "Cpu";
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
