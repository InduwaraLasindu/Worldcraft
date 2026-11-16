# Master specification coverage — 0.2.0 alpha

This is a truthful milestone map, not a declaration that the master brief is finished.

| Requirement group | Current state |
| --- | --- |
| Native Rust standalone application | Implemented; hosted builds/tests; Windows distribution checks in CI |
| Beginner create/explore workflow | Implemented, including offline coarse/detailed AI examples |
| Classical procedural terrain and LOD | Implemented; erosion is thermal relaxation, not hydraulic erosion |
| Terrain Diffusion | Native coarse hybrid and finite three-model detailed bake; infinite learned streaming, conditioning parity and GPU providers incomplete |
| Atmosphere and volumetrics | Implemented raster atmosphere and High mist/light scattering; clouds/weather/night sky incomplete |
| Ray-traced and non-ray-traced rendering | Raster modes implemented; experimental native ray queries audited in software; hardware tuning/denoising incomplete |
| Sculpting and durable edits | Seven brushes, history, persistence and save precision implemented; large-history optimization incomplete |
| Natural vegetation | Seeded geometry/placement implemented; wind, growth and advanced ecosystems incomplete |
| Hydrology | Analytic valley masks and sea-level ocean; drainage networks, connected rivers/lakes and hydraulic erosion incomplete |
| Caves and overhangs | Not implemented |
| Typed node/graph editor | Not implemented |
| Asset import and advanced placement | Not implemented |
| Terrain export | GLB, OBJ and calibrated 16-bit PNG implemented; terrain-only scope |
| Physics | Heightfield walking/jump/step and tree/rock exclusion; robust capsule/SDF/object physics incomplete |
| Cinematics and production tooling | Advanced camera tracks, undoable graph operations and production capture incomplete |
| Distribution | Optimized executable, portable ZIP and per-user installer workflow; signing/updater and broad hardware certification incomplete |

Major completed changes are followed by core/native/model/export audits. The CI results,
not the presence of this table, determine which distribution candidate passed.
