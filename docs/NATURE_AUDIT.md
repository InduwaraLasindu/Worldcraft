# Nature and surface realism iteration

Updated 2026-11-17. Current executable source: `e48b88f6647510109b54efef286d6da689f1a001`.
Branch: `codex/worldcraft-nature-realism`, draft PR #5.
Status: native/package validation in progress. This is an engineering alpha. A numerical
realism score is an editorial comparison, not a measured percentage of completion.
Reference pictures remain targets, not Worldcraft output.

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
- Local hosted desktop tests on current source: 28 passed, including exact crown
  endpoints, bounded prototypes, alpha mip coverage, normal encoding, embedded arrays,
  software shadow budgets, shared-frame/no-material-mutation and cloud-cell invariants.
- Local hosted desktop clippy: completed with style warnings, including prior code.
- First remote source 5480ef8 predates the explicit Option<f32> test annotation fix and
  the scanned surfaces/performance patch. Its final result must not certify later code.
- Current remote candidate: see the Nature realism audit workflow and this branch's
  immutable source SHA. Native results and reviewed captures will be recorded here
  after completion, including failures.

Current source also passed local API/all-target compilation, formatting and desktop
clippy (style warnings remain), plus all eleven production-asset captures through
the strict native shader-error gate on software Vulkan. It does not inherit a
Windows package approval from earlier code. The current full native run is
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
is added in this iteration. Outer dithering, low-budget horizons, physical mouse/DPI
coverage and gaming-GPU profiling from ADAPTIVE_AUDIT.md are still outstanding.

The experimental ray proxies are static copies with averaged vertex albedo. They do
not reproduce every raster procedural material or continuous morph. Do not certify
ray-path visual parity or gaming performance from software Vulkan/WARP results.

## Source art

[Forest Ground 01](https://polyhaven.com/a/forrest_ground_01) and
[Rock Ground](https://polyhaven.com/a/rock_ground) and
[Leafy Grass](https://polyhaven.com/a/leafy_grass), Poly Haven, CC0-1.0.
Their scanned albedo, OpenGL normal and roughness maps are reduced into bounded arrays.
Original botanical geometry, foliage and bark textures are generated in Rust.
