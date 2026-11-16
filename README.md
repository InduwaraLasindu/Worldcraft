# Worldcraft

**Create a world. Explore it. Shape it.**

Worldcraft is a native Rust application for procedural landscapes, with optional
terrain editing. Version **0.2.0-alpha.1** is an engineering alpha, not the completed
master specification or a photorealistic production engine.

## Start exploring

[Download the Windows alpha](https://github.com/InduwaraLasindu/Worldcraft/releases).
Use the installer or unpack the portable ZIP, then launch Worldcraft.
Choose **Create a new world**, or **Explore the AI example** for an offline,
precomputed Terrain Diffusion region. The executable embeds its shaders, original
nature geometry and example world. No Python, Java, paid API or model download is
needed for classical worlds or the example.

For a new AI region, enable **Terrain Diffusion regional hybrid** and download the
optional **22.5 MB** pinned coarse model through the app. Progress, checksum
verification and cancellation are built in. Generated relief is saved in the
world, so reopening and exploration do not run the model again.

## Available in this alpha

| Area | Implemented behavior |
| --- | --- |
| Worlds | Nine presets, text/numeric seeds, background creation, open/save, recent worlds and recovery snapshots |
| Terrain | Versioned classical geology, thermal relaxation, ten biomes, persistent sculpt/biome/density edits |
| Streaming | Four near LODs, stitched edges, bounded jobs/uploads, floating origin and distant terrain rings |
| Learned relief | Actual native ONNX coarse-model inference, custom conditioning, persisted regional height/climate, classical fine detail and surroundings |
| Graphics | Raster PBR, atmosphere, sun/shadows, exposure/day controls, High volumetric mist and SSAO, filtered ground/water detail |
| Ray queries | Experimental direct/indirect lighting on capable hardware; raster default; software API and material switching audit |
| Nature | Original broadleaf/conifer/palm meshes, grass, shrubs and rocks with seeded placement |
| Exploration | Walking, jumping, sprinting, terrain collision, tree/rock exclusion and flight |
| Editing | Seven brushes, drag transactions, undo/redo and spatial edit indexing |
| Maps | Background biome/elevation/temperature/moisture previews and flight teleport |
| Export | Terrain-only GLB, OBJ and calibrated 16-bit PNG, with bounded resolution and cancellation |
| Distribution | Portable package and per-user installer workflow, bundled native inference runtime and notices |

The learned hybrid uses a 48 × 48 regional relief field. It does **not** run the
large base model/residual decoder, reproduce the full upstream cascade, or replace
the classical world outside the saved transition band. The current inference
provider is CPU. Hardware ray-query support is detected from the initialized GPU;
gaming performance and vendor-specific behavior are not established by software
tests.

Caves/overhangs, procedural graph editing, connected rivers/lakes, hydraulic
erosion, ecosystems, weather, asset importing and production-grade cinematic
features remain incomplete. See [master coverage](MASTER_SPEC_COVERAGE.md) and
[roadmap](ROADMAP.md).

## Controls

| Input | Action |
| --- | --- |
| Click landscape | Capture mouse |
| WASD / mouse | Move / look |
| Space / Shift | Jump or ascend / sprint |
| Ctrl in flight | Descend |
| Tab / E | Walk-flight / edit |
| Left drag in Edit | Paint one undoable stroke |
| Right drag + WASD in Edit | Move editor camera |
| Esc | Release mouse |
| Ctrl+S / Ctrl+Z / Ctrl+Y | Save / undo / redo |
| F1 | Help |

Save current edits before switching worlds. If an asynchronous load finishes
after new edits were made, it waits for those edits to be saved.

## Verification

All development and execution use hosted public-repository GitHub runners. Nothing
is compiled or installed on the owner's computer. [Workflows](https://github.com/InduwaraLasindu/Worldcraft/actions)
record the actual pass/fail results. Linux software Vulkan exercises native UI,
terrain, atmosphere, volumetrics, ray queries and the learned example. Native
model tests run on Linux and Windows. Export validation uses the official Khronos
validator and checks PNG elevation reconstruction.

These tests establish their stated integration behavior, not broad gaming-GPU
performance or completion of the master brief. Windows rendering and installer
results must be read from the corresponding release workflow.

[Architecture](ARCHITECTURE.md) · [Generation](GENERATION.md) ·
[Rendering](RENDERING.md) · [Performance](PERFORMANCE.md) ·
[Building and runtime files](BUILDING.md) · [Third-party notices](THIRD_PARTY_NOTICES.md)
