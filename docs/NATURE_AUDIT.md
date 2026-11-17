# Nature and surface realism iteration

Status: native validation in progress. This is an engineering alpha. A numerical
realism score is an editorial comparison, not a measured percentage of completion.
Reference pictures remain targets, not Worldcraft output.

## Implemented

- Original deterministic broadleaf scaffold/secondary branches, irregular conifer
  whorls and bowed palm fronds; species-specific woody meshes rather than a shared stem.
- Three shared alpha-masked foliage textures with linear-light, alpha-weighted mips
  and cutoff-coverage preservation. Broadleaf leaves, needles and palm leaflets have
  different silhouettes. The existing exact geometric crown endpoints remain intact.
- Normal-mapped original bark, correctly wrapped seam UVs and generated tangents.
- Two CC0 Poly Haven scanned material families, forest floor and rock, embedded at
  512 pixels with ten mips. Albedo is sRGB; OpenGL normals and roughness stay linear.
  The two texture arrays use 5,592,400 bytes together (about 5.33 MiB), excluding
  descriptor/driver overhead. Foliage and bark add their own small shared allocations.
- World-anchored ground mapping, triplanar cliff mapping, slope/vertex-palette blending,
  filtered normal/roughness detail and a Low procedural fallback. Explicit gradients
  allow conditional cliff sampling without invalid implicit derivatives.
- Bounded GPU foliage breeze with conservative bounds and shared shadow/forward phases.
  It pauses in experimental ray mode to avoid disagreement with static ray proxies.
- FXAA for active Balanced/High world views, including scaled scene targets.
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

    timeout 360s python3 tools/ci/run_native_audit.py native-nature.log xvfb-run -a ./target/ci/worldcraft --nature-smoke-test
    timeout 720s python3 tools/ci/run_native_audit.py native-scene.log xvfb-run -a ./target/ci/worldcraft --smoke-test

Use WGPU_BACKEND=vulkan and LIBGL_ALWAYS_SOFTWARE=1 for hosted Mesa. The art route
uses actual production meshes/materials on an Empty world canvas, so it isolates art
from ecology. It captures three species, bark, the coarse endpoint, a negative-world
origin rebase and sunset. It is not evidence of a generated dense forest or gaming FPS.
The full smoke separately checks normal generated worlds, unchanged terrain handles
on foliage refinement, edits, persistence, Low/Balanced/High, coast, ray dispatch and
raster cleanup. The workflow also preserves Eco, automatic-resolution and idle audits.

## Evidence ledger

- Local hosted core/ML regressions: 27 core unit + 28 world integration + 9 ML = 64 passed.
- Local hosted API check: passed.
- Local hosted desktop tests: 22 passed, including exact crown endpoints, bounded valid
  prototypes, alpha mip coverage, normal encoding and embedded-array layout.
- Local hosted desktop clippy: completed with style warnings, including prior code.
- First remote source 5480ef8 predates the explicit Option<f32> test annotation fix and
  the scanned surfaces/performance patch. Its final result must not certify later code.
- Current remote candidate: see the Nature realism audit workflow and this branch's
  immutable source SHA. Native results and reviewed captures will be recorded here
  after completion, including failures.

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
[Rock Ground](https://polyhaven.com/a/rock_ground), Poly Haven, CC0-1.0.
Their scanned albedo, OpenGL normal and roughness maps are reduced into bounded arrays.
Original botanical geometry, foliage and bark textures are generated in Rust.
