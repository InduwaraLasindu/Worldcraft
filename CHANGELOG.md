# Changelog

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
