# Rendering

This milestone uses Bevy's PBR renderer and native wgpu graphics backend.
The application selects a supported adapter/backend through wgpu.

| Preset | Actual behavior |
| --- | --- |
| Low | Reduced view distance, no sun shadows, distance fog, vertex-color terrain, simple background |
| Balanced | Cascaded sun shadows, HDR atmosphere, bloom, tone mapping and surface detail |
| High | Higher shadow resolution, atmosphere and screen-space ambient occlusion |

Time of day changes the sun transform, illuminance and color. Haze and exposure are
independent controls. Day/night animation is optional. The current night treatment is
basic; moon lighting, stars and weather are not implemented.

Terrain uses slope/soil vertex colors plus an embedded StandardMaterial extension. Balanced
and High sample periodic 3D volume noise for albedo, roughness and analytic bump normals.
This world-space method avoids stretched cliff UVs and chunk texture seams. The f64 render
origin is uploaded modulo 4096 m; every noise period divides that domain, so rebasing does
not move the detail. The shader ships inside the executable, without runtime downloads.

Vegetation uses original procedural geometry, with separate bark and two-sided foliage.
Conifers have uneven needle-clump boughs, broadleaf crowns have layered leaf cards, and four
deformed rock meshes provide shape variation. Grass/shrubs merge per chunk; tree meshes and
materials are shared. Distant vegetation still uses simple replacement geometry.

Ocean water is transparent PBR geometry. Balanced/High add animated, world-anchored ripple
normals and a view-angle opacity response. These are shading effects; there is no displaced
wave surface, true refraction, planar reflection, river mesh or shoreline foam.

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

The native integration route also changes world epochs, captures Alpine and coastal scenes,
and compares a fixed camera before/after an origin shift. That image comparison checks
the actual shader/scene result, with tolerance for changing HUD text and render rounding.
The shader follows Bevy's [versioned material-extension pipeline](https://github.com/bevyengine/bevy/blob/v0.16.1/examples/shader/extended_material.rs).

The resident terrain currently ends at the selected view radius. At short distances, the
unrendered horizon can be visible; a far-terrain representation remains on the roadmap.
