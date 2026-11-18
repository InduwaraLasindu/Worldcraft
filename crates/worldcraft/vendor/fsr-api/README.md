AMD FSR SDK 2.4.0 C ABI headers, unchanged from official commit 7539ae8b6568ad29ed78a753404351daf3b5cca7.

Source: https://github.com/GPUOpen-LibrariesAndSDKs/FidelityFX-SDK/tree/7539ae8b6568ad29ed78a753404351daf3b5cca7/Kits/FidelityFX

The original api/include and upscalers/include hierarchy is preserved. The C++ ABI
check includes both the portable API/upscaler headers and the Vulkan backend header.
Upstream MIT notices remain in every file. The full SDK and third-party notices are
in licenses/AMD-FSR-*.md. The application calls the signed loader/upscaler binaries;
no unofficial FSR 4 shader port or model is used.
