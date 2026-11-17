# Nature and surface realism iteration

Generator-3 continuation (2026-11-18): see [BIOME_AUDIT.md](BIOME_AUDIT.md).
The source/results below describe the previous executable and remain historical
evidence. They do not certify the new ecology/ground/cloud changes.

Updated 2026-11-17. Current executable source: `57b68b97330e57905362bcd29e231bcbb102085d`.
Branch: `codex/worldcraft-nature-realism`, draft PR #5.
Status: hosted Linux/Windows native and package validation passed. This is an
engineering alpha. A numerical
realism score is an editorial comparison, not a measured percentage of completion.
Reference pictures remain targets, not Worldcraft output.

## Current software-frame follow-up

Source `e48b88f` passed Linux but failed the Windows 6 GiB memory gate at
6452.07 MiB sampled resident memory. Source `34bb65c` selected CPU mesh
preparation on software adapters and passed Linux, but its release-profile
Windows run also failed, at 6504.82 MiB after 35 samples. Those packages remain
rejected. CPU preparation alone is not a confirmed Windows memory fix.
The full failed memory history and native log are retained in
[software-preparation-34bb.json](evidence/software-preparation-34bb.json).

Current source retains the CPU choice before PBR plugins select their systems,
removing the RenderStartup initializer that would otherwise restore GPU
preparation. It also waits for submitted software-adapter work after the render
stage and before cleanup/next-frame allocation. This tests the suspected hidden
swap-chain submission backlog. Hardware adapters keep their asynchronous GPU
path; GPU/ray compute features remain available.
Both Windows profiles now pass the unchanged 6 GiB graphics/memory gate:

| Profile | Software QA window | Sampled peak RSS | Samples |
|---|---|---|---|
| CI preview | 128x96 | 1645.54 MiB | 11 |
| Release | 128x96 | 876.17 MiB | 16 |

These tiny window captures mostly show UI; they check scene/shader/resource
integration. The packaged Windows detailed-world route separately passed at
640x360, including sculpt/undo/exact save/reopen and a visibly rendered world.
Its larger viewport has no separate process-memory measurement. Hardware gaming
FPS/VRAM and physical mouse/DPI interaction remain unmeasured.


Eco uses matching background/fog colors and a horizontal 78.4–224 m fade for its
25 resident chunks. Near-ground detail stays clear; flight altitude does not add
to the boundary distance. Underwater absorption and other performance modes keep
the ordinary 3D fog. The reserved fourth animation word carries the horizon flag
in the existing 48-byte frame; toggles do not mutate materials or pipeline keys.
An elevated and ground-level native fixture checks that the edge is covered and
the landscape remains visible. The current source passed with a 0.54-level maximum
horizon row jump and 14.00 foreground luma standard deviation. The former hard edge
was 45.30; a rejected radial-fog trial erased that foreground to 2.00. These fixed
camera checks are not visual-quality or moving-route certification.

Current local totals: 64 core/ML plus 30 desktop tests (94 distinct Rust tests),
plus four log-gate tests. Formatting and the optimized CI-profile build passed.
The new full run is
[44152973365](https://github.com/InduwaraLasindu/Worldcraft/actions/runs/44152973365);
[44152973239](https://github.com/InduwaraLasindu/Worldcraft/actions/runs/44152973239)
provides Linux art and Windows CI-profile graphics/memory previews; both passed.
The full local native scene and Eco routes passed, including a completed
996.01 ms software ray dispatch and raster cleanup. Both full hosted jobs passed.
The hosted Linux ray dispatch completed in 1147.97 ms; idle Home used 0.047 logical
CPU cores and 375.53 MiB sampled peak RSS over ten seconds, with the world camera
inactive. Automatic rendering retained its software floor and exact saved world.
The 6 GiB ceiling and all native scene,
quality-switch, ray, persistence and image assertions remain enabled.
Current-source preview hashes, full local log, Eco metrics and Windows memory
samples are in [software-frame-57b68b9.json](evidence/software-frame-57b68b9.json).
The downloaded Windows artifact, portable ZIP and installer matched their SHA-256
values; BUILD-INFO identifies the exact executable source. The x64 executable,
native runtime/compilers and material/dependency notices were present. Hosted
portable/installed runtime checks and safe uninstall passed. Portable ZIP SHA-256:
`c0820ae6db240e689e5c7276b764003498ed37e343f016f0e57b5f0a3c489450`;
installer SHA-256:
`32c7967cced50082a628289bc00a78fd4ccae9fa55009230f073695aead99fe6`.
The build remains a development alpha; no main merge or public release is made.
All eleven current art images were reviewed and retained with
[provenance](evidence/nature-57b68b97330e57905362bcd29e231bcbb102085d.json).
[Current forest](media/nature/57b68b97330e57905362bcd29e231bcbb102085d/worldcraft-nature-forest-high.png)
and [current clouds](media/nature/57b68b97330e57905362bcd29e231bcbb102085d/worldcraft-nature-clouds-high.png)
show the actual engineering alpha. The
[packaged Windows view](media/nature/57b68b97330e57905362bcd29e231bcbb102085d/worldcraft-detailed-windows.png)
is also retained. Sparse undergrowth, tan forest-floor coverage,
small coarse silhouettes and dotted cloud-cell artifacts remain visible. The
reference-realism target has not been certified. Evidence below retains its exact
earlier source labels.

## Implemented

- Experimental High cloud layer: nine nonoverlapping 8 km cells share a 64³ R8
  density field (256 KiB) in the existing 24-step volumetric integrator. Their
  drifting phase is world-anchored; disabled cells are hidden and Low/Balanced
  perform no cloud-volume rendering. Cell size/attenuation and an explicitly
  artistic light compensation correct the rejected dark-ceiling checkpoint.
  No cloud self-shadowing,
  physical weather simulation or cloud-to-terrain shadows are claimed.

- Original deterministic broadleaf scaffold/secondary branches, irregular conifer
  whorls and bowed palm fronds; species-specific woody meshes rather than a shared stem.
- Three shared alpha-masked foliage textures with linear-light, alpha-weighted mips
  and cutoff-coverage preservation. Broadleaf leaves, needles and palm leaflets have
  different silhouettes. The existing exact geometric crown endpoints remain intact.
- Normal-mapped original bark, correctly wrapped seam UVs and generated tangents.
- Three CC0 Poly Haven scanned material families, forest floor, leafy grass and rock, embedded at
  512 pixels with ten mips. Albedo is sRGB; OpenGL normals and roughness stay linear.
  The two texture arrays use 8,388,600 bytes together (about 8.00 MiB), excluding
  descriptor/driver overhead. Foliage and bark add their own small shared allocations.
- World-anchored ground mapping, triplanar cliff mapping, slope/vertex-palette blending,
  filtered normal/roughness detail and a Low procedural fallback. Explicit gradients
  allow conditional cliff sampling without invalid implicit derivatives.
- Bounded GPU foliage breeze with conservative bounds and shared shadow/forward phases.
  It pauses in experimental ray mode to avoid disagreement with static ray proxies.
- Corrected full-sun camera exposure (EV15 baseline, with a half-stop default offset), curved terrain-seated grass clumps,
  bounded ground breeze and scanned triplanar boulder surfaces.
- FXAA for active Balanced/High world views, including scaled scene targets.
- Immutable surface-kind shader specialization removes terrain sampling from
  foliage/water pipelines. One persistent 48-byte storage buffer updates origin,
  observer/morph range and wind/water/quality data; material assets stay unchanged
  during animation and movement. Forward/depth/shadow passes share those values.
- Manual High on software adapters caps shadow maps at 1024; hardware retains
  its 2048/4096 Balanced/High choices. Windows QA records complete logs and sampled
  process memory, failing above its explicit 6 GiB software integration ceiling.
- Foliage-only streaming updates retain the exact terrain mesh and chunk root.
  Their background task skips terrain sampling and GPU terrain uploads. Epoch,
  revision, residency and stale synchronous-job accounting remain enforced.

This changes visual assets and materials, not generator versions, durable tree/rock
positions, world formats or explicit edits. Collision still uses conservative stem
cylinders and terrain; it is not a full branch/crown physics representation.

## Reproduction

The only added build tools are Python, Pillow and numpy. They prepare material data;
they are not shipped or invoked by the application. All source map URLs, exact sizes
and SHA-256 values are in tools/material_sources.json. Downloads use TLS, bounded sizes,
checksums and atomic replacement. The executable embeds prepared bytes and works offline.

    python -m pip install Pillow==11.3.0 numpy==2.3.5
    python tools/prepare_materials.py
    cargo fmt --all -- --check
    cargo test -p worldcraft-core -p worldcraft-ml --locked
    cargo check --profile ci -p worldcraft --all-targets --locked
    cargo test --profile ci -p worldcraft --locked
    cargo build --profile ci -p worldcraft --locked

Native Linux art and integration routes:

    timeout 480s python3 tools/ci/run_native_audit.py native-nature.log xvfb-run -a ./target/ci/worldcraft --nature-smoke-test
    timeout 720s python3 tools/ci/run_native_audit.py native-scene.log xvfb-run -a ./target/ci/worldcraft --smoke-test

Use WGPU_BACKEND=vulkan and LIBGL_ALWAYS_SOFTWARE=1 for hosted Mesa. The art route
uses actual production meshes/materials on an Empty world canvas, so it isolates art
from ecology. It captures three species, bark, the coarse endpoint, a negative-world
origin rebase, sunset, close/overlook views of a seeded generated forest and
dedicated High forest/daytime-cloud views. The image gate rejects bare canopies
and an excessively black daytime cloud layer.
The art gallery itself isolates production assets on an Empty canvas; the forest
views use the normal generator. Neither route is a gaming-FPS benchmark.
The full smoke separately checks normal generated worlds, unchanged terrain handles
on foliage refinement, edits, persistence, Low/Balanced/High, coast, ray dispatch and
raster cleanup. The workflow also preserves Eco, automatic-resolution and idle audits.

## Evidence ledger

Visual rejection: source e2c4bb3 / preview run 44126402747 exited successfully but
its captures showed missing foliage. Its log contained a WESL reserved identifier
(`macro`) that the existing audit regex did not recognize. Those captures are rejected,
not proof of rendering. The identifier and log gate are corrected; native captures
must be rerun before this candidate is accepted. The harness now rejects both WESL
processing and resolution errors even when the process exits zero.

- Local hosted core/ML regressions: 27 core unit + 28 world integration + 9 ML = 64 passed.
- Local hosted API check: passed.
- Local hosted desktop tests on captured source e48b88f: 28 passed, including exact crown
  endpoints, bounded prototypes, alpha mip coverage, normal encoding, embedded arrays,
  software shadow budgets, shared-frame/no-material-mutation and cloud-cell invariants.
- Local hosted desktop clippy: completed with style warnings, including prior code.
- First remote source 5480ef8 predates the explicit Option<f32> test annotation fix and
  the scanned surfaces/performance patch. Its final result must not certify later code.
- Current remote candidate: see the Nature realism audit workflow and this branch's
  immutable source SHA. Native results and reviewed captures will be recorded here
  after completion, including failures.

Captured source e48b88f also passed local API/all-target compilation, formatting and desktop
clippy (style warnings remain), plus all eleven production-asset captures through
the strict native shader-error gate on software Vulkan. It does not inherit a
Windows package approval from earlier code. That full native run was
[44140811487](https://github.com/InduwaraLasindu/Worldcraft/actions/runs/44140811487);
the source-bound preview run is
[44140811489](https://github.com/InduwaraLasindu/Worldcraft/actions/runs/44140811489).

That preview passed all 28 desktop tests and retained eleven captures, including
High forest and daytime sky. [Source-bound capture hashes](evidence/nature-e48b88f6647510109b54efef286d6da689f1a001.json)
and the [local full-scene ledger](evidence/nature-milestone.json) are retained.
The local full smoke passed edit/undo/save/reopen, independent vegetation refinement,
quality/home/world/origin changes, coast/map and learned save/reopen. It observed
a completed 1081.41 ms software ray dispatch and subsequent raster cleanup.

Visual correction: source `2273d7d` passed its shader gate and 27 desktop tests,
but the new sky composition exposed a black ceiling. Its daytime sky ROI had
8.11% pixels below luminance 35; the corrected local cloud/shared-frame captures
have 0.00%. The image gate now rejects above 4%. This gate detects that regression;
it is not a photorealism metric. Both sets of actual images were reviewed.
[Rejected sky](media/nature/2273d7d944614b2c4fc84891b89cf45f4912159c/worldcraft-nature-clouds-high.png)
and [corrected sky](media/nature/e48b88f6647510109b54efef286d6da689f1a001/worldcraft-nature-clouds-high.png)
are retained for direct comparison.

Windows rejection: source `fcc13f4`, run `44129931422`, built and passed its Rust
tests but exhausted memory during software DX12 rendering after switching High.
Resident samples grew from 490 MiB to 9580 MiB. Those packages are not certified.
Current shadow/frame/shader changes require their own complete Windows rerun.

Fixed material checkpoint: source 5a09d5a / preview run 44127333571 rendered the
three species, bark, coarse view, rebase and sunset with no hard shader errors.
Its verified artifact digest is 5c0036fb928d8ff220c28130523718ecbcdf2b7e91a44340845d4e5963facdae.
Actual images were reviewed. They showed the need for a corrected daylight exposure,
closer vegetation inspection and better ground clumps; those follow-on changes require
their own native rerun. The revised image gate distinguishes the rejected bare captures
(0 canopy-colored pixels under its stricter color criterion) from the fixed ones.

## Remaining visual and performance limits

Canopy art, ecosystem layering and ground-cover density need actual close and moving
views. No connected hydrology, new hydraulic solver, snow/weather or acoustic engine
is added in this iteration. Eco has a tested fixed-camera horizontal fade; outer dithering and moving-camera
horizon transitions still need review. Physical mouse/DPI coverage and gaming-GPU
profiling from ADAPTIVE_AUDIT.md remain outstanding.

The experimental ray proxies are static copies with averaged vertex albedo. They do
not reproduce every raster procedural material or continuous morph. Do not certify
ray-path visual parity or gaming performance from software Vulkan/WARP results.

## Source art

[Forest Ground 01](https://polyhaven.com/a/forrest_ground_01) and
[Rock Ground](https://polyhaven.com/a/rock_ground) and
[Leafy Grass](https://polyhaven.com/a/leafy_grass), Poly Haven, CC0-1.0.
Their scanned albedo, OpenGL normal and roughness maps are reduced into bounded arrays.
Original botanical geometry, foliage and bark textures are generated in Rust.
