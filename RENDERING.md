# Rendering

Pinned integration: Bevy 0.20.1, bevy_egui 0.43.1, native wgpu. The initialized
adapter/backend and actual ray-query features appear in Graphics settings.

| Preset | Actual behavior |
| --- | --- |
| Low | Reduced near distance, 8 km vista target, distance haze, simple background, no sun shadows |
| Balanced | 16 km vista target, atmosphere, environment-map lighting, cascaded shadows, bloom and procedural detail |
| High | 32 km vista target, higher shadow resolution, SSAO and optional volumetric ground mist |

Vista targets describe clipmap coverage, not guaranteed visibility or a measured
performance budget. One background job builds power-of-two geometric rings.
The first ring matches streamed terrain and transitions to the analytic preview
over 128 m. Later rings omit fine erosion. Coarser outer boundaries share a
polyline; tests cover positive/negative bounds and near LOD transitions.
One horizon ocean replaces overlapping per-chunk water planes.

## Raster surfaces and atmosphere

Earth scattering, sun direction/illuminance, time, exposure and haze are configurable.
High adds a bounded fog volume and raymarched light scattering. The night treatment
is basic; moon/stars, weather and cloud simulation are not implemented.

The embedded WESL terrain shader combines slope/soil colours with periodic 3D
albedo, roughness and bump detail. Derivative-based filtering fades detail smaller
than a pixel, and avoids sampling invisible noise octaves. The render origin is
uploaded modulo 4096 m to keep shading anchored through origin rebases.

Ocean water uses an opaque PBR surface with animated ripple normals. Derivative
filtering and roughness adjustment reduce distant specular aliasing. There is no
wave displacement, depth refraction, planar reflection, river mesh or shoreline foam.
Trees/rocks use shared original geometry; ground vegetation merges per chunk.
Wind and advanced foliage impostors remain future work.

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
ray queries and the saved learned example. Windows redirected screenshots may
omit the egui overlay, so they do not by themselves prove Windows UI interaction.

GeoGen and the supplied landscape videos are art-direction references. No
proprietary algorithms, textures, assets or frames are distributed, and matching
their quality or performance is not claimed.
