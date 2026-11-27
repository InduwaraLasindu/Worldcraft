# Worldcraft

**Create a world. Explore it. Shape it.**

Worldcraft is a native Rust application for procedural landscapes, with optional
terrain editing. The current client uses Godot Forward+ with a Rust
GDExtension. [Godot alpha.8 release and controls](docs/GODOT_RELEASE.md)

Download the Windows installer or portable ZIP from
[GitHub Releases](https://github.com/InduwaraLasindu/Worldcraft/releases).

## Build or run the Godot client

Use Godot 4.7.2 and run the project under `game/`. The Rust extension
preserves the existing seed and world-file format. Source builds and native
verification are described in [BUILDING.md](BUILDING.md).

## Previous Bevy alpha

[Download Windows alpha.6](https://github.com/InduwaraLasindu/Worldcraft/releases/tag/v0.2.0-alpha.6).
[Native screenshots and visual audit](docs/FOREST_AUDIT.md).
Use the installer or unpack the portable ZIP, then launch Worldcraft.
Choose **Create a new world**, or an offline AI example for precomputed Terrain
Diffusion terrain. The executable embeds its shaders, original botanical geometry and bounded shared textures
and example worlds. No Python, Java, paid API or model download is
needed for classical worlds or the examples.

For a new AI region, enable **AI terrain**. **Fast regional** uses an optional
**22.5 MB** model. **Detailed landscape** uses an optional **2.28 GB** set and runs
coarse, base and residual models sequentially on the CPU. Progress, checksum
verification and cancellation are built in. Generated relief is saved in the
world, so reopening and exploration do not run the models again.

## Available in this alpha

| Area | Implemented behavior |
| --- | --- |
| Worlds | Nine presets, text/numeric seeds, background creation, open/save, recent worlds and recovery snapshots |
| Terrain | Versioned classical geology, thermal relaxation, fourteen blended climate habitats, persistent sculpt/biome/density edits |
| Streaming | Continuous distance-based terrain morphing over internal mesh grids, bounded jobs/uploads, floating origin and distant terrain rings |
| Learned relief | Native coarse-only or three-model detailed bakes, custom conditioning, saved regional relief/climate, smooth joins and classical surroundings |
| Graphics | Raster PBR, atmosphere, sun/shadows, exposure/day controls, High volumetric mist and SSAO, procedural PBR surfaces with an optional scanned set, filtered water detail and FXAA |
| Ray queries | Experimental direct/indirect lighting on capable hardware; raster default; software API and material switching audit |
| Nature | Seeded oak-like broadleaf, conifers, palms, birches and acacias, plus grass, shrubs, ferns, flowers and reeds. Trees taper through distance transitions into canopy-shaded terrain |
| Exploration | Walking, jumping, sprinting, terrain collision, tree/rock exclusion and flight |
| Editing | Seven brushes, drag transactions, undo/redo and spatial edit indexing |
| Maps | Background biome/elevation/temperature/moisture previews and flight teleport |
| Export | Terrain-only GLB, OBJ and calibrated 16-bit PNG, with bounded resolution and cancellation |
| Distribution | Portable package and per-user installer workflow, bundled native inference runtime and notices |

Both learned recipes save a 48 × 48 regional relief/climate field. The detailed
recipe also saves a 320 × 320 height field: 30 m sample spacing over 9.57 km at
the default landscape scale. Two base-model steps and four overlapping decoder
tiles supply this central detail. It remains a **finite regional bake**, with
custom Worldcraft conditioning; infinite learned streaming, upstream conditioning
parity and GPU inference are unfinished. The current inference provider is CPU. Hardware ray-query support is detected from the initialized GPU;
gaming performance and vendor-specific behavior are not established by software
tests.

Caves/overhangs, procedural graph editing, connected rivers/lakes, hydraulic
erosion, ecosystems, weather, asset importing and production-grade cinematic
features remain incomplete. See [nature audit](docs/NATURE_AUDIT.md), [master coverage](MASTER_SPEC_COVERAGE.md) and
[roadmap](ROADMAP.md).

## Low-end PCs

Automatic mode runs a short native launch scene, then adjusts graphics, view distance
and 3D resolution from measured frame times toward **30 FPS**. Stronger PCs can start at
High; weaker ones use smaller budgets and lower resolution. UI stays at native resolution.
The Performance panel shows effective settings and when minimum quality cannot meet the
target. This cannot guarantee 30 FPS on arbitrary or unsupported hardware.

You can also choose **Eco** manually from Home or the toolbar. It caps exploration at
30 FPS, uses a two-chunk radius, one paced terrain job, simpler crowns and no grass or
far terrain. Home stops the world camera; unfocused windows pause 3D and new terrain work.
Preferences stay on this PC. `--eco` selects the low-cost mode without launch calibration.

Detailed AI generation requires at least 6 GiB available system RAM and is optional.
Use Fast regional or either offline example on smaller machines. Fewer AI threads
make new generation slower; reopening saved AI worlds does not run the models.
These budgets do not enforce a hard process-memory or CPU-percentage ceiling.

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
[Building and runtime files](BUILDING.md) · [Third-party notices](THIRD_PARTY_NOTICES.md) ·
[Alpha-2 audit and actual captures](docs/ALPHA2_AUDIT.md)

## Cinematic world development

The supplied visual references remain the target. Advanced ecosystems, connected
flowing rivers/waterfalls, physically richer snow/sky/weather and ray-based nature audio
are unfinished. The [engine plan](docs/ENGINE_PLAN.md) specifies their dependency order
and acceptance gates; the [agent handoff](docs/AGENT_HANDOFF.md) records implementation
entry points and what to do next. See [adaptive audit](docs/ADAPTIVE_AUDIT.md) for which
checks actually passed. Development branches remain alpha candidates until validated.
