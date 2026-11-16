# Agent handoff: adaptive renderer and cinematic world program

Updated: 2026-11-16. PR #4 (draft, based on PR #3). Active branch: `codex/worldcraft-adaptive-quality`.
Base: `codex/worldcraft-resource-limits`, commit ca2837b (PR #3), on alpha-2 main.
This is a follow-on feature branch; do not overwrite or silently discard the previous
resource-budget work. Do not claim the reference-quality natural-world engine is finished.

## Read first

1. Root AGENTS.md: cloud development, native Rust, free public CI, bounded streaming,
   compatibility and honest feature claims.
2. ARCHITECTURE.md and GENERATION.md: core/render separation, generator 1/2, formats 1/2/3,
   explicit edits, baked learned terrain and physics limits.
3. docs/ENGINE_PLAN.md: complete requested scope, dependencies and acceptance gates.
4. docs/ADAPTIVE_AUDIT.md: actual evidence for the current code and remaining checks.
5. PERFORMANCE.md, MASTER_SPEC_COVERAGE.md and ROADMAP.md: earlier baseline and limits.

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
| app/streaming.rs | Retains tiles during budget replacements; worker-built 32 m ground batches |
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
- UI camera reserves render layer 30, and the launch fixture uses 31. World meshes stay
  on layer 0 so the UI camera does not perform their visibility work while Home is open.
- Resolution uses a render target plus egui compositing, not window resizing. UI stays
  sharp, and picking must convert window logical coordinates to camera logical size.
- Calibration is a small real scene, not a universal GPU benchmark. It has shader warmup
  uncertainty and a short interval; below twelve samples use conservative selection.
  Width caps are 256/640/1280 for software/constrained/other adapters. Software is always
  tier 0, including when the user opens a world before the launch probe completes.
  Live adaptation is required after launch. Probe dimensions/sample counts are in the report.
- Low RAM triggers emergency Eco. No claim of a hard allocator/VRAM or CPU-% cap.
- Manual settings skip automatic tuning; pressure protection is retained.
- Pending synchronous jobs from old epochs still count until completion. Never drop
  their accounting merely because their output will be discarded.
- Vegetation crowns/density are still discrete. Teleports/late refinement and horizon
  rebakes can still produce changes. The entire world is not proven pop-free.
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

1. Check the newest queued/final run. Initial c24a509 passed Linux and Windows render/image
   checks; optional GH_TOKEN evidence publication then failed. Hardened b98a9ee (run
   43897987803) passed the full Linux job, Windows compilation/tests/inference, and Windows
   launch/frustum audit; its packaging step was still in progress at the last update.
   Final source also tightens the 30-FPS controller and gives the detailed Windows helper
   a unique C# type name. The publisher guard is fixed. Do not assume the final run passed.
2. Finish M1 integration/visual audits; inspect calibration/culling, 50% automatic scene,
   mode changes, scaled picking and the detailed Windows portable package before expanding
   unrelated graphics work.
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
with immutable source/run/checksums in ADAPTIVE_AUDIT.md. Local 76 tests include
27 core unit, 28 world integration, 9 ML and 12 desktop tests. Snapshot b98a9ee had 75
before the stricter 29-FPS regression was added. Do not call the final Windows candidate
fully verified until its run finishes. Documentation-only changes do not restart the
native workflow; source changes queue behind running builds to preserve dependency caches.

A local no-display SwiftShader experiment captured scene pixels but hung at teardown;
its temporary offscreen application route was removed. Do not claim it passed or re-add
it without fixing lifecycle behavior. Hosted native graphics remain the supported QA route.
