# Rendering

This milestone uses Bevy's PBR renderer and native wgpu graphics backend.
The application selects a supported adapter/backend through wgpu.

| Preset | Actual behavior |
| --- | --- |
| Low | Reduced view distance, no sun shadows, distance fog, simple background |
| Balanced | Cascaded sun shadows, HDR atmosphere, bloom and tone mapping |
| High | Higher shadow resolution, atmosphere and screen-space ambient occlusion |

Time of day changes the sun transform, illuminance and color. Haze and exposure are
independent controls. Day/night animation is optional. The current night treatment is
basic; moon lighting, stars and weather are not implemented.

Terrain has per-vertex linear colors and rough diffuse PBR response. Vegetation is original
procedural geometry, with separate bark and two-sided foliage materials.
Grass/shrubs are merged per chunk; tree meshes/materials are shared for renderer batching.
Distant vegetation uses simplified geometry. Ocean water is transparent PBR geometry:
it does not yet have real refraction, planar reflections, wave normals or shoreline foam.

The native screenshot check attempts Low, Balanced atmosphere and High SSAO on software
Vulkan; see the workflow result for actual pass/fail evidence. Representative Windows GPU
behavior and visual quality still need hardware and manual regression coverage.
Shader compilation is checked by running the native renderer, not inferred from Rust
compilation alone.

No hardware ray tracing, path tracing, denoising, virtual texturing or vendor upscaler is
implemented or advertised. These require separate capability detection and a measured design.

## Visual references

- https://www.youtube.com/watch?v=v-RZP5UXCsE
- https://www.youtube.com/watch?v=uR5EvBWxN14
- https://www.youtube.com/watch?v=mUl1QxCCAAQ

Direct video playback was unavailable in this execution environment. Accessible metadata
identified the forest showcase, but undocumented techniques and performance claims were not
assumed. These remain art-direction targets, not evidence that this build has matching quality.
No screenshot, texture or asset from the reference videos is included.

Hosted Linux QA captures the full native X11 window so the egui overlay is included.
Bevy's built-in redirected screenshot texture in this integration captures the 3D camera
without egui's direct swapchain overlay. The test also compares the home and world images
to catch an absent interface; merely writing a nonempty PNG is insufficient.
