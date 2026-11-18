Windows x64 test prerelease with the latest procedural biomes, botanical foliage,
PBR ground materials and the new render settings panel.

- Real NVIDIA DLSS Super Resolution and DLAA integration through Bevy/NGX.
- Official AMD FSR SDK 2.4.0 / upscaler API 4.1.2 Vulkan integration. The active
  provider is displayed: compatible hardware may use the analytical FSR 3 path.
- Native FXAA, SMAA and TAA; adaptive sharpening; manual output scale; shadow/AO
  controls; bloom; color grading and tone mapping; camera FOV and depth of field;
  bounded cloud/fog sampling; V-sync and restart-selectable graphics API.
- Previous-frame foliage/crown motion for temporal reconstruction, camera-cut
  history resets, exclusive AA ownership and safe unsupported-device fallback.
- Pinned runtime hashes, original SDK notices and native installer/portable gates.

For an RTX card: **Render settings → Display → Vulkan → restart**, then select
**DLSS Quality** or **DLAA**. DLAA operates at the current world-output resolution;
choose 100% output and a manual Performance budget for native window resolution.
The application enables vendor choices only when its initialized device/runtime
reports support. Keep the packaged DLLs beside Worldcraft.exe.

The attached native evidence archive records software Vulkan integration and
untouched biome captures from this exact source. Windows software DX12 checks
cover AA/lens switching, package launch and the saved learned-world example.
These runners cannot certify NVIDIA/AMD gaming-GPU dispatch, image quality,
frame rates or VRAM. Vendor execution is explicitly skipped on unsupported devices.

The **80% realism target remains unmet**. This is an engineering alpha with
experimental vendor paths, ray-query lighting and clouds. Distant forests,
connected rivers/waterfalls, hydraulic erosion, temporal travel review and broader
hardware testing remain unfinished. There is no frame generation or exposed
DLSS Ray Reconstruction mode. The installer is unsigned and per-user.
