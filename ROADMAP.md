# Roadmap

## Current development milestone

Native window and UI, versioned classical generation, streamed heightfields, stitched LOD,
procedural nature, PBR atmosphere, walking/flying, optional brush tools and persistence.

The second terrain pass adds halo-safe thermal erosion, richer preset landforms, spatial edit
indexing, bounded shared terrain caches, background creation and world-anchored ground/water
detail. Legacy worlds retain generator 1.

Hosted tests establish the behaviors listed in the workflow results. This is not the completed
master specification or a production-ready early release.

## Next: stronger natural worlds and visuals

- Regional drainage graphs, depression handling, connected downhill rivers and lakes.
- Connected watershed ownership and boundary-safe hydraulic erosion.
- Better ecological blending, plant diversity, natural ground detail and imported glTF assets.
- Imported terrain PBR texture sets, wind shaders, refraction/shorelines, sky/night and vegetation LOD.
- Incremental edit indexing, compact edit checkpoints and measured cache tuning.
- Better player slope/capsule collision, configurable bindings, pause/quit recovery and accessibility.
- Measured Windows GPU runs and visual regression scenes.

## Advanced work

- Native learned-inference proof of concept and license-audited model management.
- Hybrid learned/classical generation with versioned conditioning.
- Sparse volumetric caves and surface transitions.
- Typed procedural graph editor with validation and caching.
- Procedural paths, structures, ecosystem growth and weather.
- Exports, cinematic cameras and asset editing.
- GPU-driven vegetation, terrain geomorphing and capability-tested optional ray/path tracing.

## Distribution gate

Before calling a release usable: exercise the complete create → explore → edit → save → reopen
flow on Windows, preserve edits during subsequent streaming, validate graphics switching,
stress repeated worlds/travel, and resolve resource-lifetime failures.
Then package a portable executable and installer with appropriate notices, clean-machine
testing and a clearly documented support matrix.
