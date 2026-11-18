# Bevy renderer compatibility patch

Original crates.io bevy_render 0.20.1 sources (without registry metadata or the
unused crate-local lockfile), except the explicit API guard in
src/renderer/raw_vulkan_init.rs. Upstream licenses and normalized manifest are
retained. No renderer, shader, feature or device limits are replaced.

The raw Vulkan path otherwise tries Vulkan even when InstanceDescriptor.backends
excludes it. This guard uses ordinary wgpu initialization for explicit DX12/other
API selections. Vulkan choices retain Bevy's original extension/device callbacks,
including actual DLSS and AMD feature negotiation.

Upstream repository: https://github.com/bevyengine/bevy
Upstream commit: a0577b5c6a7b208719dd761c5c6a7a539e472a97
Crate SHA-256: 0c74591faf8ff1b84beb59e93f05b233a03c62d1cbc446874bd2231b72dde184

The Windows native DX12 and Linux native Vulkan gates remain required. Remove
this patch when an upstream Bevy release includes the equivalent selection guard.
