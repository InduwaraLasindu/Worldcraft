# Agent handoff: adaptive renderer and cinematic world program

## Active continuation — generator 3 ecology (2026-11-18)

Read `docs/BIOME_AUDIT.md` before the historical sections below. The active branch
and draft PR remain the same. Generator 3, weighted climate habitats, birch/acacia,
batched understory, alpha-cluster grass and a continuous HDR cloud pass are the new
candidate. Older 57b68b9 Windows/package results must not be described as validation
of the new executable. The 80% realism goal is still open; actual new images remain
below the reference target. Preserve legacy version-1/2 math and snapshots, work
budgets, persistent surface-frame/pipeline keys and software submission fence.

Current-source checks/evidence: see the final section of `docs/BIOME_AUDIT.md`.
Cloud boxes described below are historical. Do not restore those visual seams.

## Current continuation — nature alpha 4 (2026-11-17 UTC)

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

## Active continuation: nature realism branch (2026-11-17)

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
