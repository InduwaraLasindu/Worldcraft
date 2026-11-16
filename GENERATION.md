# Generation

Current generator: **version 2**. Version-1 worlds remain readable and keep their original
height, climate and scatter rules. Creating a new world selects version 2; opening, editing,
undoing or saving an old world never upgrades its generator silently.

Units are metres. Seeds are u64 values or stable FNV-1a conversions of text.

## Version 2 classical backend

1. Seeded gradient noise and noninteger domain offsets avoid an origin-centered flat spot.
2. Warped continental fields, ridge multifractals, foothills and detail form regional relief.
3. Presets add alpine ranges, terraced desert mesas, rolling savanna, archipelagos and
   volcanic cones with caldera depressions.
4. Analytic valley masks carve narrow depressions. They are **not** drainage networks.
5. Thermal transport relaxes slopes on an 8 m global lattice, for eight iterations.
6. Temperature/moisture fields classify ten biomes; slope and soil patches color the surface.
7. Explicit sculpting, biome and density edits replay in saved order.
8. Seeded, world-aligned candidates populate trees, rocks, shrubs and local grass.
9. Resident 128 m chunks sample the same height field at four mesh resolutions.

There is no mutable global random stream. Request order, negative coordinates, worker count,
eviction and regeneration do not intentionally change the terrain or placements.

## Thermal erosion and tile continuity

A 512 m region contains 64 cells plus its shared edge. Baking expands the region by 18
cells on every side before simulation. Each iteration computes four downhill fluxes from
the previous height state, using a 0.60 rise/run talus threshold and a 0.045 transport
coefficient. Incoming and outgoing material are then applied in a fixed order.

The halo exceeds the simulation's dependency radius. Adjacent tiles therefore compute
identical shared-edge heights independently, including at negative coordinates.
The simulation conserves height-volume on its closed finite grid. Tests check conservation,
peak relaxation, an unchanged stable plane and matching tile edges.

This is a small talus-relaxation simulation, not a hydraulic solver, geologic time model or
physically calibrated sediment system. Erosion is baked **before** explicit edits; a brush
does not trigger a fresh erosion simulation.

## Bounded caching and edits

Each immutable settings/version instance owns a shared LRU cache of at most 32 terrain
tiles, about 1.03 MiB of retained f64 height values. A single-flight slot prevents concurrent
requests for a resident tile from baking it repeatedly. Worker-held evicted tiles and
temporary simulation buffers add memory beyond that retained-grid figure.

Brush snapshots share that cache. Validated strokes are indexed into 256 m spatial cells,
preserving saved order within each cell. Sampling visits nearby candidates rather than
the entire history. History snapshots still clone edit metadata; compact checkpoints are
future work.

Regional map previews sample analytic geology, climate and edits without baking erosion
or touching the exploration cache. The map labels that approximation. Traveling to a map
location samples the full eroded terrain for the actual camera placement.

## Scope and limitations

Valleys are noise-derived masks, not downhill-flowing rivers. Water currently occupies
sea level. Lakes, connected tributaries, waterfalls, hydraulic erosion, rain shadows and
ecosystem growth are not implemented. Climate controls are artistic normalized values.

Biome labels are discrete. Materials combine slope/soil vertex colors with world-space
procedural surface detail; ecological transition bands and imported PBR texture sets are
future work. Terrain is a heightfield without cave openings or overhangs.

Smooth brushes blend toward a stored local mean rather than running diffusion. Terrace
brushes quantize height with radial falloff. Biome and vegetation painting are independent
of elevation.

## Version 1 compatibility

The first generator's value-noise/fBm/domain-warp/ridge math is retained, along with its
climate and scatter behavior. Golden height samples and legacy save/edit/undo tests guard
against unintended reshaping. New preset defaults do not replace settings stored in an
existing world. The generator version and project format are separate fields; legacy formats remain readable.

## Native learned regional hybrid

The optional coarse model from xandergos/terrain-diffusion-30m-onnx is pinned to
revision ad2df557eca5645f588766101cf3bc3682455c3e, with a fixed SHA-256 and byte size.
A native Rust/ONNX Runtime path performs twenty EDM DPM-Solver++ midpoint steps.
Portable PCG/Marsaglia noise is seeded locally. Worldcraft supplies explicit
preset/climate conditioning; it does not reproduce the upstream WorldClim
quantile factory.

A 64 × 64 model result is cropped to a 48 × 48 region. Height, temperature and
precipitation are stored with their origin, spacing, transition band, recipe and
model identity. Learned relief blends into classical surroundings; classical
small-scale detail supplements the interpolated regional elevation. The region
is finite; it is not an infinite diffusion cache. New AI regions use CPU inference.
The full 2 GB base model and 224 MB decoder are not used.

Format 2 persists these samples. Formats 1 and 2 remain readable; format-1 worlds
keep their original generator. Explicit edits apply after the stored relief,
survive undo/save/reopen, and never trigger inference. JSON parsing enables exact
f64 round trips, including camera and grid spacing.

Native Linux/Windows audits perform three inferences: repeat the same seed, then
change the seed. In the audited fixture, repeats matched exactly on each CPU;
cross-platform heights differed by at most about 0.00115 m. This is one test case,
not a universal cross-device bound. Saved samples remove inference variation
from reopening that particular world.

The native runtime environment is shared for the process lifetime. Downloads
stream to an isolated temporary file, check size/hash, then replace the model.
Cancellation is checked between reads and inference steps. No Python or Java
runtime is shipped. Source/model/data licensing are separate; see the notices.
