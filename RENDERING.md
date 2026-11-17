# Rendering

Pinned integration: Bevy 0.20.1, bevy_egui 0.43.1, native wgpu. The initialized
adapter/backend and actual ray-query features appear in Graphics settings.

| Preset | Actual behavior |
| --- | --- |
| Low | Reduced near distance, 8 km vista target, distance haze, simple background, no sun shadows |
| Balanced | 16 km vista target, atmosphere, environment-map lighting, cascaded shadows, bloom, FXAA and scanned ground/rock detail |
| High | 32 km vista target, higher shadow resolution, SSAO, optional volumetric ground mist and experimental textured clouds |

Vista targets describe clipmap coverage, not guaranteed visibility or a measured
performance budget. One background job builds power-of-two geometric rings.
The first ring matches streamed terrain and transitions to the analytic preview
over 128 m. Later rings omit fine erosion. Coarser outer boundaries share a
polyline; tests cover positive/negative bounds and near LOD transitions.
One horizon ocean replaces overlapping per-chunk water planes.

## Raster surfaces and atmosphere

Earth scattering, sun direction/illuminance, time, exposure and haze are configurable.
High adds a bounded fog volume and raymarched light scattering. The night treatment
is basic; moon/stars and a physical weather/cloud simulation are not implemented.
An optional experimental cloud layer uses nine nonoverlapping 8 km cells and one
shared 64-cubed R8 density texture (256 KiB). It uses the 24-step fog integrator,
reduced absorption and an explicit artistic scattering compensation. This avoids
the rejected black ceiling produced by the previous 32 km volume's attenuation
estimate. It is not a self-shadowed/multiple-scattering production cloud renderer.
Low/Balanced omit volumetric rendering; disabled cloud cells are hidden.

Software adapters cap shadow maps at 1024 pixels, including manual High, to bound
their shared-memory cost. Hardware adapters retain 2048/4096 for Balanced/High.
Software adapters also select CPU mesh preparation before PBR initialization.
CPU preparation alone did not pass the software DX12 memory audit. The follow-up
retires submitted software work before the next render frame, including hidden
windows, to prevent an unbounded in-flight backlog. Hardware adapters retain
their asynchronous path, and device compute/ray features remain available.
Both current Windows profiles passed the unchanged 6 GiB native memory gate;
the release-profile sampled peak was 876.17 MiB. This is hosted software
integration evidence, not gaming hardware or general process-memory certification.
See docs/NATURE_AUDIT.md for the exact source, viewports and package checks.

Eco enforces a horizontal 78.4–224 m fade to its matching background before the
nearest resident terrain edge, preserving its short view and 25-chunk budget.
Requested general haze settings apply outside Eco; underwater absorption retains
ordinary 3D distance. One reserved word in the shared frame carries the Eco flag.

The embedded WESL terrain shader blends CC0 scanned forest litter, grass and rock
with the existing biome palette. Cliffs/boulders use triplanar normal/roughness
mapping, and Low retains its cheap fallback. Mipmapped shared arrays use about
8 MiB together. Surface kind is an immutable shader key, so leaf shaders do not
contain terrain sampling. One persistent 48-byte GPU buffer carries wrapped
origin, observer/crown distance and animation/quality data for every surface and
forward/depth/shadow pass. Camera movement and wind do not modify material assets.

Ocean water uses an opaque PBR surface with animated ripple normals. Derivative
filtering and roughness adjustment reduce distant specular aliasing. There is no
wave displacement, depth refraction, planar reflection, river mesh or shoreline foam.
Trees/rocks use shared original geometry. The adaptive candidate builds ground cover
on workers in anchored 32 m batches and adds species-specific crown geomorphing,
ground/outer nature fading and radius-aware visibility. The world camera owns shadow
detail selection. Botanical species-specific stems/crowns, original masked leaf
textures with coverage-preserving mips, normal-mapped bark and a bounded GPU breeze
are implemented in the nature candidate. Advanced foliage impostors remain future work.

Near terrain positions/normals/colors morph toward parent grids before coarsening.
The first horizon collar follows the same observer function. Full 3D image resolution
can scale down beneath native-resolution UI. Atmosphere probes attach only while the
world camera is active; loading/Home/focus suspension must not leave a probe without a view.

## Experimental ray-query lighting

Solari supplies native direct/indirect ray-query lighting. The camera uses a
deferred primary pass; the procedural terrain material supports both forward and
deferred paths. Ray meshes are generated lazily, with at most two unique geometry
conversions and sixteen proxy instances added per frame. Static secondary-ray
geometry uses averaged vertex albedo; it does not reproduce every procedural
surface detail or transparent effect.

The normal UI enables this mode only on a non-software adapter with all required
features. Raster remains the default. The private integration route can exercise
software ray queries at 128 × 96. It checks a completed lighting dispatch through
render diagnostics, the actual terrain shader's deferred path, and return to
raster with proxy cleanup. This is an API integration audit, not a gaming benchmark.
There is no vendor upscaler, DLSS, production denoiser or path-tracing mode.

## Native evidence and visual targets

Hosted Linux captures the real X11 window, including egui. The route checks home
versus world images, a floating-origin shift, Low/Balanced/High, Alpine/coast/map,
ray queries and the saved learned example. The 128 × 96 Windows captures are obscured by the interface and establish API/state
checks rather than visible scene differences. A separate checksum-verified portable
package test captures the offline AI example at 640 × 360. Earlier alpha-2 terrain,
vegetation, shadows and haze were visually reviewed; the adaptive candidate has its
own check status in docs/ADAPTIVE_AUDIT.md and must not inherit that approval. Windows user interaction and
gaming-GPU performance remain unverified.

GeoGen and the supplied landscape videos are art-direction references. No
proprietary algorithms, textures, assets or frames are distributed, and matching
their quality or performance is not claimed.
