// Independent check against unchanged upstream AMD headers, not Rust's copies.
#ifndef _WIN32
#define __declspec(x)
#endif
#include <stddef.h>
#include "ffx_api.h"
#include "ffx_api_types.h"
#include "ffx_upscale.h"
#include "vk/ffx_api_vk.h"
static_assert(FFX_API_SURFACE_FORMAT_R16G16B16A16_FLOAT == 4, "HDR format changed");
static_assert(FFX_API_SURFACE_FORMAT_R16G16_FLOAT == 18, "motion format changed");
static_assert(FFX_API_SURFACE_FORMAT_R32_FLOAT == 28, "depth format changed");
static_assert(FFX_API_RESOURCE_TYPE_TEXTURE2D == 2, "texture type changed");
static_assert(FFX_API_RESOURCE_STATE_COMPUTE_READ == 4, "input state changed");
static_assert(FFX_API_RESOURCE_STATE_UNORDERED_ACCESS == 2, "output state changed");
static_assert(FFX_UPSCALER_VERSION == ((4u << 22) | (1u << 12) | 2u), "upscaler version changed");
static_assert(sizeof(ffxApiHeader) == 16, "FFX header ABI changed");
static_assert(sizeof(VkDeviceContext) == 24, "FFX device ABI changed");
static_assert(sizeof(ffxCreateBackendVKDesc) == 40, "FFX Vulkan backend ABI changed");
static_assert(sizeof(ffxCreateContextDescUpscale) == 48, "FFX create ABI changed");
static_assert(sizeof(ffxQueryDescGetVersions) == 56, "FFX enumeration ABI changed");
static_assert(sizeof(ffxQueryGetProviderVersion) == 32, "FFX provider ABI changed");
static_assert(sizeof(FfxApiResourceDescription) == 32, "FFX description ABI changed");
static_assert(sizeof(FfxApiResource) == 48, "FFX resource ABI changed");
static_assert(sizeof(ffxDispatchDescUpscale) == 432, "FFX dispatch ABI changed");
static_assert(offsetof(ffxDispatchDescUpscale, color) == 24, "color offset changed");
static_assert(offsetof(ffxDispatchDescUpscale, output) == 312, "output offset changed");
static_assert(offsetof(ffxDispatchDescUpscale, jitterOffset) == 360, "jitter offset changed");
static_assert(offsetof(ffxDispatchDescUpscale, reset) == 408, "reset offset changed");
static_assert(offsetof(ffxDispatchDescUpscale, cameraNear) == 412, "camera offset changed");
static_assert(offsetof(ffxDispatchDescUpscale, flags) == 428, "flags offset changed");
