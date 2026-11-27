//! Machine-local renderer choices. None participate in world generation or saves.
use serde::{Deserialize, Serialize};

// Explicit defaults keep old preference files valid when options are added.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Upscaler {
    #[default]
    Auto,
    Native,
    Dlss,
    Dlaa,
    Fsr41,
}
impl Upscaler {
    pub const ALL: &'static [Self] = &[
        Self::Auto,
        Self::Native,
        Self::Dlss,
        Self::Dlaa,
        Self::Fsr41,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Automatic",
            Self::Native => "Native / manual resolution",
            Self::Dlss => "NVIDIA DLSS Super Resolution",
            Self::Dlaa => "NVIDIA DLAA",
            Self::Fsr41 => "AMD FSR 4.1 / compatibility",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurfaceResolution {
    #[default]
    Automatic,
    OneK,
    TwoK,
    FourK,
}
impl SurfaceResolution {
    pub const ALL: [Self; 4] = [Self::Automatic, Self::OneK, Self::TwoK, Self::FourK];
    pub fn label(self) -> &'static str {
        match self {
            Self::Automatic => "Automatic",
            Self::OneK => "1024",
            Self::TwoK => "2048",
            Self::FourK => "4096 (Ultra)",
        }
    }
    pub fn side(self, high: bool, software: bool) -> u32 {
        match self {
            Self::Automatic if high && !software => 2048,
            Self::Automatic | Self::OneK => 1024,
            Self::TwoK => 2048,
            Self::FourK => 4096,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurfaceRelief {
    Off,
    #[default]
    Balanced,
    Ultra,
}
impl SurfaceRelief {
    pub fn layers(self) -> u32 {
        match self {
            Self::Off => 0,
            Self::Balanced => 12,
            Self::Ultra => 24,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpscaleQuality {
    #[default]
    Quality,
    Balanced,
    Performance,
    UltraPerformance,
}
impl UpscaleQuality {
    pub const ALL: &'static [Self] = &[
        Self::Quality,
        Self::Balanced,
        Self::Performance,
        Self::UltraPerformance,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Quality => "Quality",
            Self::Balanced => "Balanced",
            Self::Performance => "Performance",
            Self::UltraPerformance => "Ultra performance",
        }
    }
    /// Allocation ceiling only; the vendor runtime supplies exact dimensions.
    pub fn maximum_input(self, output: [u32; 2]) -> [u32; 2] {
        output.map(|n| ((n as f32 / self.ratio()).ceil() as u32).max(1))
    }
    pub fn ratio(self) -> f32 {
        match self {
            Self::Quality => 1.5,
            Self::Balanced => 1.7,
            Self::Performance => 2.0,
            Self::UltraPerformance => 3.0,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AntiAliasing {
    Off,
    #[default]
    Fxaa,
    Smaa,
    Taa,
}
impl AntiAliasing {
    pub const ALL: &'static [Self] = &[Self::Off, Self::Fxaa, Self::Smaa, Self::Taa];
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Fxaa => "FXAA (fast)",
            Self::Smaa => "SMAA (sharp edges)",
            Self::Taa => "TAA (temporal stability)",
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectQuality {
    #[default]
    Automatic,
    Off,
    Low,
    Medium,
    High,
}
impl EffectQuality {
    pub const ALL: &'static [Self] = &[
        Self::Automatic,
        Self::Off,
        Self::Low,
        Self::Medium,
        Self::High,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Automatic => "Follow graphics quality",
            Self::Off => "Off",
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
        }
    }
    pub fn shadow_size(self, high: bool, low: bool, software: bool) -> usize {
        let size = match self {
            Self::Automatic => {
                if low {
                    0
                } else if high {
                    4096
                } else {
                    2048
                }
            }
            Self::Off => 0,
            Self::Low => 1024,
            Self::Medium => 2048,
            Self::High => 4096,
        };
        if software {
            size.min(1024)
        } else {
            size
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToneMap {
    #[default]
    AgX,
    TonyMcMapface,
    Aces,
    Reinhard,
}
impl ToneMap {
    pub const ALL: &'static [Self] = &[Self::AgX, Self::TonyMcMapface, Self::Aces, Self::Reinhard];
    pub fn label(self) -> &'static str {
        match self {
            Self::AgX => "AgX",
            Self::TonyMcMapface => "Tony McMapface",
            Self::Aces => "ACES fitted",
            Self::Reinhard => "Reinhard",
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Backend {
    #[default]
    Automatic,
    Vulkan,
    Dx12,
}
impl Backend {
    pub const ALL: &'static [Self] = &[Self::Automatic, Self::Vulkan, Self::Dx12];
    pub fn label(self) -> &'static str {
        match self {
            Self::Automatic => "Automatic",
            Self::Vulkan => "Vulkan",
            Self::Dx12 => "Direct3D 12 (Windows)",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RenderOptions {
    pub upscaler: Upscaler,
    pub upscale_quality: UpscaleQuality,
    pub anti_aliasing: AntiAliasing,
    pub resolution_scale: f32,
    pub sharpening: f32,
    pub shadows: EffectQuality,
    pub shadow_distance: f32,
    pub ambient_occlusion: EffectQuality,
    pub ao_radius: f32,
    pub bloom: bool,
    pub bloom_strength: f32,
    pub tonemap: ToneMap,
    pub saturation: f32,
    pub contrast: f32,
    pub field_of_view: f32,
    pub depth_of_field: bool,
    pub focus_distance: f32,
    pub aperture: f32,
    pub procedural_surfaces: bool,
    pub photographic_materials: bool,
    pub surface_resolution: SurfaceResolution,
    pub surface_relief: SurfaceRelief,
    pub cloud_steps: u32,
    pub cloud_coverage: f32,
    pub fog_steps: u32,
    pub vsync: bool,
    pub backend: Backend,
}
impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            upscaler: Upscaler::Auto,
            upscale_quality: UpscaleQuality::Quality,
            anti_aliasing: AntiAliasing::Fxaa,
            resolution_scale: 1.0,
            sharpening: 0.0,
            shadows: EffectQuality::Automatic,
            shadow_distance: 320.0,
            ambient_occlusion: EffectQuality::Automatic,
            ao_radius: 0.5,
            bloom: true,
            bloom_strength: 0.15,
            tonemap: ToneMap::AgX,
            saturation: 1.0,
            contrast: 1.0,
            field_of_view: 70.0,
            depth_of_field: false,
            focus_distance: 40.0,
            aperture: 5.6,
            procedural_surfaces: true,
            photographic_materials: true,
            surface_resolution: SurfaceResolution::default(),
            surface_relief: SurfaceRelief::default(),
            cloud_steps: 32,
            cloud_coverage: 0.5,
            fog_steps: 24,
            vsync: true,
            backend: Backend::Automatic,
        }
    }
}
impl RenderOptions {
    pub fn sanitize(&mut self) {
        fn bounded(v: &mut f32, min: f32, max: f32, fallback: f32) {
            *v = if v.is_finite() {
                v.clamp(min, max)
            } else {
                fallback
            };
        }
        let d = Self::default();
        bounded(&mut self.resolution_scale, 0.5, 1.0, d.resolution_scale);
        bounded(&mut self.sharpening, 0.0, 1.0, d.sharpening);
        bounded(&mut self.shadow_distance, 64.0, 768.0, d.shadow_distance);
        bounded(&mut self.ao_radius, 0.1, 2.0, d.ao_radius);
        bounded(&mut self.bloom_strength, 0.0, 0.3, d.bloom_strength);
        bounded(&mut self.saturation, 0.0, 1.5, d.saturation);
        bounded(&mut self.contrast, 0.5, 1.5, d.contrast);
        bounded(&mut self.field_of_view, 45.0, 110.0, d.field_of_view);
        bounded(&mut self.focus_distance, 0.5, 10000.0, d.focus_distance);
        bounded(&mut self.aperture, 1.4, 22.0, d.aperture);
        bounded(&mut self.cloud_coverage, 0.0, 1.0, d.cloud_coverage);
        self.cloud_steps = self.cloud_steps.clamp(16, 48);
        self.fog_steps = self.fog_steps.clamp(12, 48);
    }
    pub fn shadow_resolution(self, high: bool, low: bool, software: bool) -> usize {
        self.shadows.shadow_size(high, low, software)
    }
}

/// A capability check must precede insertion of any vendor component. A failed
/// requested method retains native rendering and its selected fallback AA.
pub fn resolve(request: Upscaler, dlss: bool, fsr: bool) -> Upscaler {
    match request {
        Upscaler::Auto => {
            if dlss {
                Upscaler::Dlss
            } else if fsr {
                Upscaler::Fsr41
            } else {
                Upscaler::Native
            }
        }
        Upscaler::Dlss | Upscaler::Dlaa if !dlss => Upscaler::Native,
        Upscaler::Fsr41 if !fsr => Upscaler::Native,
        _ => request,
    }
}

/// Bevy uses `backends: None` to disable rendering entirely. Automatic must
/// retain its defaults, including WGPU_BACKEND overrides used by native QA.
pub fn renderer_settings(backend: Backend) -> bevy::render::settings::WgpuSettings {
    use bevy::render::settings::{Backends, WgpuSettings};
    let mut settings = WgpuSettings::default();
    match backend {
        Backend::Vulkan => settings.backends = Some(Backends::VULKAN),
        Backend::Dx12 if cfg!(windows) => settings.backends = Some(Backends::DX12),
        _ => {}
    }
    // Bevy's raw-Vulkan defaults omit this flag, but wgpu's DX12 backend
    // needs the validation pass for correct indirect draw preparation.
    if cfg!(windows)
        && settings
            .backends
            .is_some_and(|b| b.contains(Backends::DX12))
    {
        settings
            .instance_flags
            .insert(wgpu::InstanceFlags::VALIDATION_INDIRECT_CALL);
    }
    settings
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn automatic_api_retains_a_renderer_and_explicit_api_is_respected() {
        use bevy::render::settings::{Backends, WgpuSettings};
        assert!(renderer_settings(Backend::Automatic).backends.is_some());
        assert_eq!(
            renderer_settings(Backend::Automatic).backends,
            WgpuSettings::default().backends
        );
        assert_eq!(
            renderer_settings(Backend::Vulkan).backends,
            Some(Backends::VULKAN)
        );
        if cfg!(windows) {
            assert_eq!(
                renderer_settings(Backend::Dx12).backends,
                Some(Backends::DX12)
            );
            assert!(renderer_settings(Backend::Dx12)
                .instance_flags
                .contains(wgpu::InstanceFlags::VALIDATION_INDIRECT_CALL));
        } else {
            assert_eq!(
                renderer_settings(Backend::Dx12).backends,
                WgpuSettings::default().backends
            );
        }
    }
    #[test]
    fn old_preferences_round_trip_and_invalid_numbers_cannot_allocate_huge_targets() {
        let mut options: RenderOptions =
            serde_json::from_str("{\"resolution_scale\":20.0,\"cloud_steps\":4294967295}").unwrap();
        options.sharpening = f32::NAN;
        options.shadow_distance = f32::INFINITY;
        options.sanitize();
        assert_eq!(options.resolution_scale, 1.0);
        assert_eq!(options.cloud_steps, 48);
        assert_eq!(
            options.shadow_distance,
            RenderOptions::default().shadow_distance
        );
        assert_eq!(options.sharpening, 0.0);
        assert_eq!(
            options,
            serde_json::from_str::<RenderOptions>(&serde_json::to_string(&options).unwrap())
                .unwrap()
        );
        assert_eq!(
            serde_json::from_str::<RenderOptions>("{}").unwrap(),
            RenderOptions::default()
        );
    }
    #[test]
    fn unsupported_vendor_modes_keep_a_working_native_path() {
        for request in Upscaler::ALL {
            assert_eq!(resolve(*request, false, false), Upscaler::Native);
        }
        assert_eq!(resolve(Upscaler::Dlaa, true, false), Upscaler::Dlaa);
        assert_eq!(resolve(Upscaler::Auto, true, true), Upscaler::Dlss);
        assert_eq!(resolve(Upscaler::Auto, false, true), Upscaler::Fsr41);
    }
    #[test]
    fn reconstruction_allocations_and_software_shadows_remain_bounded() {
        for q in UpscaleQuality::ALL {
            let o = RenderOptions {
                upscale_quality: *q,
                ..Default::default()
            };
            assert!(o.upscale_quality.maximum_input([1920, 1080])[0] < 1920);
            assert_eq!(o.upscale_quality.maximum_input([1, 1]), [1, 1]);
            assert_eq!(o.upscale_quality.maximum_input([0, 0]), [1, 1]);
            for s in EffectQuality::ALL {
                assert!(s.shadow_size(true, false, true) <= 1024);
            }
        }
    }
}
