# Procedural biome and ground-cover iteration

Updated 2026-11-18. Development candidate on `codex/worldcraft-nature-realism`,
draft PR #5. Executable source: `e21d5bcf11b0ec8d3621c8bd97ea5aafd934fe60`.
The earlier 57b68b9 executable's results are historical. Exact-source results,
capture provenance and remaining acceptance work are recorded below.

## Implemented

- Generator 3 keeps version 2's exact height backend and adds fourteen continuous
  climate habitat weights. Birch woodland, savanna, wet woodland and mountain meadow
  join the existing ten labels. Near and far terrain share the weighted palette.
  Meadows require moisture; arid slopes do not become flower meadow from altitude
  alone. Painted-biome labels follow the strongest resulting habitat weight,
  including soft and overlapping brush edges. Legacy paint rules are unchanged.
  Small neighboring forest weights no longer impose forest tree density on open
  meadows; the canopy response stays continuous and retains dense woodland.
- Region-scale domain warping, humidity, temperature, altitude and stand variation
  shape original multi-noise-inspired biomes. No Minecraft assets or code are used.
- Five shared tree families, each with four procedural prototypes: broadleaf, pine,
  palm, birch and acacia. Fine crowns still reach exact coarse geometry endpoints.
  Birch has shared original paper-bark textures; palms require tropical temperature.
- Ground populations include grass, shrubs, ferns, flowers and reeds. Shrubs branch
  into small leaves; ferns have pointed pinnate leaflets. Herb stems have explicit
  pigment, with normals checked against triangle winding.
- One shared procedural grass atlas replaces thick solid blades. Twenty cards per
  clump use 40 triangles and 80 indexed vertices, compared with 96 triangles and 160
  vertices in the first iteration. The same alpha texture is used by stock forward,
  shadow and depth passes. Its opaque white lane preserves other plants' vertex
  colors inside the same ground batch. Mips preserve cutoff coverage.
- Ground geometry remains in 32 m worker-built batches with terrain-seated roots,
  root-relative wind and tight bounds. No per-blade/flower/fern entities or materials.
- Static chunk roots and sun/light components retain their change ticks between
  actual changes. Density rejection precedes the four slope reads for ground cover.
- Alpha ground batches are excluded from secondary-ray proxies: the current ray
  backend has no material alpha-cutoff field, and solid card occluders would cast
  false rectangular shadows. Primary raster/depth/shadow masking is unchanged.
  Grass self-shadowing in experimental ray mode and accurate alpha-tested tree
  secondary surfaces remain open; the UI states the ground-cover limitation.
- High clouds use one continuous HDR full-screen pass before tonemapping, replacing
  the nine fog boxes. One 64-cubed R8 volume with seven mips uses 299,593 bytes.
  The bounded march uses 32 primary samples and two sunlight probes per occupied
  sample. Sky fill approximates multiple scattering; this is not a weather model,
  terrain cloud shadows or physically calibrated radiative transfer. Low/Balanced
  and inactive views omit the pass. Clouds and ground light shafts toggle separately.
- F2 photo view hides overlays without changing world generation or the renderer.
  F2/Esc restores controls. Painting is suspended while overlays are hidden.

## Compatibility and resource boundaries

New worlds use generator 3; saved version-1/2 worlds keep their exact legacy
generation, climate and placement rules. Legacy placement digests include count,
position, scale, yaw, species and variant for both versions and negative coordinates.
Generation 3 supports existing saved learned regions; no inference is needed to
reopen them. Project format and model recipes are unchanged.

An 8 m canopy/rock lattice and 2 m ground lattice cap a tile at 4,352 instances.
Ground detail is confined to the immediate ring; Eco retains its no-ground-cover
policy. Existing job/upload/cache/stale-result limits and the 48-byte persistent
surface frame remain intact. Software frame backpressure and the Windows 6 GiB
QA ceiling remain enabled. Structural bounds are not hardware FPS guarantees.

## Verification and visual review

The new `--biome-smoke-test` route searches untouched production worlds for six
biome sites across five seeds. It uses no authored plants or biome paint, captures
the production camera, and records coordinates, species counts, resident triangles,
cloud draw commands and queue state. `--nature-smoke-test` separately checks shared
assets, negative-coordinate rebase, different light/quality conditions, and a paired
cloud-off/cloud-on view. Image gates require visible cloud contribution as well as
nonempty rendering; they do not calculate a realism score.

Preliminary six-biome captures rendered successfully on hosted software Vulkan.
Review exposed sparse floor coverage, over-large shrub leaves, reversed grass
lighting normals, white herb stems and cloud sampling bands. Corrections were checked against the final native captures retained below; the
remaining visual gaps are still listed explicitly. The preliminary route's 1,716.43 MiB maximum
child RSS and 166.24 seconds are historical iteration measurements, not current
performance certification or gaming FPS.

The requested 80% realism target has **not** been certified. Current images still
show sparse/patchy understory, simple botanical forms, obvious outer dither fades,
smooth distant slopes and limited cloud detail. Water remains sea-level only.
Connected rivers, waterfalls, hydraulic erosion, far-forest impostors, moving-route
validation and real Windows gaming-GPU profiling remain open. Do not substitute
feature counts or green-pixel percentages for a reference-image visual assessment.

## Exact-source results

Executable source: `e21d5bcf11b0ec8d3621c8bd97ea5aafd934fe60`.
Executable tree: `bf69d8a58d13dd684ac9dd12eafaba5a83d42794`.
Subsequent evidence/documentation commits do not change that executable source.

| Check | Exact-source result |
|---|---|
| [Full native audit 44198247964](https://github.com/InduwaraLasindu/Worldcraft/actions/runs/44198247964) | Linux and Windows passed |
| [Native previews 44198247963](https://github.com/InduwaraLasindu/Worldcraft/actions/runs/44198247963) | Linux and Windows passed |
| Rust regressions on both platforms | 105 passed: 30 core unit, 5 ecology, 28 world integration, 9 ML, 33 desktop |
| Python checks on both platforms | 7 passed: 4 renderer-log gates and 3 runtime-bootstrap regressions |
| Formatting, optimized builds, core/ML Clippy | Passed; Clippy retains non-fatal style warnings |
| Native Linux | Launch calibration, frustum/quality behavior, Eco images, automatic quality, idle, eight scene states, rebase and ray cleanup passed |
| Native plant/cloud fixtures | 12 images; sky toggle changed 97.52% of the sky crop and recorded 23 real draw commands |
| Untouched biome route | 6 sites / 5 seeds; 25 resident chunks and zero queued jobs at every capture |
| Windows packaging | Portable/installed native runtime, safe uninstall and extracted portable learned-world rendering passed |

The Linux ray fixture observed a completed software Vulkan dispatch at
499.196309 ms over 75 frames and verified release of ray instances/pending work
after returning to raster. Windows' software DX12 adapter lacks the required ray
API and explicitly keeps raster. That skip is not ray execution or a gaming
benchmark. Ground cards deliberately do not cast secondary-ray shadows.

Both hosts performed three real native fast-model inferences, with repeated-seed
maximum error 0 and relief range 33.52–726.60 m. Linux took 8.48 seconds and Windows
8.76 seconds. This checks the coarse model, not the detailed cascade or GPU inference.

The idle Linux process sampled 385,312 KiB RSS (376.28 MiB), 0.038 logical CPU
cores over 10.01 seconds, with Home active and the world camera inactive. The
release Windows scene's twelve console samples reached a rounded 835 MiB.
The CI-profile Windows preview's thirteen console samples reached a rounded
1,497 MiB. These are separate software QA processes/profiles, not comparable
gaming performance or combined process-tree/VRAM budgets. Exact byte histories
are retained in the workflow artifacts; the unchanged 6 GiB scene QA gate passed.

Windows' eight-state test uses a 128 × 96 viewport for state/shader/memory checks;
it does not certify scenery appearance. The extracted portable additionally
rendered a 640 × 360 learned world and passed central-image variation plus exact
sculpt/undo/save/reopen gates. Its image and inner installer/ZIP bytes were not
independently downloaded/visually inspected after the development workspace ended.
The hosted install/extraction checks and GitHub outer-artifact metadata are the
verification scope for this new package.

[Windows candidate artifact](https://github.com/InduwaraLasindu/Worldcraft/actions/runs/44198247964/artifacts/14381876383):
92,005,849 bytes; GitHub outer ZIP SHA-256
`21953f12fb4791cd53e1015f25c90de1fb93b489310c6618a0b16109d9272480`.
It contains the portable ZIP, per-user alpha-4 installer, `dist/SHA256SUMS.txt`
and retained QA reports/logs. This is a development candidate, not a published release.
The full [evidence ledger](evidence/biome-e21d5bcf11b0ec8d3621c8bd97ea5aafd934fe60.json) records all four
artifact IDs, sizes, digests, exact source and per-platform material manifests.
Complete Linux and Windows job logs are retained beside it.

## Reproduction

Prepare the pinned embedded materials using `tools/prepare_materials.py` before
building. Run formatting, core/ML tests, desktop tests and the Python regressions
as specified in `.github/workflows/nature.yml`. Native graphics require the
Linux packages listed in that workflow. With the optimized CI executable:

```sh
WGPU_BACKEND=vulkan LIBGL_ALWAYS_SOFTWARE=1 xvfb-run -a python3 tools/ci/run_native_audit.py native-nature.log ./target/ci/worldcraft --nature-smoke-test
python3 tools/ci/check_nature_images.py
WGPU_BACKEND=vulkan LIBGL_ALWAYS_SOFTWARE=1 xvfb-run -a python3 tools/ci/run_native_audit.py native-biomes.log ./target/ci/worldcraft --biome-smoke-test
python3 tools/ci/check_biome_images.py
```

Use an isolated output directory and immutable executable; do not accept stale
captures or replace the running binary. The workflow preserves bounded timeouts,
strict renderer-log gates and all ordinary Eco/automatic/ray/learned checks.
Do not repeat passed unchanged checks without a new change or unresolved concern.

## Earlier rejected or superseded runs

Source `99aaf63fbdaf551215ad603d1d208a6da9e478b1` passed 103 Rust tests
and both native previews ([run 44193367189](https://github.com/InduwaraLasindu/Worldcraft/actions/runs/44193367189)).
Its full Linux and Windows jobs failed downloading the pinned runtime (HTTP 404);
that full run is rejected. Its local nature route used 1,401.11 MiB maximum child
RSS over 169.45 seconds, and the six-world route 1,870.34 MiB over 167.04 seconds.
These are historical software integration measurements, not current gaming FPS.

Source `97ae0ca0fbde19e4301298feb1f0e02b94e02141` added the moist-meadow,
brush-label and runtime fixes; its full run was superseded by the final source.
The final ray/clearing revision adds actual ECS checks for ground-proxy exclusion
and canopy continuity/density. An earlier local full scene run passed its scene
checks but failed relaunching an executable replaced by a concurrent rebuild.
That run is rejected. An isolated executable copy then passed the entire route,
including ray dispatch, raster cleanup and learned save/reopen. CI never rebuilds
its running executable.

## Runtime download repair

The official GitHub archive for ONNX Runtime 1.32.0 returned HTTP 404. The downloader
now falls back to the same-version official Microsoft NuGet package, without
changing the native inference backend. The package is 184,391,411 bytes, SHA-256
`a9150bc2992cbbb6722070da9910e1ffa989a8bfc67bdd5bb8e745ca755641db`.
Its SHA-512 and size matched the
[official NuGet catalog](https://api.nuget.org/v3/catalog0/data/2026.11.13.10.13.14/microsoft.ml.onnxruntime.1.32.0.json).
Only platform-selected native libraries and licenses are extracted. TLS and hash
checks remain enabled; checksum mismatch is a hard failure, and partial downloads
cannot replace a verified cache. No Python or managed runtime is shipped.

## Next visual acceptance work

All eighteen final-source PNGs were visually inspected, including all six untouched
biomes. [Original native capture gallery](BIOME_GALLERY.md) and
[immutable image provenance](evidence/nature-e21d5bcf11b0ec8d3621c8bd97ea5aafd934fe60.json)
retain the unedited output. The six ecosystem captures hide UI through the normal
photo mode; the twelve asset/quality fixtures intentionally retain their audit UI.

The ecology is more varied, but this is still below the owner's reference target.
Review found bare ground between repeated clumps, oversized/simple leaf clusters,
visible checkerboard outer vegetation fades, smooth largely bare distant slopes,
and cloud grain/structured sampling patterns with insufficient volume definition.
The artifact fixtures are useful engineering checks, not attractive showcase scenes.
Do not call this iteration 80% realistic or derive a score from image variation.

Priority order for the next implementation:

1. Improve continuous ground/material coverage and botanical scale while retaining
   bounded batches, shared prototypes and the Eco no-ground policy.
2. Add a bounded distant-forest representation and review moving crown/outer
   transitions at several resolutions; preserve endpoint/shadow invariants.
3. Improve cloud shape and sampling stability using paired and moving native views;
   keep explicit sample/memory budgets and no per-frame texture/material churn.
4. Implement versioned connected regional hydrology before promising rivers,
   waterfalls or erosion-derived drainage.
5. Measure fixed routes on real Windows gaming GPUs, including frame-time
   percentiles, RAM/VRAM, streaming stalls, temporal appearance and mouse/DPI editing.

Terrain PBR layers retain the embedded CC0 scanned forest/rock/grass maps from the
earlier candidate. Terrain layout, ecology, plant geometry, original leaf/bark/grass
atlases and cloud density are generated; this is not a claim that every texel is
procedural. Private reference images are not redistributed.

The hosted development workspace became unavailable after source and previews
were pushed. Hosted GitHub checks continued independently. Do not describe this as
a completed reference-quality release or an ongoing background development session.
