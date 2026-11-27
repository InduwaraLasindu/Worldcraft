# Agent handoff: adaptive renderer and cinematic world program

## Procedural tree anatomy · alpha.12

`worldcraft-core::trees` generates swept indexed trunks and branches, frond
leaflets, and a seeded 32-cell foliage mask. Tree placements remain in the
versioned scatter system. A world-position hash selects one of 2,048 structural
recipes per family, mapping four mesh profiles and eight levels each of height,
spread and branch clearance into the existing MultiMesh transforms and custom
data. The near and coarse foliage meshes share locations and seeds. Godot
reuses up to 40 mesh pairs per world and shares one mipmapped tree atlas. Bark
and foliage use the same shader deformation for steady wind and gusts.

The release checks the family view from the exported game with software Vulkan.
Native tests cover deterministic variation, mesh indices, atlas reproducibility,
and the triangle reduction across the two detail levels.

## Procedural scenery · alpha.10

`worldcraft-core::ground_cover` adds deterministic scenery to the Godot client
without changing world serialization or the original terrain generator. It
samples habitat by 8 m patch, admits jittered 2 m cells, and seats props on the
rendered triangles. On nine River Valley tiles, the total includes both the new
layer and the original canopy and understory. The HUD reports resident objects.
Windows install and portable smoke tests require at least 10,000 objects.

The existing Rust worker builds packed MultiMesh data for each species and
prototype. The main thread uploads one completed terrain packet at a time.
Distant and Eco tiles use subsets of the same grid. Ground batches skip shadow
and GI passes. Logs and major rocks retain shadows. The release workflow renders
the scenery gallery in Forward+ on software Vulkan.

## Godot landscape rendering update · alpha.9

The bridge supplies near-terrain sand, ash and snow weights and distant canopy
cover from the existing versioned generator. The Godot surfaces use these for
rock bedding, soil, snow and canopy shading. The river packet identifies the
plunge pool and encodes aeration, so the water, local fog and GPU spray align
with the seed and survive world rebasing. Ferns, shrubs and reeds have separate
procedural meshes. Godot evaluates the low cloud banks in a half-resolution
sky pass. The continuous terrain algorithm and saved-world format are unchanged.

Major releases use the matching `v0.2.0-alpha.N` tag. The workflow packages
and launches the installed and portable Windows builds, renders the River
Valley and woodland scenes with software Vulkan on the Linux runner, and
publishes both captures and the Windows installer. See [Godot release notes](GODOT_RELEASE.md).

## Godot Forward+ migration · 2026-11-27

The desktop entry point is `game/project.godot`. The Rust GDExtension keeps
versioned terrain, biome classification, scatter positions and all world
transactions in the existing core. The worker accepts at most four in-flight
meshes, drops obsolete generations, and joins before unloading the extension.
Godot uploads one terrain packet per frame. MultiMesh receives one packed
buffer per prototype and tile. Vegetation prototypes and the water, ground,
wood, foliage and sky shader recipes generate from the seed. One live noise
texture job supplies a mipmapped field up to 4096 pixels while the previous
field remains available.

Forward+ uses directional shadows, thin-leaf transmission, procedural fog
volumes and SDFGI. Atlas size, lighting, view radius and FSR 2 scaling are
independently adjustable. Static colliders are currently limited to terrain
height. Hardware ray-traced lighting in Godot and real AMD/NVIDIA profiling
remain development work. The previous Bevy client is retained for historical
fixtures and tools. See [Godot release notes](GODOT_RELEASE.md) for controls,
build scope and renderer references.

## Photographic-style environments · 2026-11-27

All quality tiers default to the bundled field-art materials. Eight original
source surfaces cover litter, granite, turf, sand, snow, gravel, basalt and
exposed soil. The bake produces seamless albedo, estimated height and normals,
and material roughness. Terrain uses local climate, canopy cover, and slope
to blend the surfaces. Weather still changes roughness, snow coverage and
surface pigment. Procedural relief remains selectable under Terrain materials.

The pack replaces all five tree-family cutouts, the grass atlas, and oak and
birch bark. Pine whorls use offset heights and variable reach. Nearby boulders
gain small chips seated on the terrain triangles within existing ground
batches. These are renderer changes and do not change saved placements.

Low uses a single near-field color lookup. The embedded atlas uses 512-pixel
BC3 mips or a 256-pixel decoded fallback. The terrain map arrays take about
5.3 MiB in either format. The photographic branch uses compact legacy inputs.
See the field-art source sheets and bake recipe under `crates/worldcraft/art`.

All 141 workspace tests and the native render-log gates passed. The desert,
woodland, and alpine comparisons used matching seeds, cameras, lighting,
and 960 × 640 software Vulkan settings. The left panel is the previous
build. The right panel is this pass. Wide terrain shapes, distant vegetation
and lighting remain visible gaps against the photographic targets.

[Desert](media/field-art/desert.webp) · [Woodland](media/field-art/woodland.webp) · [Alpine](media/field-art/alpine.webp)

## Day cycle, local storms, and weathered surfaces · 2026-11-27

Play starts with a 16-minute day. Light and atmosphere contains an hour
scrubber, a cycle toggle, the day length, and manual storm previews. One
solar path drives the sun, terrain illumination, fog, and exposure.
Balanced and High render procedural stars and a moving moon through the sky
pass. Low uses the lightweight clear sky and the same time-of-day lighting.

Automatic weather combines the local biome, altitude, temperature, and
moisture with seeded moving fronts. Warm wet regions get rain and thunder,
cold regions get snow, and dry regions get blowing sand. The strongest fronts
can bring hurricane conditions to warm, humid coasts. Fast particles and
clouds share wind and gusts. Thunder follows visible lightning after a short
delay and plays a generated audio clip. The storm state is session-local.

Rain darkens and smooths terrain, stone, and foliage. Snow builds in sheltered
patches, sand leaves deposits, and surfaces recover gradually. Strong wind
bends foliage and strengthens water ripples. Boot-scale pigment and relief
variation use the existing terrain maps in all biomes. The effects add one
adaptive mesh for nearby precipitation and waterfall spray. Snow, wetness,
and dust are visual surface state.

The native render-log gate passed on software Vulkan for night grassland,
snowy tundra, desert sandstorm, rainy meadow, estuary hurricane, and
River Valley thunderstorm. The hurricane and thunderstorm recorded two
lightning strikes each. All 141 workspace tests passed, including synthetic
thunder decoding and post-storm surface recovery. The hosted audio checks
used an ALSA null device.

Set `WORLDCRAFT_AUDIT_HOUR` or `WORLDCRAFT_AUDIT_WEATHER` with the native
`--biome-smoke-test` fixtures to repeat these views.

## River valley, close surface detail, and regional weather · 2026-11-27

New World → River Valley opens on a dry bank facing a waterfall. The seeded
finite river and tributary share a water level at their clipped confluence.
The tributary follows a gentler grade and the trunk reaches sea level at its
mouth. One static mesh carries water normals, shallow bank shading,
and animated whitewater. Vegetation fades out below the waterline. Generator
version 4 gates the new preset. Legacy terrain and edits still use their
stored recipe. Terrain-routed catchments,
regional lakes, and hydraulic simulation remain separate generation work.

The shared terrain shader bends rock, grass, and litter sampling using
world-space variation. Filtered close-range grain affects pigment, normals,
and roughness across biomes. Dry exposed sediment adds broken wind ripples.
Integer noise hashing replaces per-corner sine calculations. The cloud field
uses cross-axis phase offsets and a longer period with the existing ray-march
limit. Seeded humidity links cloud density,
mist, and the wet appearance of the ground. These changes use the existing
material and cloud assets.

Capture the waterfall with `WORLDCRAFT_BIOME_CASE=river-valley` and
`--biome-smoke-test` through the native render-log audit. The render-log
gate passed on software Vulkan for River Valley and the close desert and
snow fixtures. The river mesh contained 1,432 triangles. All 137 workspace
tests passed with the `ci` profile, including exact-site coverage for the
14 biome fixtures.

In a matched 960 × 640 aerial estuary comparison at the 4K material tier,
the median of 24 settled frames fell from 1,302 to 1,001 ms and p95 fell from
2,078 to 1,164 ms on Mesa llvmpipe. Each build ran once on software Vulkan.
The Windows gaming-GPU frame-time gate still requires physical hardware.

## Photorealism release fixtures and finite drainage · 2026-11-26

The versioned fixture manifest covers all 14 production biomes. Each case
pins a seed, preset, hour, and horizontal site. The audit checks the exact
generated biome at that site. Close, mid, and wide stills and the 60-second
camera route use fixed clouds and mist with wind and time progression off.
The native runner isolates captures,
checks their render logs and metadata, and records the executable digest.

```sh
python3 tools/ci/run_photoreal_audit.py --binary target/ci/worldcraft \
  --output captures/photoreal-ocean --xvfb --case ocean --view close
```

The Windows release checks use `--reference-stills` for native 1440p and
`--gpu-gate` for three 1440p Vulkan/DLSS Quality routes per biome. Use a
reference Windows PC with a 1440p or larger desktop and the release build
with `nvidia-dlss` enabled and its pinned runtime installed. The frame-time check
requires 16.7 ms p95 and 25 ms p99 in every route. Physical hardware validation,
CPU and GPU profiling, memory budgets, photographic reference assignment, and
visual sign-off remain separate release gates.

Software Vulkan validation passed for a native 2560 × 1440 ocean still at the
4K texture tier, the wetland mid view, and the 60-second ocean route. The
workspace tests cover all 14 fixed fixture sites.

The renderer-independent drainage solver fills depressions in a bounded
finite grid with explicit boundary outlets. It emits cardinal downstream
links, contributing cell counts, connected lake IDs, spill cells, and
channel edges. Neighboring-region outlet constraints, channel carving, and
rendered river and waterfall geometry remain to be integrated.

## Regional cloud banks and horizon contrast · 2026-11-26

The cloud shader uses both channels of the existing weather sample to bend
the smaller billow field and separate banks with regional gaps. The cloud
field and wind retain their periodic coordinates across origin wraps.
The view march keeps its 16–48 sample limit. Long rays place more samples
near the camera. The density mip and optical depth follow each segment's
length. On rays that see the sky, distant clouds approach the rendered
atmosphere color. Rays with an opaque surface behind the cloud keep the
ordinary depth-aware composition.

Matched seed-42 ground and aerial estuary captures passed the native render-log
gate at 960 × 640 with 4K procedural materials. In a sky-only crop of the
ground view, the mean absolute brightness change between adjacent rows fell
from 3.82 to 2.15 on an 8-bit scale. This measures local band contrast.
In the aerial view, the median of 24 settled frames went from 1,769 to
1,716 ms and p95 went from 1,921 to 1,791 ms on Mesa llvmpipe. Each timing is
from one run. Physical GPU performance and cloud stability during movement
remain unmeasured.

## Mist depth ordering and cloud sampling · 2026-11-26

The atmosphere pass splits the valley-mist integral at the near cloud boundary.
Clouds can now obscure mist behind them in an aerial view. A ray still has at
most six mist probes. Two visible intervals share those probes, with more weight
near the camera. Cloud samples outside the possible density envelope skip the
weather lookup. Saturated cloud interiors skip the edge-erosion lookup because
its result cannot change their density.

The fixed-camera regression view is part of the native biome audit. It looks
down through the cloud deck over the estuary with a tall mist layer. Its JSON
includes the camera, active material resolution, and median and p95 frame time
over 24 settled frames.

The matched 960 × 640 estuary view used 4K procedural materials and Mesa
llvmpipe. With the exact shader from `629bfcb`, median frame time was 1,777 ms
and p95 was 2,025 ms. Depth ordering alone measured 1,863 ms median and 3,289 ms
p95. With the texture-read optimizations it measured 1,769 ms median and
1,921 ms p95. These are single-run software-renderer measurements. The mean
RGB difference introduced by the texture-read optimization was 0.000024 on an
8-bit scale. The native capture and render-log validation passed. The visual
comparison used the procedural material path because the cloud workspace
denied access to the pinned scanned-texture host.

```sh
WORLDCRAFT_BIOME_CASE=estuary-cloud-descent WORLDCRAFT_AUDIT_4K=1 \
  WGPU_BACKEND=vulkan LIBGL_ALWAYS_SOFTWARE=1 \
  python3 tools/ci/run_native_audit.py render.log xvfb-run -a \
  ./target/ci/worldcraft --biome-smoke-test
```

## 4K world materials and visible volumetrics · 2026-11-26

Renderer revision `9d03a3b` applies **Render settings → Terrain materials → World texture size**
to ground pigment, roughness, normals, shore sediment, five leaf families,
grass, two bark sets and the distant tree atlas. The procedural ground also has
4K height for parallax. The optional scan set adds fine synthetic surface detail
to its pinned 1K photographs. Water waves and clouds have their own procedural
inputs. Low and Eco load 512 px, and adapters without BC compression cap
world textures at 1K. A single worker reads selected mip levels and discards an
obsolete result if the requested tier changes. The Windows packages include all
21 external material files. The primary 4K procedural world textures occupy
about 416 MiB of GPU texture payload with mipmaps. Driver allocations, shadow
maps and scene geometry are additional.

Balanced and High fog request their depth prepass even with anti-aliasing,
ambient occlusion and clouds off. Six depth-bounded intervals resolve moist air
and cloud-shadowed sun across valleys. A 32³ feathered fog volume handles
nearby tree shadows within the directional-light cascade range. High with
volumetrics enabled shows both. Zero fog or zero mist density extinguishes the
volume. Native estuary and woodland pairs keep the sun, world and camera fixed.

All 131 Rust tests and three material-baker tests passed. Nine material and landscape
captures passed the native parallax checks. The estuary's average upper-half
pixel difference rose from 5.68 to 55.83 on an 8-bit RGB scale. A walking-height
forest comparison measured 19.43. Both used 4K world maps with 960 × 640 output.
Four static estuary runs on llvmpipe gave 582.1 ms median with volumetrics off and
750.4 ms with them on. Gaming-GPU frame times are still unmeasured.

[Estuary toggle](media/ultra/estuary-toggle.webp) · [Woodland pair](media/ultra/woodland.jpg) · [Native rock comparison](media/ultra/stone.jpg) · [Texture detail crops](media/ultra/texture-detail.jpg) · [Evidence](evidence/ultra-world-materials.json)

## Atmospheric surfaces · 2026-11-26

High quality samples 1K ground normal and roughness maps alongside the selected
procedural relief maps (up to 4K). Low, balanced and Eco upload only the 512
base mip tails. The scanned stone ray proxy reads the matching 256 mip.
Grass and near foliage have 512-square cutouts. Birch and acacia now have
distinct leaf shapes and materials, while tussocks pick up local colour variation.
Beach sediment carries tileable sand, ellipsoidal pebbles and shell fragments in one extra normal-array layer. Pine crown heights and widths follow four growth forms, with the same profiles used by distant cards.
Rough bark and birch bark use 512- and 256-square colour and normal maps.

High quality with clouds, volumetrics and ground mist enabled integrates four
intervals of cloud-shadowed valley haze along each depth-clipped sightline.
This supplements the existing shadowed near fog. Cloud-bank erosion is baked
into the existing volume. The estuary native smoke route enables volumetrics;
set WORLDCRAFT_AUDIT_CLEAR_AIR=1 for a matched capture with volumetrics off.

All 130 Rust tests and three offline relief-material tests passed. Seven native biome captures and the ordinary estuary spawn were rendered at 960 × 640. Nine additional fixed views passed the 4K parallax and landscape checks. The repeated flat exposure was pixel-identical. The light comparison uses the same estuary, camera and 08:00 sun position. Haze softens the far ridge. Repeating distant crowns and the broad sandy foreground are still visible.

On software Vulkan the matched 78-frame moving forest route measured 963.0 → 944.0 ms median, 1,100.9 → 1,066.5 ms p95, 1,134.6 → 1,110.7 ms p99. Both runs stayed within 25 resident tiles and four queued jobs. This route holds volumetrics off. The cost of godrays on a gaming GPU is still unmeasured.

[Four native scenes](media/atmosphere/gallery.jpg) · [Matched terrain and foliage views](media/atmosphere/comparison.jpg) · [Estuary with and without volumetrics](media/atmosphere/estuary-light.jpg) · [Capture and timing data](evidence/atmospheric-surfaces.json)

## Tidal estuary · 2026-11-26

New world → Tidal estuary opens on a walkable bank. A seeded tidal inlet splits
around an island and rejoins before reaching open sea. The normal thermal terrain,
brushes, map, exports and save paths consume the same heightfield. Regional model
output supplies the uplands when that generation mode is selected. Tides are
fixed at the selected sea level. The channels are procedural landforms; flow,
sediment transport and moving tides are future work.

The estuary shares one water surface with the open sea. One existing vista
worker samples up to 2,401 signed terrain depths on an 8 m lattice. On a shore
tile the water mesh has 2,601 vertices and 5,000 triangles, including the outer
horizon. Other presets, Eco, deep water and dry tiles use the ordinary plane.
Shallow-water tint and intermittent foam use bed depth and the ordinary waves.
Rebases, regenerated edits and observer movement retain world-aligned samples.
Within 148 m, wet sand, broken wrack deposits, gravel lenses and filtered bed
ripples share the terrain draw. Distant pine cards now retain four branch
architectures in the existing 512-square atlas, including a high-crowned open
pine. The representation still uses two triangles per tree. To capture the new
world's unmodified player camera, set WORLDCRAFT_BIOME_CASE=estuary-spawn
on the existing --biome-smoke-test route.

Seed 42's exact 8 m height samples cover 2.56 × 5.12 km. The water occupies
one four-neighbour component and surrounds one 11.84 ha dry island. All seven
native biome cases and the unmodified estuary spawn were captured at 960 × 640
on software Vulkan. Each settled at 25 resident tiles with an empty queue. The
resident water mesh spanned both sides of sea level at both estuary viewpoints.
The final checks include 130 Rust tests. Distinct tree profiles survive at a
distance, but repeated cones, smooth cloud edges and broad beach bands are
still visible. The current single-review land realism estimate remains about 50/100.

Repeating the moving forest route from the earlier forest-light baseline to the
complete estuary candidate gave 1,019.08 → 961.22 ms median,
1,151.00 → 1,123.42 ms p95 and 1,457.21 → 1,150.30 ms p99. Each run pairs
78 moving frames at the same positions, with at most four queued jobs.
These are software Vulkan timings. A gaming-GPU frame rate is still unmeasured.

[Estuary views and terrain survey](media/estuary/views.jpg) · [Measurements and capture hashes](evidence/tidal-estuary.json)

## Crown lighting, regional grasses and cloud banks · 2026-11-26

Continue from the forest-light branch at `10f6b550`. The woodland-canopy branch
adds an offline cellular cloud volume and a five-direction leaf-area bake on the
twenty shared tree models. The voxel field exists only during prototype creation.
The existing vertex colours carry the result into raster shading. This approximates
local sky exposure and does not implement scene-wide voxel GI.

Full crown sprays reach the existing parent geometry by 128–160 m, depending on
the view radius. Band-three tiles skip the fine crown entity. The distant forest
still reaches 1,536 m. Grass spends the same 28 triangles per patch on four
habitat silhouettes and irregular footprints. The shared grass atlas remains
256 square. The cloud pipeline uses a 64³ RG8 volume (599,186 bytes including
mips), one regional weather lookup per view step and a two-channel density sample.
Local sunlight probes reuse the regional weather. Sunrise strengthens ground
mist and warms the haze. Zero density still disables the corresponding effect.

A walking-height review prompted a second pass. Broadleaf cards now depict
eight lateral shoots with six smaller leaves each. Birch bark uses irregular
horizontal pores and peeling bands. Trunks intersect sloping ground at the
root flare. Eligible generator-3 woodland cells can include one fallen limb per
32 m batch, seated on the walking terrain and limited to 60 extra triangles.
It shares the herb material and adds no entity or wind motion.

The final forest source is `abb72f14dfc553a7603eb2ffd83ae8fe65781f5c`.
The hosted tests passed 127 Rust cases. All six untouched biomes passed the
native Linux checks. The same 78-frame woodland flight on software Vulkan at
960 × 640 went from 1,003.1 to 949.5 ms median and 1,119.9 to 1,134.8 ms at
p95. At the standard six-tile start eye, visible-range tree geometry fell
from 8,321,384 to 5,942,100 triangles for the same 4,663 trees. The frame-time
test used the two-tile route. Gaming-GPU timing is still open.

[Six original views](media/woodland/biomes.jpg) · [Matched flight view](media/woodland/forest-comparison.jpg) · [Both frame traces and image hashes](evidence/woodland-canopy.json)

The six land views score about 50/100 in one art-direction review on the
existing five-category rubric. Each category is 2/4. Distinct crowns and
wooded distant slopes read well, but repeated clumps, smooth bare banks,
soft cloud contours and simple rock faces remain visible.

Generate embedded inputs with `python tools/prepare_materials.py` before a
desktop build. The existing six-biome and moving woodland commands exercise the
ordinary renderer. No project-format, seed, model, or brush migration is required.

## Distant forest and frame pacing

New generator-3 worlds and existing generator-3 saves use two triangles per distant tree.
The 512-pixel species atlas includes three profiles for each of the five tree families.
The detailed tree range ends by 480 m. The sparse canopy tapers into the terrain cover
between 1,280 and 1,536 m. Eco skips the distant tree batches. Beyond streamed tiles,
heights follow the drawn clipmap triangles and species weights use 128 m climate samples.
The far silhouettes receive raster lighting and use the existing terrain shade for ground
contact. Secondary rays and shadow casting use the existing scene geometry.

The moving woodland fixture records 78 two-metre camera steps with actual adapter, render
dimensions and p50/p95/p99 frame intervals. Capture I/O and settling frames are excluded.
Automatic quality has a selectable target and uses measured limiter sleep to identify
headroom. The [moving-camera measurements](ADAPTIVE_AUDIT.md) cover 25 and 169 resident
terrain tiles. The [matched ridge captures](media/skyline/WorldCraft-woodland-comparison.jpg)
show the visual change. Physical Windows hardware remains the performance acceptance gate.

## Current continuation — alpha.7 material relief (2026-11-18)

Based on the alpha.6 forest branch. The material data, forward/deferred shading,
secondary stone hits, terrain settings and native comparison route are implemented.
[Release notes](ALPHA7_RELEASE.md) give the control paths and physical limits.
The verification route records fixed-camera repeated and POM exposures, a lateral
camera move, 4K/1K, the coast and the alpine snow belt. The release job requires
Linux and Windows pixel differences plus a completed native ray dispatch.

## Previous continuation — alpha.6 procedural landscapes (2026-11-18)

Product source `bd0288b09a74feaebe48dc17d7549875e4d76d1c`, product tree
`09e6b1c88f27f490b0fff59d9238d030ddb34756`. [PR #9](https://github.com/InduwaraLasindu/Worldcraft/pull/9)
is based on the renderer branch. The alpha.5 release remains available at its
original immutable tag. Use [the current audit](FOREST_AUDIT.md) and [the matched
images](media/forest/WorldCraft-alpha5-alpha6-comparison.jpg) for this iteration.

New work includes a default fully procedural ground material set, an optional live
switch to the scans, 14-card terrain-seated grass, geometric outer tree withdrawal,
climate/brush-aware distant canopy pigmentation, and weather-shaped volumetric
clouds. The old seeds and edits still use their recorded generator versions.
Native DLSS SR/DLAA and AMD FSR 4.1 support from alpha.5 carries forward.

Cloud verification includes 115 Rust tests, nine Python regressions, a 162-tile
three-version ground sweep, and 32 material switches. The alpha.6 image run uses
an actual Linux window at 960 × 640. All six untouched worlds and eight camera-flight
checkpoints passed. All matched camera transforms, flora counts and biome weights
agree with the released alpha.5 baseline. The Windows candidate, installer, portable
checksums, exact source, SDK runtime hashes and original license notices were checked
independently.

[Windows alpha.6 is published](https://github.com/InduwaraLasindu/Worldcraft/releases/tag/v0.2.0-alpha.6).
Its tag is the product commit above. Release 449837628 has ten public assets.
The [exact-source ledger](evidence/alpha6-release.json) identifies the tested
packages and screenshot hashes. Run 44280142267 completed successfully on Linux
and Windows and published the prerelease. Windows exercised all nine AA/lens
cases, the installed and portable runtimes, safe uninstall, and the extracted
portable's 640 × 360 learned-world edit/save/reopen sequence. Linux exercised
software ray queries and collected 49 PNGs. Vendor upscaler execution on
compatible physical GPUs remains unverified. Keep this tag immutable.

Art-direction estimate for the six land views is approximately 45/100 under the
rubric in FOREST_AUDIT.md. One review cannot certify the 80% or 100% targets. Three
high-impact gaps visible in the supplied references are irregular overlapping
ground cover, actual distant trees above ridgelines and connected waterways with
convincing banks. The far vegetation currently lives in surface shading beyond the
tree cutoff. For the next iteration, start with a bounded clump or impostor skyline
over the existing camera route, then tackle versioned drainage. Preserve the
worker, upload, entity and image budgets, saved seeds and edit IDs. Review on
physical gaming hardware before giving FPS, VRAM or vendor temporal quality claims.

## Previous continuation — alpha 5 published (2026-11-18)

[Windows test prerelease](https://github.com/InduwaraLasindu/Worldcraft/releases/tag/v0.2.0-alpha.5) is public.
Product source: `a58a3d45988fcb9bc1dea22a4a7cdbd3035aeebe`.
Product tree: `30df4273e41bcdb307995725abf49b0831ae08c6`.
The v0.2.0-alpha.5 tag points exactly to that source; never move/reuse it.
Subsequent documentation/CI-only commits do not change the released executable.

Branch `codex/worldcraft-render-quality`, PR #6, based on ecology PR #5.
No main merge. The owner authorized the test release and continued graphics work.
Read [RENDER_AUDIT.md](RENDER_AUDIT.md),
[the release ledger](evidence/alpha5-release.json) and [recent original images](https://github.com/InduwaraLasindu/Worldcraft/blob/f0f860b380eb26d988623bc9f928b39f1b90c575/docs/RENDER_GALLERY.md).

Implemented actual Bevy/NGX DLSS SR/DLAA; official AMD SDK 2.4.0 / upscaler API 4.1.2
Vulkan integration with active-provider reporting; native SMAA/TAA, sharpening,
output scaling, shadows/AO, bloom, color, lens/camera, sampling and display controls.
Previous observer/wind and the current shore parameters use one fixed 96-byte shared frame. Prepass ownership/history
cuts, resource caps, old seeds/saves and authored edits remain intact. The retained
upstream bevy_render copy has one explicit non-Vulkan API guard; Windows settings
restore required indirect validation. Original SDK/renderer notices remain bundled.

Final evidence: 113 Rust and nine Python tests; actual SDK-enabled builds;
independent upstream C++ ABI/enum checks. Linux job 134618604124 / run 44250142452
passed every native route, nine renderer cases and 27 original captures.
The overall matrix failed because its first Windows route timed out; do not call
that whole workflow green. Final Windows job 134652064975 / run 44260405593 passed
all gates, including installer/portable/runtime/safe uninstall, nine AA/lens cases,
128 × 96 software scenes and the extracted portable's 640 × 360 learned-world
sculpt/undo/exact save/reopen. AA scope: 400 × 300 WARP, sampled peak 960 MiB,
unchanged 6 GiB ceiling. These are software checks, not gaming measurements.

The first Windows harness reached 12 minutes after six clean captures; its 20-minute
retry reached eight. Measured WARP settling cost was roughly 2–2.5 minutes per case.
The successful bounded harness allows 30 minutes for all nine cases and preserves
candidate packages before long graphics checks. No shader/pixel/ownership/memory
checks were removed. The tested harness is copied into the primary CI after release.

Publisher run 44261967041 verified exact-source reports, inner hashes, SDK runtime
hashes and original notices, then published. The public installer SHA-256 is
`1305045280666a1db88cda773c995fd2b5254e6146fb30faf6935b12d73cde57`;
portable `8718b14c718ab582df2545bd4731a7365f116819c8107853909956594659e66f`.
An independent cloud download reproduced both hashes and all four runtime hashes.

All 27 final-source PNGs are retained on the evidence branch at f0f860b380eb26d988623bc9f928b39f1b90c575.
TAA woodland and savanna were inspected; earlier c717 woodland/alpine images are
separately retained. Do not relabel those earlier images as a58 output.

The 80% realism target remains unmet. Next useful art iteration: ground coverage/
botanical scale and bounded distant-forest representation, then review a moving
route across crown/outer transitions and improve cloud sampling/shape. Preserve
worker/chunk/cache/upload caps, shared prototypes, software fencing and save digests.
Connected hydrology, broad hardware tests and physical editing/DPI behavior remain open.
DLSS/DLAA and FSR ML execution were unsupported-device skips on these runners.

The cloud workspace detached/reset during the task; code and original evidence were
already durable on GitHub. Do not use the owner's computer or depend on old scratch
SDK paths. Future releases must bump the version/tag and re-run exact-source gates.

## Historical ecology baseline — source below predates renderer settings

## Previous continuation — generator 3 ecology (2026-11-18)

Remote branch: `codex/worldcraft-nature-realism`.
Draft [PR #5](https://github.com/InduwaraLasindu/Worldcraft/pull/5), based on
`codex/worldcraft-adaptive-quality` (PR #4). No main merge or public release.

Executable source: `e21d5bcf11b0ec8d3621c8bd97ea5aafd934fe60`.
Executable tree: `bf69d8a58d13dd684ac9dd12eafaba5a83d42794`.
Documentation/evidence commits after this source do not change executable code.
Fetch the branch into a fresh cloud checkout before continuing. The previous
hosted workspace became unavailable after code and original captures were pushed;
do not depend on its scratch files or use the owner's computer.

Read [BIOME_AUDIT.md](BIOME_AUDIT.md),
[the original native gallery](BIOME_GALLERY.md) and
[the exact-source ledger](evidence/biome-e21d5bcf11b0ec8d3621c8bd97ea5aafd934fe60.json)
before the historical notes below.

Implemented: generator 3's fourteen normalized climate habitat weights; birch,
acacia, wetland, moist meadow and habitat-aware mixed plant populations; shared
original plant prototypes/atlases; bounded 32 m herb batches; 40-triangle masked
grass; retained static change ticks; continuous High cloud pass; F2 photo mode.
Legacy v1/2 math, save rules, biome enums/placements and regression digests remain
unchanged. Saved learned regions reopen without new inference.

Final checks: 105 Rust tests and seven Python tests pass on Linux and Windows.
Full run 44198247964 and preview run 44198247963 both passed both jobs. Linux
completed an actual software ray dispatch (499.196309 ms, not gaming speed) and
raster cleanup. Windows explicitly retained raster on its unsupported software
adapter. Portable/installed runtime, safe uninstall and the extracted portable's
640 × 360 learned-world sculpt/undo/save/reopen gates passed. New inner package
bytes and packaged PNG were not independently downloaded/viewed after workspace
loss; use the precise verification scope in BIOME_AUDIT.md. Windows artifact:
14381876383, 92,005,849 bytes, outer GitHub SHA-256
`21953f12fb4791cd53e1015f25c90de1fb93b489310c6618a0b16109d9272480`.

All eighteen exact-source Linux PNGs were visually inspected. Six sites across
five seeds contain zero authored placements. Every site settled at 25 chunks /
zero queued jobs. The 80% realism target is **unmet**. Ground coverage/plant scale,
far forests and outer checkerboard fades, cloud sampling/shape, connected water,
temporal travel, physical mouse/DPI editing and gaming-GPU costs still need work.

Next concrete task: improve ground coverage/botanical scale and the distant-forest
representation with bounded shared prototypes, then inspect a moving route across
crown/outer transitions before adjusting cloud shape/sampling. Use the original
gallery as the baseline. Keep per-tile/job/upload/cache budgets, shadow/morph
endpoints, software submission fence and Windows 6 GiB gate. No blanket density
increase or unique material/mesh per plant.

The Solari material backend lacks alpha cutoff for secondary rays. Ground batches
must retain `NoRayProxy`; the UI states grass self-shadowing is unavailable in
experimental ray mode. Primary raster/depth/shadow alpha remains active. Accurate
tree secondary surfaces and deformation/morph parity are still incomplete.
High clouds use the continuous HDR pass, not the historical nine fog boxes.

The official runtime archive returned HTTP 404. The final source uses a
same-version official Microsoft NuGet fallback with TLS, pinned length/SHA-256,
native-only extraction and licenses. Both platforms performed three actual coarse
inferences with repeat error 0. Preserve its integrity/path-traversal regressions.
Earlier failed/superseded sources and the rejected local executable-replacement
race are recorded in BIOME_AUDIT.md.

## Historical notes — sources below are superseded

## Historical continuation — nature alpha 4 (2026-11-17 UTC)

Active remote branch: `codex/worldcraft-nature-realism`, draft PR #5, based on PR #4.
Current executable source: `57b68b97330e57905362bcd29e231bcbb102085d`.
Read docs/NATURE_AUDIT.md first. Earlier adaptive evidence is historical, not
certification of this executable. API pushes can create equivalent source trees with different commit IDs. Compare
trees when synchronizing the local continuation checkout with the remote branch.

Implemented botanical prototypes, shared masked/mipmapped foliage and bark,
embedded CC0 scanned forest/grass/rock arrays, terrain-seated grass and FXAA.
Foliage-only refreshes retain terrain/root handles and avoid terrain sampling/uploads.
Surface kind is an immutable shader key. One persistent 48-byte ShaderBuffer
(binding 103) contains wrapped origin, observer/crown end, and time/breeze/detail/
Eco-horizontal-fog flag. Animation, movement and mode changes must not modify
material assets or create pipeline keys. This is a single active world-camera
design; revisit per-view data before adding independent observers.

High clouds use nine nonoverlapping 8 km cells sharing one 64-cubed R8 texture.
Smaller bounds and explicitly artistic scattering correct the rejected dark
ceiling. This is not physical weather, cloud self-shadowing or reference-level
volumetrics. Low/Balanced omit cloud-volume rendering. Software shadow maps cap
at 1024; hardware retains 2048/4096. Windows QA keeps full logs/memory history
and rejects above 6 GiB; this is a QA ceiling, not a process allocator guarantee.

Source e48b88f passed Linux but exceeded Windows memory at 6452.07 MiB. CPU mesh
preparation source 34bb65c also passed Linux but failed Windows at 6504.82 MiB;
35 samples show initial growth/recovery followed by another increase after the
High-to-Alpine transition. Do not certify either package. Retained failed CPU
source: docs/evidence/software-preparation-34bb.json and its native log.
Current source waits for submitted software GPU work before next-frame allocation,
testing hidden-window submission backlog. Hardware adapters keep asynchronous
preparation/rendering; required ray/compute features remain available. CPU fallback
also removes the RenderStartup initializer that would undo its extraction mode;
the startup/recovery regression exercises this twice.

Eco now fades its finite horizontal boundary before the nearest resident edge,
with matching background color, 25 chunks, one paced job and no distant triangles.
Nearby/underwater behavior and manual-quality restoration are tested. The current
native aerial/ground route passed the image gate; radial fog was rejected because
it erased the elevated view. Moving-camera transitions still need review.

Current local: 64 core/ML + 30 desktop tests, four log-gate tests, formatting and
optimized CI-profile build passed. The combined software-wait full native route
and Eco two-camera route passed. The actual ray dispatch completed in 996.01 ms
and returning to raster released ray work. The image harness rejects both
historical bad Eco captures. Preview 44152973239 passed both Linux art and
Windows CI-profile native checks.
The sampled Windows peak was 1645.54 MiB (11 samples), then stable. Its 128x96
window images mostly show UI, so this is state/shader/memory integration. Full
release/package run 44152973365 passed both jobs too. Release Windows peaked at
876.17 MiB (16 samples); its 640x360 packaged world rendered and passed exact
sculpt/undo/reopen checks. Installer/portable bytes and source ID were verified,
along with bundled runtime/compilers/notices. The installer runtime, portable
runtime and safe uninstall tests passed. Full artifact/source hashes are in
docs/evidence/software-frame-57b68b9.json. All eleven current art captures and
the Windows packaged-world image were reviewed and retained in docs/media/nature.
This is a verified engineering alpha; no main merge or release was made.
Prior source 34bb65c had all eleven native art captures, a completed software ray
dispatch/raster cleanup and calibration/culling checks; none is a gaming benchmark.
Current byte-identical source tree: 16f110907b69d43cd843e4cf873ca47e9cde4f21.

Next: preserve the source/package evidence and unchanged memory gate. Improve
sparse ground/ecosystem coverage, sky/cloud cell artifacts and moving-camera terrain/crown routes. Actual gaming-GPU FPS/VRAM,
physical mouse/DPI editing and >=50 reference-realism acceptance remain unmeasured.
Supplied libraries are assessed in ENGINE_PLAN.md: Serde already runs,
bevy_generative targets Bevy 0.16.1 and Terra moved to Codeberg. Do not swap legacy
noise or introduce unbounded nested Rayon pools.

## Previous adaptive baseline

Updated: 2026-11-16. PR #4 (draft, based on PR #3). Active branch: `codex/worldcraft-adaptive-quality`.
Base: `codex/worldcraft-resource-limits`, commit ca2837b (PR #3), on alpha-2 main.
Executable source: `094ecd1b3c14ceee2110f1269c359c8700cf13b2`.
Final strict run: 43923268904. Both jobs passed; native logs/captures were reviewed
and Windows ZIP/installer bytes were checksum-verified. Documentation-only commits do
not restart native CI. This is a verified engineering alpha, not the full cinematic engine.
This is a follow-on feature branch; do not overwrite or silently discard the previous
resource-budget work. Do not claim the reference-quality natural-world engine is finished.

## Read first

1. Root AGENTS.md: cloud development, native Rust, free public CI, bounded streaming,
   compatibility and honest feature claims.
2. ARCHITECTURE.md and GENERATION.md: core/render separation, generator 1/2, formats 1/2/3,
   explicit edits, baked learned terrain and physics limits.
3. docs/ENGINE_PLAN.md: complete requested scope, dependencies and acceptance gates.
4. docs/ADAPTIVE_AUDIT.md: actual evidence for the current code and remaining checks.
5. docs/NEXT_ENGINE_TASKS.md: concrete follow-on tickets, hydrology contracts and acoustic work ceilings.
6. PERFORMANCE.md, MASTER_SPEC_COVERAGE.md and ROADMAP.md: earlier baseline and limits.

## What was already present

- Released alpha 2: classical terrain and finite detailed Terrain Diffusion bake;
  explore/edit/save/recover/export; atmosphere/mist and optional experimental ray queries.
- PR #3: idle home camera fix; Eco/Balanced/Performance budgets; persistent preferences;
  bounded worker pools and nonspinning native AI; available-RAM checks before detailed AI.
- Existing Bevy frustum culling and multi-threaded workers. Do not re-advertise them as
  entirely new or add duplicate culling that breaks shadow/collision residency.

## What this branch adds

| Entry point | Responsibility |
|---|---|
| core/adaptive.rs | Pure launch classification, p90 sampling and hysteretic runtime controller |
| app/calibration.rs | Small isolated rendered launch fixture + one background terrain CPU probe |
| app/performance.rs | Effective automatic mode/graphics/radius/resolution; manual modes and pressure protection |
| app/scene.rs::SceneTarget/resolution | Entire 3D pipeline renders into scaled texture, native-resolution UI |
| app/ui.rs | Scene compositing, measured status/effective settings and minimum-quality warning |
| app/control.rs | Cursor coordinates transformed into the scaled camera viewport for editing |
| core/continuous.rs | Observer-distance topology selection, parent targets and CPU reference morph |
| app/terrain_vertex.wesl | Same position/normal/color morph in forward and depth/shadow passes |
| app/surface.rs | Custom vertex layout/material integration and shared rebased observer |
| core/vista.rs::build_continuous_vista | First horizon collar matches the near morph and fades to analytic far terrain |
| app/streaming.rs | Retains tiles during budget replacements; worker-built anchored 32 m ground batches |
| app/foliage.rs | Radius-aware geometric crown boundary, gradual ground/outer fading; explicit shadow observer |
| resource-limits.yml | Hosted Linux/Windows builds, tests, launch/Eco/idle/general graphics and packages |

`app/` above means `crates/worldcraft/src/`; `core/` means `crates/worldcraft-core/src/`.

## Invariants and gotchas

- Generator/save formats are unchanged. Render detail must not reshape durable geography.
- Actual collisions still use finest terrain near the player; morph begins hundreds of
  metres away. Tests must continue checking collision samples against that original grid.
- GPU morph is continuous; internal topology remains finite grids to reduce real triangle
  work. Do not remove topology reduction and call a constantly dense mesh an optimization.
- Morph targets reuse sampled grid vertices, avoiding repeated generator work. Negative
  world positions must keep floor-based globally aligned parent grids.
- Parent normal/color must meet the same endpoint as position, not only geometry.
- The same observer uniform drives shadows/depth and forward color. Do not use a light's
  camera position as the shadow-pass detail observer.
- AABB bounds include the morph targets because those targets are existing grid samples
  inside the original tile. Revisit bounds if future displacement goes beyond that domain.
- Attach atmosphere environment probes only to active world views. Bevy 0.20.1 can
  panic preparing their bind groups when no active atmosphere view supplies transforms.
  Camera activation must be part of the graphics reconciliation state.
- UI camera reserves render layer 30, and the launch fixture uses 31. World meshes stay
  on layer 0 so the UI camera does not perform their visibility work while Home is open.
- Resolution uses a render target plus egui compositing, not window resizing. UI stays
  sharp, and picking must convert window logical coordinates to camera logical size.
- Calibration is a small real scene, not a universal GPU benchmark. It has shader warmup
  uncertainty and a short interval; below twelve samples use conservative selection.
  Width caps are 256/640/1280 for software/constrained/other adapters. Software is always
  tier 0, including when the user opens a world before the launch probe completes.
  Warmup needs at least one second/eight frames, followed by two seconds of measurement;
  the total cutoff is eight seconds. Sparse samples are shown as limited in the UI.
  Live adaptation is required after launch. Probe dimensions/timing/sample counts are in the report.
- Low RAM triggers emergency Eco. No claim of a hard allocator/VRAM or CPU-% cap.
- Manual settings skip automatic tuning; pressure protection is retained.
- Pending synchronous jobs from old epochs still count until completion. Never drop
  their accounting merely because their output will be discarded.
- Vegetation refresh keys include their own detail independently of terrain topology;
  never rely only on the terrain key or approaching a tile can retain its distant crown.
  Eco clamps equivalent requests to avoid rebuilding identical vegetation.
- Crowns geometrically morph to species-specific leaf/bough proxies over 64 m.
  parent.w == 1 is the crown domain; observer.w is its radius-aware end distance.
  Terrain uses parent.w as its distance in metres. Do not confuse these domains.
- Two unindexed bounding vertices make base-position AABBs contain all morph targets;
  they add no indexed triangles. Keep this or provide equally conservative explicit bounds.
- Near/coarse crowns share the exact endpoint, with an abrupt visibility switch there;
  they do not use checkerboard crown crossfades. 3D ground anchors must match
  CPU/shader range origins. Paired crowns share transforms/margins and exclude the cheap
  copy from ray proxies. The image-rendering world camera must own ShadowLodOrigin, or
  the UI camera can incorrectly control shadow detail. Radius changes refresh ranges.
- Teleports/late refinement, budget changes and horizon rebakes can still produce
  visible changes. Dithering is quantized; the entire world is not proven pop-free.
- Experimental ray proxies are static mesh copies; evaluate their morph mismatch before
  claiming ray lighting is compatible with every continuous-terrain transition.
- Do not add controls for planned rivers/audio/weather until those features actually run.

## Verification commands

```sh
cargo fmt --all -- --check
cargo test -p worldcraft-core -p worldcraft-ml --locked
cargo clippy -p worldcraft-core -p worldcraft-ml --all-targets --locked
cargo check --profile ci -p worldcraft --all-targets --locked
cargo test --profile ci -p worldcraft --locked
cargo build --profile ci -p worldcraft --locked
```

Linux additionally needs native graphics libraries (see BUILDING.md). If using a
workspace-local pkg-config tree, set PKG_CONFIG_PATH for that workspace; do not modify
the owner's PC. The hosted workflow installs the standard native packages.

```sh
WGPU_BACKEND=vulkan LIBGL_ALWAYS_SOFTWARE=1 xvfb-run -a ./target/ci/worldcraft --calibration-audit
WGPU_BACKEND=vulkan LIBGL_ALWAYS_SOFTWARE=1 xvfb-run -a ./target/ci/worldcraft --eco-smoke-test
WGPU_BACKEND=vulkan LIBGL_ALWAYS_SOFTWARE=1 xvfb-run -a ./target/ci/worldcraft --auto-smoke-test
WGPU_BACKEND=vulkan LIBGL_ALWAYS_SOFTWARE=1 xvfb-run -a ./target/ci/worldcraft --smoke-test
xvfb-run -a python3 tools/ci/idle_audit.py ./target/ci/worldcraft
```

Use timeouts in CI. Remove/copy old captures to avoid accepting stale image files.
Preserve actual JSON reports/captures tied to the checked commit. Windows additionally
runs package/install/render scripts plus tools/test-windows-calibration.ps1 from the workflow. Software backends certify API
integration and visible output only; they cannot certify a gaming-FPS target.

## Next actions

1. Source 094ecd1 and run 43923268904 passed both native jobs. Evidence is in
   docs/evidence/adaptive-final-linux.json and adaptive-final-windows.json; actual
   final captures are in docs/media. Windows package bytes match the manifest.
   Linux's strict log gate found no hard errors and observed a 497.70 ms software
   ray dispatch/cleanup. Preserve the four subprocess harness regressions; older
   green jobs lacked this protection. No need to rerun unchanged checks without a
   new change, failure or unresolved concern. Do not confuse this alpha with the
   reference-quality target or gaming hardware certification.
2. Finish remaining M1 visual/interaction gates: low-budget horizon/outer fades,
   physical mouse/DPI painting and timed movement routes. Calibration/culling, native
   projection and Windows package checks already passed; preserve those regressions.
3. Record exact check results, links/artifacts and any failed tests in ADAPTIVE_AUDIT.md.
4. Capture deliberate observer movement across every topology threshold and near/far join,
   including negative coordinates and fast travel. Verify no terrain cracks and document
   late-refinement limitations. Add more endpoint invariants if a failure is found.
5. Keep the PR reviewable; do not merge/publish unverified packages.
6. Continue ENGINE_PLAN.md M2 then M3. Preserve version-1/2 saved worlds. A new hydrology
   generator requires a new version and explicit compatibility policy, not hidden edits.
7. Ray-based audio needs actual native audio output and bounded ray/proxy work; ordinary
   ambience must not be misrepresented as geometry-derived acoustics.

## Required handoff maintenance

Before stopping: update the audit and this file with current ref/PR, what passed,
what is incomplete, exact reproduction commands, known risks and the next concrete task.
Do not leave a green 'done' status while a relevant hosted check is pending or failing.

## Evidence and non-retained experiments

The first run's measured JSON and High/Home captures are in docs/evidence and docs/media,
with immutable source/run/checksums in ADAPTIVE_AUDIT.md. Local 82 tests include
27 core unit, 28 world integration, 9 ML and 18 desktop tests. Snapshot b98a9ee had 75
before the stricter 29-FPS and independent-vegetation regressions were added. Do not call the final Windows candidate
fully verified until its run finishes. Documentation-only changes do not restart the
native workflow; source changes queue behind running builds to preserve dependency caches.

A local no-display SwiftShader experiment captured scene pixels but hung at teardown;
its temporary offscreen application route was removed. Do not claim it passed or re-add
it without fixing lifecycle behavior. Hosted native graphics remain the supported QA route.

## Performance tradeoffs to measure next

32 m ground-cover batches tighten frustum bounds but add draw submissions when the whole
area is visible. Continuous terrain distance bands also retain more fine geometry than
legacy rings. Measure real CPU/GPU frame costs before tuning batch size or error thresholds;
keep near/far endpoint tests and collision consistency intact. The foliage refresh currently
rebuilds a bounded whole tile; a separate vegetation path can avoid resampling terrain later.

The latest native smoke writes foliage-render-audit.json after checking matching crown
transforms/margins, ground anchors and exclusion of duplicate ray occluders. The automatic
scene also round-trips three window cursor positions through the initialized image camera;
this verifies projection at 50% scale, not physical mouse/egui event delivery. Ground-anchor
and distance-gap/eviction invariants pass locally. These checks and the geometric crown revision passed on final source 094ecd1. Physical mouse
event delivery, temporal route appearance and gaming hardware still require the listed audits.

Native ray QA must observe a completed Solari GPU diagnostic on an adapter supporting
its required API. Unsupported adapters may explicitly skip; do not describe that skip
as a successful ray dispatch. The fixture has no saved world, so retain its explicit
camera-activation exception without allowing ordinary empty Home views to render.

Use tools/ci/run_native_audit.py for native Linux checks (see workflow). It retains
child output and fails on shader/pipeline validation errors, including those the render
cache logs while the app continues. Recoverable asynchronous library-import warnings
are not treated as final validation failures. Four harness tests exercise this boundary.

Current visual limits: outer dithering is still visible at small view radii/low resolution;
Eco has no far terrain and its boundary is visible from high viewpoints. Add a cheap
horizon/sky/fog fallback and distant forest representation before claiming seamless
whole-world appearance. See the rewritten ADAPTIVE_AUDIT.md for the current ledger.

## Historical start of nature realism branch (2026-11-17)

Follow-on branch: codex/worldcraft-nature-realism, PR #5 based on this adaptive branch.
Read docs/NATURE_AUDIT.md for changes, build preparation and the live verification ledger.
It preserves the adaptive/resource work and saved geography. New sources are tree.rs,
nature_texture.rs, terrain_texture.rs and nature_audit.rs. Foliage-only streaming now
retains terrain mesh identities; native smoke asserts those identities on approach/return.

Prepare CC0 embedded maps before desktop compilation (tools/prepare_materials.py).
The app remains entirely native/offline after building. Source art provenance and hashes
are checked in. Do not publish or describe this candidate as visually verified while
its native workflow is pending/failing. Update the audit with exact source/run results
and inspect actual captures; a percentage requested by the owner is not a test result.
