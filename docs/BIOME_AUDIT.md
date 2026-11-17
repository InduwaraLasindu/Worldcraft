# Procedural biome and ground-cover iteration

Updated 2026-11-18. Development candidate on `codex/worldcraft-nature-realism`,
draft PR #5. New source supersedes the 57b68b9 executable; that older executable's
Windows/package passes do not certify these changes. Current-source results and
immutable capture provenance are recorded below when available.

## Implemented

- Generator 3 keeps version 2's exact height backend and adds fourteen continuous
  climate habitat weights. Birch woodland, savanna, wet woodland and mountain meadow
  join the existing ten labels. Near and far terrain share the weighted palette.
  Meadows require moisture; arid slopes do not become flower meadow from altitude
  alone. Painted-biome labels follow the strongest resulting habitat weight,
  including soft and overlapping brush edges. Legacy paint rules are unchanged.
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
lighting normals, white herb stems and cloud sampling bands. Fixes are being
validated against new native captures. The preliminary route's 1,716.43 MiB maximum
child RSS and 166.24 seconds are historical iteration measurements, not current
performance certification or gaming FPS.

The requested 80% realism target has **not** been certified. Current images still
show sparse/patchy understory, simple botanical forms, obvious outer dither fades,
smooth distant slopes and limited cloud detail. Water remains sea-level only.
Connected rivers, waterfalls, hydraulic erosion, far-forest impostors, moving-route
validation and real Windows gaming-GPU profiling remain open. Do not substitute
feature counts or green-pixel percentages for a reference-image visual assessment.

## Current-source results

Source `99aaf63fbdaf551215ad603d1d208a6da9e478b1` passed 103 Rust tests,
the optimized hosted build, formatting, Clippy (non-fatal style warnings), four
renderer-log tests, twelve native art views and six untouched procedural worlds.
The local nature route used 1,401.11 MiB maximum child RSS and 169.45 seconds;
the six-world route used 1,870.34 MiB and 167.04 seconds. These are software
integration measurements, not gaming FPS or combined process-tree budgets.
The cloud pair changed 97.50% of its sky ROI and recorded 23 actual draw commands.
The corresponding [native preview run](https://github.com/InduwaraLasindu/Worldcraft/actions/runs/44193367189)
passed on Linux and Windows. Both full jobs stopped at the pinned runtime's
HTTP-404 download, before later renderer/package checks; that full run is rejected.

The follow-up fixes the two ecological review findings above and adds a pinned
official Microsoft NuGet fallback for the same ONNX Runtime 1.32.0 version. The
package is 184,391,411 bytes, SHA-256
`a9150bc2992cbbb6722070da9910e1ffa989a8bfc67bdd5bb8e745ca755641db`.
Its SHA-512 and size matched the [official NuGet catalog](https://api.nuget.org/v3/catalog0/data/2026.11.13.10.13.14/microsoft.ml.onnxruntime.1.32.0.json).
Only selected native libraries and licenses are extracted. TLS and hash checks
remain enabled; a checksum mismatch stays a hard failure. Partial downloads do
not replace a verified cache. No Python or managed runtime is shipped.

Follow-up local checks: 104 Rust tests and seven Python checks pass. The fallback
native library passes offline installation validation and three real coarse
inferences (7.47 seconds total; repeated-seed maximum error 0; relief range
33.52–726.60 m). A new untouched moist-meadow capture rendered successfully.
The final full Linux/Windows rerun and immutable new capture provenance remain
pending until the exact follow-up source is published and checked.
