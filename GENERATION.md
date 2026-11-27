# Generation

Current generator: **version 4**. Version-1 through version-3 worlds keep their
original height, climate and scatter rules. Opening or editing an old world
retains its saved generator version.

Units are metres. Seeds are u64 values or stable FNV-1a conversions of text.

## Version 4 river valley

The River Valley preset adds a finite seeded trunk and tributary near spawn.
Both flow toward sea level and share the same water height where they merge.
The river bed uses the same height query as rendering, collision, edits, and
exports. The waterfall is an 18 m drop at the default vertical scale. The
water uses one world-anchored mesh and does not need a streaming job. This
preset uses a fixed drainage layout with seeded meanders. Vegetation admission
fades around the waterline on both branches. The independent
finite drainage solver is available for future terrain-dependent routing.

## Version 3 climate and ecology

Version 3 preserves version 2's terrain heights and thermal erosion. It adds domain-warped
temperature and humidity regions, altitude cooling, stand variation and fourteen normalized
biome weights. The four new biomes are birch woodland, savanna, wet woodland and mountain
meadow. The visible label is the dominant weight; terrain pigment and habitat probabilities
blend continuously through the region boundaries. Explicit biome paint blends those weights
in saved stroke order. These are original multi-noise-inspired rules, without Minecraft code
or assets. They are artistic habitats rather than a calibrated climate or ecosystem model.

Canopy/rocks use an 8 m world-cell lattice and nearby ground cover uses a 2 m lattice.
Hashed admission, stand age/gaps and local patches produce reproducible populations. Forest,
pine, birch, acacia and palm probabilities follow habitat weights; ferns, flowers and reeds
join grass/shrubs. The structural limit is 256 canopy/rock plus 4096 ground instances per
128 m chunk. Detail bands omit ground cover beyond the near ring. Eco omits that ground
ring entirely. Density-rejected plants do not request additional slope samples.

The renderer shares four procedural prototypes per tree family, shared materials and
small texture sets. Understory is appended to 32 m meshes on the existing bounded workers;
there are no per-plant entities or materials for grass, ferns, flowers or reeds. Root
transforms stay unchanged between origin rebases. This bounds work structurally; actual
device cost still requires measurement. Legacy v1/2 placement digest tests guard seed,
position, scale, yaw, species and variant compatibility.

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

The finite River Valley layout supplies the inland water network. Other presets use
noise-derived valleys with water at sea level. General watershed routing, regional
lakes, hydraulic erosion, rain shadows, and ecosystem growth remain future work.
Climate controls are artistic normalized values.

Biome labels are discrete; version 3 materials and habitat probabilities have continuous
transition bands. Materials combine slope/soil/biome pigment with world-space procedural
detail and the pinned shared CC0 terrain PBR arrays. Terrain is a heightfield without cave
openings or overhangs. The placement rules do not yet simulate drainage, soil moisture
transport, root competition or ecosystem succession.

Smooth brushes blend toward a stored local mean rather than running diffusion. Terrace
brushes quantize height with radial falloff. Vegetation placement near the River Valley
waterline uses the edited height.

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
The fast recipe does not use the larger base model and decoder.

Format 2 persists these samples. Formats 1, 2 and 3 remain readable; format-1 worlds
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

## Finite native detailed recipe

The optional `terrain-diffusion-detailed-region-v1` recipe uses all three pinned
models from the same revision. Its download totals 2,276,345,629 bytes. A 20-step
coarse sample uses 7,680 m spacing at default scale. Its central 4 × 4 coarse
values provide the reference 58-component base conditioning vector, including
p5 conversion, climate means, mask and magnitude-preserving group scales.
Worldcraft still supplies its own preset/climate conditioning upstream of that
coarse result; WorldClim parity is not claimed.

The base model performs two normalized flow steps, at atan(160) and atan(0.7).
Four 256 × 256 residual tiles use overlapping 32 × 32 latent crops, nearest
latent upsampling, the reference flow output sign and residual standard deviation
0.7. Shared global PCG noise supports negative coordinates and identical overlap
pixels. Weighted decoder overlap covers a 448 × 448 native field.

The fifth latent channel supplies the low-frequency square-root elevation
(mean -31.4, standard deviation 38.6). Reconstruction uses extrapolated bilinear
upsampling, antialiased downsampling and reflected Gaussian filtering before
signed-square metre conversion. A 64-cell halo is discarded; the saved central
field is 320 × 320 at 30 m spacing, spanning 9,570 m. Landscape scale multiplies
all spacings. The outer 32 cells blend smoothly into the coarse parent. Classical
micro-relief fades from 12 m to 2 m inside the detailed region. Edits apply last.

Format 3 stores this detail and the base/decoder hashes. No model is needed to
sample, edit, export or reopen it. Legacy coarse worlds omit the new field and
retain the original coarse recipe. The models load sequentially to bound peak
memory; downloads stream to disk and inference runs on a background worker.
Cancellation is checked during file reads and between native model calls.

This is not the full infinite reference pipeline: neighboring base-window
ensembles, arbitrary learned-region streaming/caches, upstream conditioning
parity and GPU providers remain future work. Hosted CPU timing and memory are
fixture measurements, not universal device guarantees.
