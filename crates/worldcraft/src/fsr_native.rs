//! Native Vulkan interop with AMD's signed FSR SDK, including its analytical fallback.
//! Compiled on Linux to type-check the same wgpu/HAL integration used on Windows.
use crate::{
    fsr::{FsrState, FsrUpscaler},
    fsr_ffi as ffi,
    render_options::UpscaleQuality,
};
use ash::vk::{self, Handle};
use bevy::{
    camera::{CameraMainTextureUsages, MainPassResolutionOverride},
    core_pipeline::{
        prepass::ViewPrepassTextures,
        schedule::{Core3d, Core3dSystems},
    },
    diagnostic::FrameCount,
    math::Vec4Swizzles,
    prelude::*,
    render::{
        camera::{MipBias, TemporalJitter},
        extract_component::ExtractComponentPlugin,
        render_resource::{PollType, TextureUsages},
        renderer::{RenderContext, RenderDevice, ViewQuery},
        view::{prepare_view_targets, ExtractedView, ViewTarget},
        Render, RenderApp, RenderSystems,
    },
};
use std::{
    ffi::{c_void, CStr},
    io::Read,
    path::Path,
    sync::{atomic::Ordering, Arc, Mutex},
};
use wgpu::hal::api::Vulkan;

// These two new Vulkan feature structures are not yet named in ash 0.38.
// Their ABI/type IDs match Khronos vulkan_core.h. AMD SDK 2.4's native sample
// enables them alongside cooperative matrices for its ML/INT8 kernels.
#[repr(C)]
struct ExtraFeature {
    kind: vk::StructureType,
    next: *mut c_void,
    bits: [vk::Bool32; 4],
}
// Feature records are only mutated during device initialization under the
// retained mutex. vkCreateDevice consumes the chain before rendering begins.
unsafe impl Send for ExtraFeature {}

pub fn configure_device(app: &mut App) {
    if !cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        return;
    }
    use bevy::render::renderer::raw_vulkan_init::RawVulkanInitSettings;
    let retained = Arc::new(Mutex::new(Vec::<Box<ExtraFeature>>::new()));
    let mut settings = app
        .world_mut()
        .get_resource_or_init::<RawVulkanInitSettings>();
    // SAFETY: only supported feature bits/extensions are added; existing wgpu
    // requirements stay enabled. Boxes remain alive after the callback returns.
    unsafe {
        settings.add_create_device_callback(move |args, adapter, _| {
            let instance = adapter.shared_instance().raw_instance();
            let physical = adapter.raw_physical_device();
            let Ok(extensions) = instance.enumerate_device_extension_properties(physical) else {
                return;
            };
            let mut retained = retained.lock().unwrap_or_else(|p| p.into_inner());
            for (name, kind, bit_count) in [
                (
                    c"VK_KHR_cooperative_matrix",
                    vk::StructureType::PHYSICAL_DEVICE_COOPERATIVE_MATRIX_FEATURES_KHR,
                    2,
                ),
                (
                    c"VK_EXT_shader_float8",
                    vk::StructureType::from_raw(1000567000),
                    2,
                ),
                (
                    c"VK_KHR_compute_shader_derivatives",
                    vk::StructureType::from_raw(1000201000),
                    2,
                ),
                (
                    c"VK_KHR_shader_integer_dot_product",
                    vk::StructureType::PHYSICAL_DEVICE_SHADER_INTEGER_DOT_PRODUCT_FEATURES,
                    1,
                ),
                (
                    c"VK_KHR_shader_float16_int8",
                    vk::StructureType::PHYSICAL_DEVICE_SHADER_FLOAT16_INT8_FEATURES,
                    2,
                ),
                (
                    c"VK_KHR_vulkan_memory_model",
                    vk::StructureType::PHYSICAL_DEVICE_VULKAN_MEMORY_MODEL_FEATURES,
                    3,
                ),
                (
                    c"VK_EXT_subgroup_size_control",
                    vk::StructureType::PHYSICAL_DEVICE_SUBGROUP_SIZE_CONTROL_FEATURES,
                    2,
                ),
                (
                    c"VK_KHR_16bit_storage",
                    vk::StructureType::PHYSICAL_DEVICE_16BIT_STORAGE_FEATURES,
                    4,
                ),
                (
                    c"VK_KHR_8bit_storage",
                    vk::StructureType::PHYSICAL_DEVICE_8BIT_STORAGE_FEATURES,
                    3,
                ),
            ] {
                if retained.len() >= 64 {
                    break;
                }
                if !extensions
                    .iter()
                    .any(|e| CStr::from_ptr(e.extension_name.as_ptr()) == name)
                {
                    continue;
                }
                let mut feature = Box::new(ExtraFeature {
                    kind,
                    next: std::ptr::null_mut(),
                    bits: [0; 4],
                });
                let mut query = vk::PhysicalDeviceFeatures2 {
                    p_next: std::ptr::from_mut(&mut *feature).cast(),
                    ..default()
                };
                instance.get_physical_device_features2(physical, &mut query);
                if feature.bits[..bit_count].iter().all(|b| *b == 0) {
                    continue;
                }
                if !args.extensions.contains(&name) {
                    args.extensions.push(name);
                }
                let mut existing = args.create_info.p_next as *mut vk::BaseOutStructure<'_>;
                let mut found = false;
                // Extension chains are short. The cap also avoids following a bad
                // third-party chain forever during an optional enhancement.
                for _ in 0..128 {
                    if existing.is_null() {
                        break;
                    }
                    if (*existing).s_type == kind {
                        // Touch only the bool fields of the exact Vulkan type;
                        // padding in one-/two-bit structures is never read.
                        let bits = existing
                            .cast::<u8>()
                            .add(std::mem::offset_of!(ExtraFeature, bits))
                            .cast::<vk::Bool32>();
                        for (i, bit) in feature.bits[..bit_count].iter().enumerate() {
                            *bits.add(i) |= *bit;
                        }
                        found = true;
                        break;
                    }
                    existing = (*existing).p_next;
                }
                if !found {
                    feature.next = args.create_info.p_next as *mut c_void;
                    args.create_info.p_next = std::ptr::from_mut(&mut *feature).cast();
                    retained.push(feature);
                }
            }
        });
    }
}

const RUNTIME_FILES: [(&str, u64, &str); 2] = [
    (
        "amd_fidelityfx_loader_vk.dll",
        39184,
        "38a56a01fb3565c0757d9f5c1e4e69de4b65e5f2e5769b323b200dc1f142bab9",
    ),
    (
        "amd_fidelityfx_upscaler_vk.dll",
        36135696,
        "4a7e895af6e0b615d40a84d59eb8c2b1054ad91cdd0c739567940270d51bdab7",
    ),
];

#[derive(Resource)]
struct Runtime(Arc<Api>);
struct Api {
    _loader: libloading::Library,
    _upscaler: libloading::Library,
    create: ffi::CreateContext,
    destroy: ffi::DestroyContext,
    query: ffi::Query,
    dispatch: ffi::DispatchContext,
    device: wgpu::Device,
    vk: ffi::DeviceContext,
}
// The API function table and device handles are immutable. Actual FFX contexts
// are used on the render thread behind a mutex; no borrowed SDK descriptor moves.
unsafe impl Send for Api {}
unsafe impl Sync for Api {}

#[derive(Component)]
struct Prepared {
    context: Mutex<Context>,
    output: UVec2,
    input: UVec2,
    quality: UpscaleQuality,
}
struct Context {
    api: Arc<Api>,
    handle: *mut c_void,
    // AMD requires descriptor pointers to remain valid until context destruction.
    _create: Box<ffi::Create>,
    _backend: Box<ffi::Backend>,
    _version: Box<ffi::Version>,
    fresh: bool,
}
// Moved only inside Prepared's mutex. SDK calls and destruction are serialized;
// the owning wgpu device and both libraries outlive every context and dispatch.
unsafe impl Send for Context {}
impl Drop for Context {
    fn drop(&mut self) {
        if self.handle.is_null() {
            return;
        }
        // Wait only when retiring a native context (resize/switch/shutdown), not
        // every hardware frame. Native scratch resources are invisible to wgpu.
        if self.api.device.poll(PollType::wait_indefinitely()).is_ok() {
            unsafe {
                (self.api.destroy)(&mut self.handle, std::ptr::null());
            }
        }
    }
}

fn verify(path: &Path, size: u64, digest: &str) -> Result<(), String> {
    use sha2::{Digest, Sha256};
    let mut file = std::fs::File::open(path).map_err(|_| {
        format!(
            "Missing {}",
            path.file_name().unwrap_or_default().to_string_lossy()
        )
    })?;
    if file.metadata().map_err(|e| e.to_string())?.len() != size {
        return Err("AMD runtime size verification failed".into());
    }
    let mut hash = Sha256::new();
    let mut block = [0u8; 65536];
    loop {
        let n = file.read(&mut block).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hash.update(&block[..n]);
    }
    if format!("{:x}", hash.finalize()) != digest {
        return Err("AMD runtime SHA-256 verification failed".into());
    }
    Ok(())
}
impl Api {
    fn load(device: wgpu::Device) -> Result<(Arc<Self>, String), String> {
        let base = std::env::current_exe()
            .map_err(|e| e.to_string())?
            .parent()
            .ok_or("No executable directory")?
            .to_path_buf();
        for (name, size, hash) in RUNTIME_FILES {
            verify(&base.join(name), size, hash)?;
        }
        let vk_context = {
            // SAFETY: read-only handle access. The returned device is retained
            // for the entire API/context lifetime; no wgpu object is destroyed.
            let d = unsafe { device.as_hal::<Vulkan>() }.ok_or(
                "AMD FSR requires the Vulkan graphics API; restart after selecting Vulkan",
            )?;
            ffi::DeviceContext {
                device: d.raw_device().handle().as_raw(),
                physical_device: d.raw_physical_device().as_raw(),
                get_device_proc_addr: d
                    .shared_instance()
                    .raw_instance()
                    .fp_v1_0()
                    .get_device_proc_addr as *const c_void,
            }
        };
        // SAFETY: absolute paths in the executable directory, both size and
        // SHA-256 verified against the official AMD SDK 2.4 commit. Preload the
        // effect DLL so the loader reuses this exact module, never a cwd copy.
        let upscaler = unsafe { libloading::Library::new(base.join(RUNTIME_FILES[1].0)) }
            .map_err(|e| e.to_string())?;
        let loader = unsafe { libloading::Library::new(base.join(RUNTIME_FILES[0].0)) }
            .map_err(|e| e.to_string())?;
        let api = unsafe {
            Self {
                create: *loader
                    .get::<ffi::CreateContext>(b"ffxCreateContext\0")
                    .map_err(|e| e.to_string())?,
                destroy: *loader
                    .get::<ffi::DestroyContext>(b"ffxDestroyContext\0")
                    .map_err(|e| e.to_string())?,
                query: *loader
                    .get::<ffi::Query>(b"ffxQuery\0")
                    .map_err(|e| e.to_string())?,
                dispatch: *loader
                    .get::<ffi::DispatchContext>(b"ffxDispatch\0")
                    .map_err(|e| e.to_string())?,
                _loader: loader,
                _upscaler: upscaler,
                device,
                vk: vk_context,
            }
        };
        let api = Arc::new(api);
        let mut count = 0u64;
        let mut vk = api.vk;
        let mut versions = ffi::Versions {
            header: ffi::Header::new(ffi::GET_VERSIONS),
            create_type: ffi::CREATE_UPSCALE,
            device: std::ptr::from_mut(&mut vk).cast(),
            count: &mut count,
            ids: std::ptr::null_mut(),
            names: std::ptr::null_mut(),
        };
        let result = unsafe { (api.query)(std::ptr::null_mut(), &mut versions.header) };
        if result != 0 || count == 0 || count > 32 {
            return Err(format!(
                "No supported FSR provider for this initialized device (code {result})"
            ));
        }
        let mut names = vec![std::ptr::null(); count as usize];
        let mut ids = vec![0u64; count as usize];
        versions.names = names.as_mut_ptr();
        versions.ids = ids.as_mut_ptr();
        let result = unsafe { (api.query)(std::ptr::null_mut(), &mut versions.header) };
        if result != 0 || count as usize > names.len() {
            return Err(format!("FSR provider enumeration failed ({result})"));
        }
        let names = names
            .into_iter()
            .take(count as usize)
            .filter(|p| !p.is_null())
            .map(|p| {
                unsafe { CStr::from_ptr(p) }
                    .to_string_lossy()
                    .chars()
                    .take(128)
                    .collect::<String>()
            })
            .collect::<Vec<_>>();
        Ok((api,format!("AMD FSR available · {}. The runtime selects ML 4.1 or its supported analytical provider.",names.join(", "))))
    }

    fn context(
        self: &Arc<Self>,
        output: UVec2,
        quality: UpscaleQuality,
    ) -> Result<(Context, UVec2, String), String> {
        if output.min_element() == 0 || u64::from(output.x) * u64::from(output.y) > 33_554_432 {
            return Err("FSR output exceeds the bounded reconstruction target".into());
        }
        let maximum = UVec2::from_array(quality.maximum_input(output.to_array()));
        let mut version = Box::new(ffi::Version {
            header: ffi::Header::new(ffi::API_VERSION),
            version: ffi::UPSCALE_VERSION,
        });
        let mut backend = Box::new(ffi::Backend {
            header: ffi::Header::new(ffi::CREATE_VK),
            device: self.vk,
        });
        backend.header.next = &mut version.header;
        let mut create = Box::new(ffi::Create {
            header: ffi::Header::new(ffi::CREATE_UPSCALE),
            flags: ffi::FLAGS,
            max_render: maximum.to_array().into(),
            max_output: output.to_array().into(),
            message: None,
        });
        create.header.next = &mut backend.header;
        let mut handle = std::ptr::null_mut();
        let result = unsafe { (self.create)(&mut handle, &mut create.header, std::ptr::null()) };
        if result != 0 || handle.is_null() {
            return Err(format!(
                "FSR context creation failed ({result}); native rendering retained"
            ));
        }
        let mut context = Context {
            api: Arc::clone(self),
            handle,
            _create: create,
            _backend: backend,
            _version: version,
            fresh: true,
        };
        let mut width = 0;
        let mut height = 0;
        let mut resolution = ffi::Resolution {
            header: ffi::Header::new(0x10003),
            display_width: output.x,
            display_height: output.y,
            quality: match quality {
                UpscaleQuality::Quality => 1,
                UpscaleQuality::Balanced => 2,
                UpscaleQuality::Performance => 3,
                UpscaleQuality::UltraPerformance => 4,
            },
            width: &mut width,
            height: &mut height,
        };
        let result = unsafe { (self.query)(&mut context.handle, &mut resolution.header) };
        if result != 0 || width == 0 || height == 0 || width > maximum.x || height > maximum.y {
            return Err(format!("FSR returned invalid input dimensions ({result})"));
        }
        let mut provider = ffi::Provider {
            header: ffi::Header::new(ffi::GET_PROVIDER),
            id: 0,
            name: std::ptr::null(),
        };
        let result = unsafe { (self.query)(&mut context.handle, &mut provider.header) };
        if result != 0 || provider.name.is_null() {
            return Err(format!("FSR active provider query failed ({result})"));
        }
        let name = unsafe { CStr::from_ptr(provider.name) }
            .to_string_lossy()
            .chars()
            .take(128)
            .collect::<String>();
        Ok((
            context,
            UVec2::new(width, height),
            format!("AMD FSR active · {name} · {}", quality.label()),
        ))
    }
}

pub fn install(app: &mut App) {
    app.add_plugins(ExtractComponentPlugin::<FsrUpscaler>::default());
    let state = app.world().resource::<FsrState>().clone();
    if let Some(render) = app.get_sub_app_mut(RenderApp) {
        render.insert_resource(state).add_systems(
            Render,
            prepare
                .in_set(RenderSystems::PrepareViews)
                .before(prepare_view_targets),
        );
        render.add_systems(Core3d, dispatch.in_set(Core3dSystems::EarlyPostProcess));
    }
}
pub fn finish(app: &mut App) {
    if !cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        return;
    }
    let state = app.world().resource::<FsrState>().clone();
    let Some(render) = app.get_sub_app_mut(RenderApp) else {
        return;
    };
    let Some(device) = render.world().get_resource::<RenderDevice>() else {
        return;
    };
    match Api::load(device.wgpu_device().clone()) {
        Ok((runtime, message)) => {
            state.status(true, message);
            render.insert_resource(Runtime(runtime));
        }
        Err(message) => {
            info!("FSR unavailable: {message}");
            state.status(false, message);
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn prepare(
    mut commands: Commands,
    runtime: Option<Res<Runtime>>,
    state: Res<FsrState>,
    frame: Res<FrameCount>,
    mut views: Query<(
        Entity,
        &ExtractedView,
        &FsrUpscaler,
        &mut Camera3d,
        &mut CameraMainTextureUsages,
        &mut TemporalJitter,
        &mut MipBias,
        Option<&Prepared>,
    )>,
    retired: Query<Entity, (With<Prepared>, Without<FsrUpscaler>)>,
    #[cfg(feature = "nvidia-dlss")] dlss: Query<(), With<bevy::anti_alias::dlss::Dlss>>,
) {
    for e in &retired {
        commands.entity(e).remove::<Prepared>();
        #[cfg(feature = "nvidia-dlss")]
        let other_owns_resolution = dlss.get(e).is_ok();
        #[cfg(not(feature = "nvidia-dlss"))]
        let other_owns_resolution = false;
        if !other_owns_resolution {
            commands.entity(e).remove::<MainPassResolutionOverride>();
        }
    }
    let Some(runtime) = runtime else {
        return;
    };
    if !state.available() {
        return;
    }
    for (entity, view, settings, mut camera, mut usages, mut jitter, mut bias, prepared) in
        &mut views
    {
        let output = view.viewport.zw();
        usages.0 |= TextureUsages::STORAGE_BINDING;
        camera.depth_texture_usages = (TextureUsages::from(camera.depth_texture_usages)
            | TextureUsages::TEXTURE_BINDING)
            .into();
        let input = if let Some(p) =
            prepared.filter(|p| p.output == output && p.quality == settings.quality)
        {
            p.input
        } else {
            match runtime.0.context(output, settings.quality) {
                Ok((context, input, message)) => {
                    state.status(true, message);
                    commands.entity(entity).insert((
                        Prepared {
                            context: Mutex::new(context),
                            output,
                            input,
                            quality: settings.quality,
                        },
                        MainPassResolutionOverride(input),
                    ));
                    input
                }
                Err(message) => {
                    warn!("FSR fallback: {message}");
                    state.status(false, message);
                    state.0.failures.fetch_add(1, Ordering::Relaxed);
                    commands
                        .entity(entity)
                        .remove::<(Prepared, MainPassResolutionOverride)>();
                    continue;
                }
            }
        };
        jitter.offset = jitter_offset(frame.0, input.x, output.x);
        bias.0 = (input.x as f32 / output.x.max(1) as f32).log2() - 1.0;
    }
}
pub fn jitter_offset(frame: u32, input_width: u32, output_width: u32) -> Vec2 {
    fn halton(mut index: u32, base: u32) -> f32 {
        let mut f = 1.0;
        let mut r = 0.0;
        while index > 0 {
            f /= base as f32;
            r += f * (index % base) as f32;
            index /= base;
        }
        r
    }
    let ratio = output_width.max(1) as f32 / input_width.max(1) as f32;
    let phases = (8.0 * ratio * ratio).ceil().clamp(8.0, 128.0) as u32;
    let index = frame % phases + 1;
    Vec2::new(halton(index, 2) - 0.5, halton(index, 3) - 0.5)
}

fn resource(view: &wgpu::TextureView, output: bool) -> Result<ffi::Resource, String> {
    let texture = view.texture();
    let format = match texture.format() {
        wgpu::TextureFormat::Rgba16Float => 4,
        wgpu::TextureFormat::Rg16Float => 18,
        wgpu::TextureFormat::Depth32Float | wgpu::TextureFormat::R32Float => 28,
        _ => return Err("Unsupported FSR input/output texture format".into()),
    };
    let depth = texture.format() == wgpu::TextureFormat::Depth32Float;
    let handle = unsafe { texture.as_hal::<Vulkan>() }.ok_or("FSR requires Vulkan textures")?;
    Ok(ffi::Resource {
        resource: unsafe { handle.raw_handle() }.as_raw() as usize as *mut c_void,
        description: ffi::ResourceDescription {
            kind: 2,
            format,
            width: texture.width(),
            height: texture.height(),
            depth: 1,
            mips: texture.mip_level_count(),
            flags: 0,
            usage: if output {
                2
            } else if depth {
                4
            } else {
                0
            },
        },
        state: if output { 2 } else { 4 },
    })
}

fn dispatch(
    view: ViewQuery<(
        &FsrUpscaler,
        &Prepared,
        &TemporalJitter,
        &ViewTarget,
        &ViewPrepassTextures,
    )>,
    state: Res<FsrState>,
    mut ctx: RenderContext,
) {
    let (settings, prepared, jitter, target, prepass) = view.into_inner();
    if !state.available() {
        return;
    }
    let (Some(depth), Some(motion)) = (&prepass.depth, &prepass.motion_vectors) else {
        return;
    };
    let output = target.post_process_write();
    let sources: [&wgpu::TextureView; 3] = [
        output.source,
        &depth.texture.default_view,
        &motion.texture.default_view,
    ];
    let mut context = prepared.context.lock().unwrap_or_else(|p| p.into_inner());
    let parameters = (|| -> Result<ffi::Dispatch, String> {
        Ok(ffi::Dispatch {
            header: ffi::Header::new(ffi::DISPATCH_UPSCALE),
            command_list: std::ptr::null_mut(),
            color: resource(sources[0], false)?,
            depth: resource(sources[1], false)?,
            motion: resource(sources[2], false)?,
            exposure: ffi::Resource::default(),
            reactive: ffi::Resource::default(),
            transparency: ffi::Resource::default(),
            output: resource(output.destination, true)?,
            jitter: (-jitter.offset).to_array().into(),
            motion_scale: (-prepared.input.as_vec2()).to_array().into(),
            render_size: prepared.input.to_array().into(),
            output_size: prepared.output.to_array().into(),
            enable_sharpening: false,
            sharpness: 0.0,
            frame_ms: settings.frame_ms.clamp(0.01, 1000.0),
            pre_exposure: settings.pre_exposure.max(f32::MIN_POSITIVE),
            reset: settings.reset || context.fresh,
            near: 0.1,
            far: f32::INFINITY,
            vertical_fov: settings.field_of_view,
            metres: 1.0,
            flags: 0,
        })
    })();
    let outcome = parameters.and_then(|mut parameters| {
        // wgpu owns all external images. Transition its tracked usage before
        // recording on a separate native encoder, exactly as in Bevy's NGX path.
        ctx.command_encoder().transition_resources(
            std::iter::empty(),
            sources
                .iter()
                .map(|v| wgpu::TextureTransition {
                    texture: v.texture(),
                    selector: None,
                    state: wgpu::TextureUses::RESOURCE,
                })
                .chain(std::iter::once(wgpu::TextureTransition {
                    texture: output.destination.texture(),
                    selector: None,
                    state: wgpu::TextureUses::STORAGE_READ_WRITE,
                })),
        );
        let mut encoder =
            context
                .api
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Worldcraft AMD FSR reconstruction"),
                });
        let api = Arc::clone(&context.api);
        let handle = &mut context.handle;
        let result = unsafe {
            encoder.as_hal_mut::<Vulkan, _, _>(|raw| {
                let Some(raw) = raw else {
                    return Err("FSR command encoder has no Vulkan backend".to_string());
                };
                let command = raw.raw_handle();
                parameters.command_list = command.as_raw() as usize as *mut c_void;
                let native = api
                    .device
                    .as_hal::<Vulkan>()
                    .ok_or("FSR device backend disappeared")?;
                let image = depth
                    .texture
                    .texture
                    .as_hal::<Vulkan>()
                    .ok_or("FSR depth backend disappeared")?
                    .raw_handle();
                // wgpu uses DEPTH_STENCIL_READ_ONLY_OPTIMAL for sampled depth;
                // FFX's COMPUTE_READ contract uses SHADER_READ_ONLY_OPTIMAL. Return
                // to wgpu's layout after dispatch so its tracker remains truthful.
                depth_layout(
                    native.raw_device(),
                    command,
                    image,
                    vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
                    vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                );
                let code = (api.dispatch)(handle, &parameters.header);
                depth_layout(
                    native.raw_device(),
                    command,
                    image,
                    vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                    vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
                );
                if code == 0 {
                    Ok(())
                } else {
                    Err(format!("AMD FSR dispatch failed ({code})"))
                }
            })
        };
        result?;
        ctx.add_command_buffer(encoder.finish());
        context.fresh = false;
        state.0.dispatches.fetch_add(1, Ordering::Relaxed);
        Ok(())
    });
    if let Err(error) = outcome {
        // The failed native encoder is not submitted. Restore the original HDR
        // ping-pong side; next frame removes the input-resolution override.
        let _ = target.post_process_write();
        state.status(false, error.clone());
        state.0.failures.fetch_add(1, Ordering::Relaxed);
        error!("FSR dispatch failed; using native fallback: {error}");
    }
}
unsafe fn depth_layout(
    device: &ash::Device,
    command: vk::CommandBuffer,
    image: vk::Image,
    old: vk::ImageLayout,
    new: vk::ImageLayout,
) {
    let barrier = vk::ImageMemoryBarrier::default()
        .old_layout(old)
        .new_layout(new)
        .image(image)
        .src_access_mask(vk::AccessFlags::MEMORY_READ)
        .dst_access_mask(vk::AccessFlags::SHADER_READ)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::DEPTH,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        });
    // SAFETY: caller owns an open Vulkan command buffer and a live depth image;
    // only layout is changed, with matching restoration before returning to wgpu.
    unsafe {
        device.cmd_pipeline_barrier(
            command,
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[barrier],
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn subpixel_jitter_is_bounded_periodic_and_safe_at_tiny_resolutions() {
        for (input, output) in [(1280, 1920), (1, 1), (640, 1920)] {
            for frame in 0..256 {
                let v = jitter_offset(frame, input, output);
                assert!(v.is_finite());
                assert!(v.min_element() >= -0.5 && v.max_element() <= 0.5);
            }
            assert_ne!(
                jitter_offset(0, input, output),
                jitter_offset(1, input, output)
            );
        }
        assert!(jitter_offset(u32::MAX, 0, 0).is_finite());
    }
}
