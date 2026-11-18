//! Minimal C ABI for the official AMD FSR SDK 2.4 / upscaler 4.1.2.
//! Descriptors follow AMD's MIT-licensed headers, not an unofficial shader port.
//! Original headers, provenance and license are retained under vendor/fsr-api.
use std::ffi::{c_char, c_void};

pub const CREATE_UPSCALE: u64 = 0x10000;
pub const DISPATCH_UPSCALE: u64 = 0x10001;
pub const CREATE_VK: u64 = 0x01000003;
pub const API_VERSION: u64 = 0x1000b;
pub const GET_VERSIONS: u64 = 4;
pub const GET_PROVIDER: u64 = 6;
pub const UPSCALE_VERSION: u32 = (4 << 22) | (1 << 12) | 2;
pub const FLAGS: u32 = (1 << 0) | (1 << 3) | (1 << 4) | (1 << 5);

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Header {
    pub kind: u64,
    pub next: *mut Header,
}
impl Header {
    pub fn new(kind: u64) -> Self {
        Self {
            kind,
            next: std::ptr::null_mut(),
        }
    }
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
}
impl From<[u32; 2]> for Dimensions {
    fn from(v: [u32; 2]) -> Self {
        Self {
            width: v[0],
            height: v[1],
        }
    }
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Float2 {
    pub x: f32,
    pub y: f32,
}
impl From<[f32; 2]> for Float2 {
    fn from(v: [f32; 2]) -> Self {
        Self { x: v[0], y: v[1] }
    }
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct DeviceContext {
    pub device: u64,
    pub physical_device: u64,
    pub get_device_proc_addr: *const c_void,
}
#[repr(C)]
pub struct Backend {
    pub header: Header,
    pub device: DeviceContext,
}
#[repr(C)]
pub struct Version {
    pub header: Header,
    pub version: u32,
}
#[repr(C)]
pub struct Create {
    pub header: Header,
    pub flags: u32,
    pub max_render: Dimensions,
    pub max_output: Dimensions,
    pub message: Option<unsafe extern "C" fn(u32, *const u16)>,
}
#[repr(C)]
pub struct Versions {
    pub header: Header,
    pub create_type: u64,
    pub device: *mut c_void,
    pub count: *mut u64,
    pub ids: *mut u64,
    pub names: *mut *const c_char,
}
#[repr(C)]
pub struct Provider {
    pub header: Header,
    pub id: u64,
    pub name: *const c_char,
}
#[repr(C)]
pub struct Resolution {
    pub header: Header,
    pub display_width: u32,
    pub display_height: u32,
    pub quality: u32,
    pub width: *mut u32,
    pub height: *mut u32,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ResourceDescription {
    pub kind: u32,
    pub format: u32,
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub mips: u32,
    pub flags: u32,
    pub usage: u32,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Resource {
    pub resource: *mut c_void,
    pub description: ResourceDescription,
    pub state: u32,
}
#[repr(C)]
pub struct Dispatch {
    pub header: Header,
    pub command_list: *mut c_void,
    pub color: Resource,
    pub depth: Resource,
    pub motion: Resource,
    pub exposure: Resource,
    pub reactive: Resource,
    pub transparency: Resource,
    pub output: Resource,
    pub jitter: Float2,
    pub motion_scale: Float2,
    pub render_size: Dimensions,
    pub output_size: Dimensions,
    pub enable_sharpening: bool,
    pub sharpness: f32,
    pub frame_ms: f32,
    pub pre_exposure: f32,
    pub reset: bool,
    pub near: f32,
    pub far: f32,
    pub vertical_fov: f32,
    pub metres: f32,
    pub flags: u32,
}
pub type CreateContext = unsafe extern "C" fn(*mut *mut c_void, *mut Header, *const c_void) -> u32;
pub type DestroyContext = unsafe extern "C" fn(*mut *mut c_void, *const c_void) -> u32;
pub type Query = unsafe extern "C" fn(*mut *mut c_void, *mut Header) -> u32;
pub type DispatchContext = unsafe extern "C" fn(*mut *mut c_void, *const Header) -> u32;

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{align_of, offset_of, size_of};
    #[test]
    fn abi_matches_the_official_amd_x64_c_descriptors() {
        // Values are also checked by tools/ci/check_fsr_abi.cpp compiled against
        // the retained upstream headers, independently of these Rust definitions.
        assert_eq!(size_of::<Header>(), 16);
        assert_eq!(size_of::<DeviceContext>(), 24);
        assert_eq!(size_of::<Backend>(), 40);
        assert_eq!(size_of::<Create>(), 48);
        assert_eq!(size_of::<Versions>(), 56);
        assert_eq!(size_of::<Provider>(), 32);
        assert_eq!(size_of::<ResourceDescription>(), 32);
        assert_eq!(size_of::<Resource>(), 48);
        assert_eq!(size_of::<Dispatch>(), 432);
        assert_eq!(align_of::<Dispatch>(), 8);
        assert_eq!(offset_of!(Dispatch, color), 24);
        assert_eq!(offset_of!(Dispatch, output), 312);
        assert_eq!(offset_of!(Dispatch, jitter), 360);
        assert_eq!(offset_of!(Dispatch, reset), 408);
        assert_eq!(offset_of!(Dispatch, near), 412);
        assert_eq!(offset_of!(Dispatch, flags), 428);
    }
}
