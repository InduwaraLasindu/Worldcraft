# Changelog

## 0.2.0-alpha.6 — procedural landscapes (2026-11-18)

- Added original generated forest litter, rock and turf as the default terrain materials, with a live switch to the scanned set.
- Seated lighter, biome-specific grass clumps on the walking surface and trimmed each grassy patch from 40 triangles to 28.
- Replaced the outer tree checkerboard with a geometric fade and extended forest cover into distant terrain shading.
- Varied cloud height and thickness across the weather field within the existing sample limits.
- Added a moving woodland camera route and a fixed-camera comparison of the two terrain material sets. [Native audit](docs/FOREST_AUDIT.md)

## 0.2.0-alpha.5 — render quality test release (2026-11-18)

- Real Bevy/NGX DLSS Super Resolution and DLAA, with initialized-device support checks.
- Official AMD FSR 4.1.2 Vulkan API integration; query and report the actual ML or analytical provider.
- Native SMAA/TAA, sharpening, output scale, shadows/AO, bloom, tone mapping/color controls, FOV/DOF, cloud/fog sampling, V-sync and restart-selectable API.
- Exclusive AA/prepass ownership, history resets on edits/rebases/cuts and previous-frame wind/crown motion.
- Reuse one persistent 80-byte frame buffer; retain shared texture/prototype and streaming budgets.
- Pinned SDK inputs, release-only runtimes, original notices, native AA/lens pixel gates and exact-source release checks.
- Software CI cannot certify vendor hardware execution or AAA quality. The 80% realism target remains unmet.

## 0.2.0-alpha.4 — nature and surface candidate

- Botanical broadleaf, conifer and palm geometry with species-specific stems and exact reduced-crown endpoints.
- Shared cutout foliage textures, coverage-preserving mips, normal-mapped bark and bounded GPU foliage breeze.
- Embedded CC0 scanned forest-floor/rock PBR arrays, triplanar cliffs and FXAA in Balanced/High.
- Reuse resident terrain meshes during foliage-only refreshes; retain epochs, revisions and bounded stale-job accounting.
- High adds an experimental bounded textured cloud volume; Low/Balanced retain their cheaper sky.
- Share one persistent 48-byte animation/observer GPU buffer across nature/surface materials; specialize shaders by immutable surface kind.
- Expand grass coverage with bounded terrain-seated clumps, cap software shadow maps at 1024 and retain full Windows memory/error logs.
- Replace the visually rejected dark cloud slab with smaller shared-texture cells and add a daytime sky image regression.
- Added native art fixtures and source/material provenance. Validation is in progress; this entry does not announce a published release.

## 0.2.0-alpha.3 — resource budgets

- Separate UI camera and reactive home/background pacing; no hidden 3D viewport behind Home.
- Persistent Automatic/Eco/Balanced/Performance budgets, FPS cap, paced terrain jobs and smaller Eco residency.
- Preserve stale synchronous jobs in the work count until completion; reuse settled streaming plans.
- Nonspinning, configurable AI threads, Eco inter-call pauses and detailed-generation memory admission.
- Saved-world unloading and resource-policy/scene/idle audits. Hardware performance remains unverified.

## 0.2.0-alpha.2 — 2026-11-16

- Added optional native detailed terrain generation using pinned coarse, base and residual-decoder models, loaded sequentially.
- Added reference conditioning groups, two flow refinement steps, globally seeded overlap noise, weighted residual tiles and Laplacian height reconstruction.
- Persisted a 320 × 320 finite detail field with smooth coarse joins and reduced classical micro-relief; default sample spacing is 30 m over 9.57 km.
- Added 2.28 GB optional streamed model management, per-stage generation progress and cancellation while preserving the fast coarse option.
- Added project format 3 with exact detail persistence and unchanged legacy generator/coarse geography.
- Expanded real repeated/different-seed model audits and native rendered saved-world checks.

Scope remains alpha: infinite learned streaming, upstream conditioning parity, GPU inference and the other unfinished master-spec features remain on the roadmap.

## 0.2.0-alpha.1 — 2026-11-16

- Migrated the renderer to pinned Bevy 0.20.1 / bevy_egui 0.43.1 and embedded WESL terrain materials.
- Added configurable Earth atmosphere, horizon ocean, High volumetric ground mist/light scattering, SSAO and filtered procedural surface detail.
- Added bounded distant terrain rings and checks for all near-boundary orientations and negative coordinates.
- Added experimental capability-gated native direct/indirect ray-query lighting, deferred terrain support, bounded proxy preparation and raster cleanup.
- Added native Rust ONNX inference using the pinned Terrain Diffusion coarse model and custom conditioning. Generated regional relief/climate is saved; this is not the full upstream cascade.
- Added optional streamed model downloads with size/hash verification and cancellation, plus an offline precomputed AI example.
- Fixed repeated native-runtime initialization and exact JSON floating-point save/reopen behavior.
- Added bounded background terrain export to GLB, OBJ and calibrated 16-bit PNG.
- Protected newly modified worlds when an asynchronous world load finishes.
- Bundled checksum-pinned Microsoft DXC and expanded native Windows image validation/evidence.
- Added optimized Windows packaging, a per-user installer, bundled native runtime/notices, and hosted portable/install/uninstall validation.
- Expanded native graphics, learned inference, export and dependency audits. The matching workflow records each candidate's actual Windows distribution result.

Scope: engineering alpha. Full diffusion cascade/GPU inference, connected hydrology,
hydraulic erosion, caves, typed generation graphs, asset importing, advanced ecosystems,
cinematics, signing/updating and broad gaming-hardware validation remain incomplete.
The feature matrix in MASTER_SPEC_COVERAGE.md is authoritative.

## Terrain/visual development pass — 2026-11-16

- Added generator 2 with seeded gradient fields, alpine ranges, mesas, archipelagos and calderas.
- Added conservative thermal transport on overlapping tiles with exact shared-edge tests.
- Retained generator 1 for existing saves, including legacy height fixtures and edit/undo coverage.
- Added bounded shared terrain caching, single-flight bakes and spatial edit indexing.
- Moved new-world preparation and spawn search off the main thread.
- Added fast regional map previews that preserve edits without churning terrain caches.
- Fixed native capture commits under concurrent CI runs and made superseded jobs cancellable.
- Added embedded periodic volume-noise terrain detail and animated water ripple normals.
- Added denser local ground cover, needle-clump conifers, layered foliage and varied rocks.
- Expanded native render checks to world switching, floating-origin stability, mountains and coasts.

## 0.1.0 development — 2026-11-16

- Established a renderer-independent Rust world core and cloud-only build/test workflow.
- Added continuous seeded landscapes, biome/climate fields, deterministic scatter,
  terrain meshes and stitched multi-resolution boundaries.
- Added validated, versioned world files with brush transactions, undo/redo and backups.
- Added the native exploration application, background streaming, procedural nature,
  walking/flying, optional editing, map previews and asynchronous persistence.
- Added Low/Balanced/High raster settings and hosted native-renderer smoke coverage.

Not a stable or production release. Hardware-specific graphics quality, Windows interaction,
performance targets and the advanced master-prompt systems are not yet verified.

### Adaptive renderer candidate (not yet released)
- Short rendered launch calibration and terrain CPU sampling, with ongoing p90-based
  automatic graphics/radius/resolution adjustment toward 30 FPS.
- Full-scene render scaling under native-resolution UI and scaled viewport brush picking.
- Continuous near-terrain position/normal/color morph targets, shared forward/shadow
  observer and a matching first horizon collar; internal topology budgets remain bounded.
- Worker-built anchored 32 m ground-cover batches and incremental replacement on budget changes.
- Radius-aware species-specific crown geomorphs, ground/outer nature fading and a shared
  world-camera shadow observer; paired cheap crowns do not duplicate ray occluders.
- Atmosphere features follow camera activation, avoiding probes without active views.
- Native audits cover crown endpoints/bounds and scaled cursor projection as well as culling.
- Staged cinematic-world program, evidence ledger and detailed future-agent handoff.
- Hosted scene/UI evidence remains separate from broad real-hardware certification.
