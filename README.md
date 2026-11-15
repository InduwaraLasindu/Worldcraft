# Worldcraft

**Create a world. Explore it. Edit it when you want to.**

Worldcraft is a native Rust desktop application for continuous procedural landscapes.
It opens its own Windows/Linux window; no browser, Java, Python runtime, paid API,
commercial engine, or downloaded AI model is needed.

> Development milestone, not a finished release. The visual target is a richly detailed,
> realistic world engine. The current procedural artwork and terrain are an initial
> foundation, not photorealistic or equivalent to the reference showcases.

## What is implemented

- Home screen, new worlds, nine presets, numeric/text seeds, open/save and recent worlds.
- Continuous coordinate-based terrain with domain warping, coherent ridge masks,
  analytic river valleys, climate fields and ten biome classifications.
- Background chunk generation, four terrain LOD levels, stitched boundaries,
  temporary skirts, bounded jobs/uploads and a floating render origin.
- Procedural broadleaf trees, conifers, palms, shrubs, grass and rocks with
  biome, moisture, slope, elevation and density conditions.
- PBR sunlight, cascaded shadows, atmosphere, haze, day/night controls, exposure,
  ocean surfaces and three raster graphics presets.
- First-person walking, ground collision, jumping, sprinting, tree/rock exclusion
  cylinders and free flight. Exploration is the default.
- Optional raise/lower, flatten, local smoothing, terrace, biome and vegetation
  brushes; drag transactions with undo/redo.
- Versioned JSON worlds, validation, settings fingerprints, previous-save backups,
  minute and close-time recovery snapshots and persistence of terrain/vegetation edits.
- Background map previews for biome/elevation/temperature/moisture; double-click
  a map location to travel there in flight mode.

Only implemented controls appear in the application. Ray tracing, learned generation,
physical hydrology, hydraulic erosion, caves, graph editing, weather, imported assets
and polished installers remain on the [roadmap](ROADMAP.md).

## Controls

| Input | Action |
| --- | --- |
| Click the landscape | Capture mouse for exploration |
| WASD / mouse | Move / look |
| Space / Shift | Jump or ascend / sprint or fast flight |
| Ctrl in flight | Descend |
| Tab | Switch walking and flight |
| E | Switch exploration and editing |
| Left drag in Edit | Paint one undoable stroke |
| Right drag + WASD in Edit | Move the editor camera |
| Esc | Release mouse |
| Ctrl+S / Ctrl+Z / Ctrl+Y | Save / undo / redo |
| F1 | Help |

Save current changes before replacing a world. Recovery snapshots can be opened through
**Open**; their location is described in [BUILDING.md](BUILDING.md).

## Development and verification

Work happens through GitHub branches and hosted Linux/Windows runners. No local build
or dependency installation is required on the project owner's computer.
[Cloud verification](../../actions) checks the core on both operating systems, compiles
the desktop application, and runs a native Linux window with software Vulkan.

Software rendering checks integration and shader execution. It does not establish Windows
interactive behavior, RTX support, photorealism, or performance on a gaming GPU.

See [ARCHITECTURE.md](ARCHITECTURE.md), [GENERATION.md](GENERATION.md),
[RENDERING.md](RENDERING.md), [BUILDING.md](BUILDING.md) and [PERFORMANCE.md](PERFORMANCE.md).
