# WorldCraft: cinematic procedural world implementation plan

Status: active engineering plan, 0.2 alpha. Updated 2026-11-16.
Owner's target: the supplied conifer valley, waterfall/river, layered mountain,
forest-floor and warm coastal reference images. These are visual direction, not
screenshots of WorldCraft and not redistribution licenses for external assets.

## 1. Non-negotiable outcomes

- Native Rust desktop application. Simple create → explore workflow; advanced editing optional.
- Beautiful coherent natural environments at walking distance and mountain-scale views.
- Continuous perceived detail: no distracting terrain changes at distance boundaries.
- Procedural variation with ecological/geological causes, not random objects everywhere.
- Automatic machine-local settings, a short launch probe, and ongoing 30 FPS targeting.
- Bounded CPU workers, terrain residency, uploads, caches, AI inference and audio work.
- Low-end machines can trade view distance, shading, vegetation and resolution for responsiveness.
- Strong machines receive higher defaults, but model/vendor names alone never certify speed.
- Stable seeds, explicit edits and old saves survive renderer changes.
- Honest evidence: no claim of photorealism, universal 30 FPS, infinite AI terrain,
  physically complete acoustics or a finished release without the corresponding proof.

30 FPS means a frame budget of 33.33 ms. Arbitrarily weak/unsupported hardware cannot
be guaranteed to meet it. Report the minimum-quality limit instead of claiming success.

## 2. Existing foundation and this iteration

| System | Existing alpha baseline | Adaptive-quality iteration |
|---|---|---|
| Terrain | Versioned deterministic heightfield, classical and finite baked AI relief | Distance-based GPU geomorph targets, including normal/color blending |
| Streaming | Worker tasks, epochs/revisions, fixed topology keys, near/far terrain | Ground-cover geometry built on workers; 32 m culling batches; replace budget changes incrementally |
| Rendering visibility | Bevy mesh AABB/frustum culling and shared nature meshes | Preserve engine culling; tighter ground-cover bounds; explicit launch-fixture visibility audit |
| Quality | Eco/Balanced/Performance work limits, hardware heuristics | Measured launch probe plus frame-time governor; automatic effective graphics and view radius |
| Resolution | Native window-sized 3D render | Full 3D render target scales down while UI stays at native resolution |
| Atmosphere | Bevy atmosphere, time of day, distance fog, High volumetric ground mist | Existing effects participate in automatic quality decisions |
| Nature | Four mesh variants per family, seeded placement, simple grass and shrubs | Distance-dithered crown pairs and ground cover; no claim of reference-quality forests or perfectly invisible temporal transitions |
| Water | Sea-level surface and animated normal detail | Connected rivers, waterfalls and lakes still planned |
| Sound | No environmental audio system | Ray-based acoustic propagation and actual audio output still planned |

Implementation presence is distinct from validation. Consult ADAPTIVE_AUDIT.md for
checks run on this exact iteration and AGENT_HANDOFF.md for remaining integration risks.

## 3. Dependency order and acceptance gates

Finish and audit each milestone before presenting it as complete. Each milestone
needs repeatable fixtures, save compatibility checks and evidence appropriate to its scope.

### M1 — Adaptive performance and continuous terrain (active)

1. Short native rendered fixture on launch; bounded terrain CPU sampling in one worker.
2. Warmup, p90 frame interval, real initialized adapter, available/total RAM; capped probe
   resolution on constrained/software adapters and conservative handling of sparse samples.
3. Conservative defaults; runtime controller lowers quality after sustained overruns and
   raises it only with sustained headroom and cooldown. Manual modes remain available.
4. Resolution scaling for the entire 3D pipeline with sharp UI and correct brush picking.
5. Internal mesh grids remain discrete to actually remove triangles, but their visible
   geometry morphs continuously to the parent grid before coarse topology is installed.
6. Use one observer for forward, depth and shadow passes; preserve floating-origin behavior.
7. First far-terrain collar uses the same morph function as the near perimeter.
8. Continue work budgets, RAM pressure protection, stale-job accounting and idle throttling.
9. Audit normal rendering, low-resolution editing, near/far joins, moving observer,
   negative coordinates, world switching, rapid flight and automatic mode changes.
10. Benchmark real Windows integrated/discrete GPUs later; software Vulkan is integration only.

Remaining M1 work must be stated explicitly: vegetation crossfades internal representations;
render-time resolution changes have not proven a hard process-RAM or VRAM budget; delayed
refinement/teleport transitions need movement-route captures and screen-error evaluation.

### M2 — Continuous vegetation, clustering and GPU visibility (partial implementation)

Implemented in the adaptive candidate: paired crowns share exact transforms and complementary
distance margins; ground patches fade at 80–128 m; all nature fades before tile eviction.
End distance follows the active view radius. Eco retains a single cheap crown. Ground
patches use 3D local anchors, and the world camera explicitly owns shadow detail selection.
Native shader/scene-structure audits are queued; temporal route captures and gaming-GPU
costs remain open. This does not complete the broader M2 work below.

- Keep deterministic placements, then separate placement identity from rendering density.
- Stable species/age/health/branch architecture variation and material palettes; avoid
  changing generation 1/2 worlds silently.
- Cluster instancing, distance/screen-coverage selection, dithered transitions with stable
  per-instance masks and shadow-compatible fading. Impostors only where silhouettes hold.
- Wind bands for trunks, branches and leaves; analytic low-cost fallback.
- GPU occlusion/Hi-Z where supported and measured; conservative frustum fallback.
- Distance-aware ground cover with density recovery instead of rebuilding whole worlds.
- Acceptance: dense forest traversal, rapid rotation and still-image tests, CPU/entity
  counts, draw counts, GPU timing, memory, identical saved positions after reload.

### M3 — Regional drainage, rivers and lakes

- Versioned regional fields with owned borders/outlets, halo exchange and deterministic
  downstream priority. Never derive connected rivers by drawing unrelated noise stripes.
- Depression handling, flow directions and accumulation with explicit ocean outlets.
- River network widths from contributing area, discharge proxy, terrain slope and geology.
- Carved channel/bank profiles; lake spill levels and connected confluences.
- Water surfaces follow hydraulic heads; no uphill segments, cracks at region borders,
  duplicated waterfalls, disconnected lake outlets or depth discontinuities at confluences.
- Persist recipe/version/region identity. Author edits stay explicit and replay after the
  terrain bake; local edits need defined drainage invalidation and recompute bounds.
- Acceptance: topology/monotonicity/conservation tests plus joined-region visual captures,
  dry/wet/alpine/ocean fixtures, travel/edit/save/reopen and bounded background work.

### M4 — Erosion, rocks and snow

- Bounded hydraulic erosion with rainfall, transport capacity, deposition, erosion limits
  and thermal relaxation. Cache immutable outputs; account for dependency halos.
- Geological strata/fractures and talus placement at appropriate scales.
- Seasonal/permanent snow influenced by altitude, temperature, slope, aspect and exposure.
- Slope-aware snow and rock layering, wetness, shoreline sediment and moss habitats.
- Version new geographic rules; legacy golden samples must keep passing.
- Acceptance: mass and numerical stability checks, tile joins, no unbounded remeshing,
  close-up material captures and repeatable snowy mountain scenes.

### M5 — Forest/ecosystem generation

- Continuous climate/soil/water/light suitability weights rather than abrupt biome labels.
- Canopy/understory competition, stand age, gaps, disturbance, fallen logs, deadwood,
  edge effects, rock exclusion, riparian vegetation and altitude tree lines.
- Species distributions appropriate to climate; related individuals with individual
  variation instead of unrelated random specimens.
- Deterministic bounded local dependency neighborhoods; no full-world growth simulation.
- Staged graph/scatter controls for advanced users; beginner presets remain coherent.
- Acceptance: reference views at ground/mid/far scales across conifer, temperate,
  rainforest, savanna, desert, volcanic, tundra, alpine and coastal worlds.

### M6 — Surface materials and near-field detail

- Procedural PBR material families: bark, needles/leaves, rock, soil, snow, sand, litter,
  moss, wet banks and water. Triplanar or appropriate local mappings without stretching.
- World-stable multiscale variation, normals, roughness and detail filtering.
- Bounded texture/material atlases; imported assets optional, provenance and licenses kept.
- Blend geological/ecological materials continuously. Contact grounding matters as much
  as polygon counts. Avoid identical tree silhouettes and uniform saturated greens.
- Acceptance: close-up comparisons at multiple sun angles, no subpixel shimmer,
  shader complexity/texture-memory measurements and low-end fallbacks.

### M7 — Sky, weather and volumetrics

- Atmosphere/sun/moon, physically sensible exposure, continuous day/night lighting.
- Layered volumetric clouds, aerial perspective, terrain/valley mist, cloud shadows,
  wind-driven weather and altitude-dependent precipitation.
- Shared weather fields drive cloud motion, fog, wetness, snow and audio coherently.
- Quarter/half-resolution and temporal options with disocclusion handling; lower modes
  retain an attractive non-volumetric fallback. No mandatory ray tracing.
- Acceptance: sunset/sunrise/night/alpine views, camera movement ghosting checks,
  bounded volumetric sample counts and graphics-adapter feature gates.

### M8 — Moving water and waterfalls

- Network-aligned velocity/flow maps, depth-aware color and absorption, shallow-water
  normals, bank foam and turbulence at obstacles/confluences.
- Waterfall geometry/spray/mist from real network drops; source audio follows those drops.
- Reflections/refraction with tiered raster and optional hardware-ray paths.
- Acceptance: steep/flat water, river-to-lake and lake-to-ocean boundaries, underwater,
  transparency ordering, no flow against downhill routing, low/high quality captures.

### M9 — Ray-based nature audio

- Actual native audio output, volume/mute controls and a cheap non-ray fallback first.
- Deterministic source emitters: rivers/waterfalls, wind, foliage, biome/daytime species.
- Spatial source selection in a bounded radius; shared coordinate/rebase policy.
- Time-sliced CPU rays against a coarse acoustic BVH or heightfield/obstacle proxies.
- Direct occlusion and material transmission first, then bounded early reflections and
  statistical late reverb from geometry/absorption. Smooth gains and filters over time.
- Never run unbounded propagation per source/per frame. Increase ray/voice/update budgets
  only when measured CPU headroom exists. Audio callbacks do not allocate or block.
- Distinguish geometric acoustic approximation from wave diffraction/physically complete
  sound. Do not label ordinary stereo ambience as ray-traced audio.
- Acceptance: occluder/canyon/forest/open-field fixtures, audible WAV captures and
  propagation tests, HRTF/device fallback, mute, no callback underruns during streaming.

### M10 — Authoring and world operations

- Graph inputs/outputs/types, validation, node caching and incremental invalidation.
- Non-destructive terrain, river, material, biome and ecosystem painting.
- Paths/structures, imported glTF/PBR assets, placement editing and asset validation.
- Sparse SDF caves/overhangs with explicit heightfield/SDF collision and material seams.
- Durable migration/checkpoints and bounded undo histories; no silent world reshaping.
- Acceptance: beginner exploration still requires no graph, save/load every operation,
  crash/recovery tests and bounded edit-update latency.

### M11 — Cinematic tools

- Camera paths, focal controls, exposure, shot presets, stable time/weather/seed settings.
- Optional high-quality still/video export with explicit memory/time estimates, offline
  accumulation and cancellation. Interactive defaults continue targeting responsiveness.
- Acceptance: repeatable short route through forest/river/alpine/coast in the reference
  composition style, actual engine output, not image-generated marketing substitutes.

### M12 — Distribution and release certification

- Optimized native Windows installer and portable build, dependency/license manifests,
  checksum provenance and clean-machine install/uninstall checks.
- Hardware matrix: older integrated, modern integrated, midrange discrete and high-end
  discrete; 8/16/32 GiB RAM; 720p/1080p/1440p; raster and supported ray modes.
- Warm/cold traversal frame-time distributions, CPU/process-RAM/VRAM and streaming stalls.
- Publish a prerelease only from the same source and verified package bytes. Keep alpha
  labeling until real stability, content, usability and performance gates are met.

## 4. Visual review checklist

| Reference cue | Required engine behavior | Evidence |
|---|---|---|
| Dense conifer valley | Coherent stands, varying age/crowns, open gaps and understory | Ground and overlook shots; traversal |
| Rock/grass forest floor | Ground contact, litter, moss, rocks with varied scale and roughness | Close shots at grazing light |
| Clear turquoise river | Depth/flow variation, rocks affecting foam, connected banks | Still and moving captures |
| Waterfall | Connected drop, spray and mist, downstream motion and local sound | Camera route and audio capture |
| Snowy mountain distance | Stable silhouettes, snowline, rock exposure, aerial perspective | Near/far moving camera |
| Warm coastal/autumn scene | Natural color variation, directional light, sky/exposure balance | Sunrise/day/sunset comparison |
| Real-life variation | Coupled geology/climate/ecology, bounded local rules | Several seeds per biome |

## 5. Engineering evidence rules

1. Record exact source/ref, command, platform/backend and result.
2. Separate source compiled, tests passed, renderer exercised, visually reviewed and
   measured real gaming hardware; none implies all the others.
3. Capture low/high views using identical seed, position and lighting.
4. Never use reference images as claims of produced output.
5. Track unfinished work with file/function entry points and reproducible commands.
6. Every major milestone includes an audit, compatibility checks and updated handoff.

## 6. Primary design references

- [Continuous distance-dependent terrain detail, original paper and implementation](https://github.com/fstrugar/CDLOD).
- [Pinned Bevy source](https://github.com/bevyengine/bevy/tree/v0.20.1): material extension,
  render-to-texture, frustum visibility, prepass/shadow and async-task APIs.
- [Terrain Diffusion](https://github.com/xandergos/terrain-diffusion): existing finite baked
  integration has explicit scope in GENERATION.md; it is not unlimited learned streaming.

These inform design. Source code, assets and model weights retain their own licenses.
