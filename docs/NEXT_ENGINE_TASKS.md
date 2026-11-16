# Concrete continuation backlog

Updated 2026-11-16. These are implementation tickets, not delivered features or FPS claims.
Read ENGINE_PLAN.md for the complete dependency order, and ADAPTIVE_AUDIT.md for current
verification. Preserve the native Rust application and cloud-only development rules.

## A. Finish the current candidate first

| Ticket | Entry points | Completion evidence |
|---|---|---|
| A1 — final native checks | resource-limits.yml, verify.rs, raytrace::qa_check, scene::update | Latest source passes Linux scene/Auto/culling/real ray-dispatch checks and Windows release/portable/install/detailed-world checks; review actual captures |
| A2 — temporal terrain route | continuous.rs, terrain_vertex.wesl, vista.rs | Move through every parent-grid threshold and near/horizon join, including negative coordinates, slow travel, rapid flight and rebases; retain a short native capture sequence |
| A3 — nature transition review | foliage.rs, streaming.rs, assets.rs | Walk through crown and ground fades; check silhouette changes, 4x4 dither visibility, shadows and density recovery; compare stationary and moving views |
| A4 — scaled editing | control::viewport_cursor, scene::resolution, UI scene compositing | Actual mouse paint/undo at 50/67/85/100% scales and non-100% desktop DPI; save/reopen exact edits |
| A5 — hardware measurements | calibration.rs, performance.rs, core/adaptive.rs | Fixed routes on integrated and discrete Windows GPUs; p50/p95/p99 frame times, process RAM/VRAM, cold/warm startup and streaming stalls |

Do not weaken assertions to make CI green. Explicit unsupported ray-API skips are different
from observed GPU dispatches. Keep PR #4 draft while a relevant check is failing or pending.
The ray fixture has no saved world and needs its explicit active-camera path. Normal Home
and unfocused world cameras must remain inactive. Never attach an atmosphere probe without
an active 3D view on the pinned Bevy version.

Current source uses bounded terrain grids plus continuous parent morphing. Nature uses
complementary distance dithering over bounded meshes. This is a perceived-detail strategy;
it is not infinitely dense geometry or proof that every temporal transition is invisible.
The legacy cheap crown is a generic cone: family-specific silhouette matching, especially
palms and broadleaf trees, is a real remaining art/representation task.

## B. Complete continuous nature without changing geography

1. Separate terrain and vegetation upload paths. Today a vegetation band change rebuilds
   a bounded whole tile. Keep independent epoch/revision/detail identities and count every
   old synchronous task until it finishes. Do not invalidate unrelated terrain on density changes.
2. Keep existing instance identities/positions for generation 1/2. Derive render-only age,
   tint, lean, crown asymmetry and wind stiffness from stable world-cell hashes. Add golden
   placement fixtures before introducing new ecological placement rules.
3. Replace the generic cheap cone with family-specific silhouettes derived from the same
   branch/crown recipe. Near and far representations must share the exact instance transform
   and complementary distance margins. Audit palms, conifers and broadleaf separately.
4. Introduce material/geometry variation through shared prototypes and GPU instance data;
   do not create an unbounded unique material or high-detail mesh for every tree. Measure
   entity count, draw submissions and retained mesh bytes before increasing prototype counts.
5. Add wind using the same deformation in forward, depth and shadow passes. Increase mesh
   bounds by the maximum displacement, and preserve the rebased world anchor. Secondary-ray
   proxies remain approximate until deformation-aware ray geometry is implemented and tested.
6. Investigate measured GPU occlusion/cluster submission after CPU/frustum and instancing
   costs are known. Provide the existing conservative visibility path on unsupported adapters.

A first forest gate needs at least three seeds, ground/mid/far views and a fixed traversal.
Show original engine captures. Extreme random scale variation by itself is not ecology.

## C. Add versioned regional hydrology

Proposed new core modules: hydrology/region.rs, hydrology/drainage.rs and hydrology/network.rs.
These names are proposals; they do not exist yet. Keep data/generation in worldcraft-core
and water meshes/rendering in the desktop crate.

**Required region contract**

- Stable region key, generator/hydrology recipe version, sample spacing and owned interior.
- Neighbor halo plus deterministic border/outlet ownership. Define the macro drainage
  boundary rule before implementing independent region-local flood fills.
- Ground elevation, conditioned drainage elevation, downstream direction, contributing
  area, lake/spill identity and river segments with endpoint hydraulic heads.
- Explicit cancellation/progress and bounded immutable cache entries. Reject stale edit
  revisions and maintain a bounded dependency neighborhood for drainage recomputation.

**Implementation order**

1. Bake a finite validation domain with an explicit ocean boundary. Resolve depressions
   and flats deterministically, then construct an acyclic downstream graph and accumulation.
2. Validate conservation, no cycles/uphill segments, deterministic ties and lake spill levels.
3. Design connected neighboring domains with shared outlet constraints. Compare a joined
   bake with separately requested regions, in both load orders and negative coordinates.
4. Derive channel widths/depths from contributing area and slope; carve banks and confluences.
   Add a versioned recipe rather than silently reshaping existing saved worlds.
5. Build connected water geometry from the network. Derive flow direction/foam inputs from
   that network; keep velocity fields distinct from animated surface ripple noise.
6. Add waterfalls only where a real network drop exists. Their spray, fog and acoustic
   emitters must share that drop's identity and position.

Do not call existing valley masks connected rivers. Keep legacy generator fixtures and
explicit edit replay exact. A global arbitrary-world drainage solution is a separate
scaling problem from a successful finite regional bake.

## D. Improve geological materials, snow and biomes

- Add a versioned hydraulic-erosion recipe with bounded steps, positive water/sediment,
  tracked mass and numerical-stability tests. Current thermal erosion remains available.
- Separate geographic changes from render-only material changes. Snow coverage should use
  elevation/temperature, slope, aspect and exposure; avoid a horizontal white colour cutoff.
- Add world-stable rock strata, fractures, talus, soil, moss, litter and wet-bank materials.
  Filter high-frequency detail and measure shader cost at multiple distances.
- New ecological placement should depend on continuous suitability fields: climate, soil,
  drainage, slope, light, stand age and disturbance. Establish halo/dependency bounds before
  adding competitive growth or canopy/understory simulation.
- Use conifer valley, snowy alpine, temperate/autumn, rainforest, dry/savanna, desert,
  volcanic, tundra and coast fixtures with close/mid/far views. Retain shadows and ground
  contact; uniform saturation and floating objects cannot be fixed by adding polygons.

## E. Add coherent sky, weather and water rendering

- Keep the existing atmosphere/High fog as the baseline. Add moon/stars and exposure tests
  before presenting night as finished. Do not hide an inactive-camera crash with a graphics downgrade.
- Share one deterministic weather field across clouds, wind, mist, precipitation, snow,
  surface wetness and sound. Define spatial/time units and save recipe/time controls.
- Add bounded cloud/valley-volume sampling with lower-resolution fallbacks. Test sunlit
  edges, silhouettes, camera disocclusion, sunrise/sunset and low-end modes.
- Add network-aligned water flow, depth absorption, bank foam, reflections/refraction and
  underwater behavior after C. Raster must remain usable without hardware ray tracing.
- Record effects separately: atmospheric scattering, volumetric ground fog, cloud volumes,
  water reflections and acoustic rays are different systems and need separate evidence.

## F. Implement actual ray-based environmental sound

Proposed separation: core/acoustics for geometry/path queries, desktop/audio for source
selection, native device/mixing and UI. Neither module exists yet. Choose a maintained
native Rust audio backend after checking its current platform support and license.

1. First deliver actual audio output, master/ambience volume, mute and device-loss recovery.
   Provide deterministic source/voice IDs and a cheap positional fallback.
2. Build immutable acoustic snapshots from resident terrain grids and coarse obstacle
   proxies. Do not call Generator::height, allocate, take scene locks or run ray queries
   inside the real-time audio callback.
3. Use listener/source-relative local coordinates tied to a shared snapshot origin.
   Update/rebase geometry and emitters consistently; discard stale scene epochs.
4. Time-slice direct occlusion rays on one bounded worker. Add material transmission and
   smooth gain/filter changes before adding early reflections or statistical late reverb.
5. Derive river/waterfall emitters from C's real network; foliage/wind/animal emitters use
   ecology and time-of-day fields. A stereo wind loop alone is not ray-based acoustics.
6. Add bounded reflection paths and late-reverb estimates only after direct propagation
   and callback stability are measured. Treat diffraction/wave physics as separate scope.

Initial **provisional** engineering limits, to tune with measurements:

| Work | Eco fallback | Balanced acoustic path | High acoustic path |
|---|---:|---:|---:|
| Audible spatial voices | 4 | 12 | 24 |
| Geometry updates per second | 0 | 5 | 10 |
| Ray queries per update | 0 | 24 | 64 |
| Reflection bounces | 0 | 0 initially | at most 2 after validation |
| Acoustic background jobs | 0 | 1 | 1 |

These are proposed ceilings, not current settings, performance promises or a physical model.
A measured time budget may lower work below these ceilings. Keep audio queues bounded;
never block the callback waiting for a geometry job or a device/UI event.

Acceptance fixtures: open field, terrain ridge, canyon, forest proxy and a moving source
behind an obstacle. Check unobstructed distance attenuation, occlusion/transmission,
reflection delay, finite output, smoothed transitions, mute and device fallback. Save WAV
captures for DSP/listening review and separately verify device playback. CI offline mixing
alone does not prove audible native output or callback behavior on real hardware.

## G. Later authoring, cinematics and release work

Follow ENGINE_PLAN.md M10–M12 after the above foundations. Graphs need typed immutable
inputs, cache identities, cycle/error handling and incremental invalidation. Cave/overhang
geometry needs an explicit heightfield/SDF boundary and compatible collision/brush saves.
Imported assets need bounded loading and license/provenance records. Beginner exploration
must still require no graph knowledge.

Cinematic paths and offline exports should use reproducible seed/camera/time/weather settings,
explicit memory/time estimates and cancellation. Publish only the exact verified package bytes.
Keep alpha labeling until the visual, stability, interaction and hardware gates actually pass.

## H. Stop-point checklist for the next agent

- Identify the exact source SHA, active PR/base and final workflow run; inspect failures first.
- Preserve saved-world formats/versions and pending-job accounting.
- Pick one bounded ticket with observable native behavior; implement, audit and update docs.
- Write the current state in AGENT_HANDOFF.md and results/provenance in ADAPTIVE_AUDIT.md.
- Do not mark future modules above implemented merely because their design is documented.
