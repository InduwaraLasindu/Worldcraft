# Changelog

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
