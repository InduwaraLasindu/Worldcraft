# Performance and evidence

No gaming-GPU FPS, memory-budget, VRAM-use or photorealism target is verified yet.
Software Vulkan on a hosted runner is an integration test, not a gaming benchmark.

## Machine-local resource budgets (alpha 3)

| Budget | Near radius maximum | Terrain jobs | Uploads/frame | New-job interval | AI threads maximum |
| --- | ---: | ---: | ---: | ---: | ---: |
| Eco | 2 (25 chunks) | 1 | 1 | 150 ms | 1 |
| Balanced | 6 (169 chunks) | 2 | 1 | 35 ms | 2 |
| Performance | 12 (625 chunks) | 4 | 2 | 0 ms | 4 |

Automatic starts conservatively, renders a short isolated foliage fixture on launch,
and samples terrain work on one background worker. After warmup it uses p90 wall-clock
frame intervals, actual adapter type, CPU threads and available/total memory to choose
an initial tier. Software adapters and scarce-memory/small-CPU systems have lower
ceilings. This is a small calibration, not a representative gaming benchmark.

| Automatic tier | Graphics | Near radius | 3D scale | Work budget |
|---|---|---:|---:|---|
| 0 | Low | 2 | 50% | Eco |
| 1 | Low | 2 | 67% | Eco |
| 2 | Low | 4 | 85% | Balanced |
| 3 | Balanced | 6 | 100% | Balanced |
| 4 | High | 9 | 100% | Performance |

During focused exploration, a two-second p90 window (at least twelve samples) lowers
one tier after sustained budget overruns (two percent timing tolerance). Recovery needs twelve seconds of headroom,
with additional change cooldowns. The target is 30 FPS, or a lower explicitly selected
FPS cap. The settings panel reports effective graphics, radius, scale, recent p90 and
when minimum quality remains below target. Manual modes preserve user graphics choices.
No hardware, total RAM or model-name heuristic can guarantee 30 FPS on arbitrary PCs.

All modes use Eco protection below 1 GiB available RAM, recovering above 1.5 GiB.
These are sampled system-memory/work budgets, not hard process allocator/VRAM limits.
The full 3D pipeline renders at a reduced image resolution on lower tiers while UI
remains sharp. This reduces scene textures/post-processing as well as shading work;
actual savings depend on the renderer/backend. Cursor picking uses the scaled viewport.

Manual Eco keeps Low graphics, simpler crowns, no grass/shrubs or far terrain, finest
collision geometry near the player, and a 30 FPS cap. Automatic's lower tiers can render
up to the selected FPS ceiling while evaluating available headroom. Home caps at 20 FPS
with a reactive idle wait and a separate UI camera, disables world rendering, and runs
only its short calibration scene when necessary. Unfocused windows pause 3D/new terrain
work; the calibration camera also pauses when unfocused. Existing synchronous jobs can
finish; old epochs remain counted. A settled plan avoids rebuilding/sorting when the
observer and desired keys have not changed.

Default Bevy worker pools reserve capacity for the rest of the computer and cap
async workers at one or two. Pools are created at startup; changing a runtime
budget limits new work but does not rebuild those pools. AI sessions obey the
selected thread budget for the next generation, disable idle thread spinning and
pause 40 ms between model calls in Eco. A native call is not preemptible mid-call.
Detailed AI admission checks require 6 GiB available RAM immediately before start;
this cannot guarantee memory remains available during an entire bake. Saved AI
examples and classical terrain need no AI inference. A saved world can be unloaded
from Home after edits are saved to release scene and generator references.

## Other controls

- Up to four asynchronous near-chunk generation tasks, plus at most one far-mesh job when enabled.
- Two completed terrain uploads maximum per frame.
- 2–12 chunk view radius, nearest chunks requested first.
- 32/16/8/4 cells per terrain chunk side internally; continuously morphed positions,
  normals and colors instead of an instantaneous visible topology switch.
- Shared distance function in forward/depth/shadow passes; near/far morph collar.
- Ground-cover geometry built in background workers and uploaded in 32 m batches for
  tighter engine frustum bounds. Trees/rocks retain shared mesh/material batching.
- Full-scene scaled image target, composited beneath native-resolution UI.
- Shared tree geometry/materials; merged local grass and shrub geometry.
- Simplified distant crowns and no farthest-ring scatter. Vegetation detail invalidates
  independently of terrain topology so near representations refresh as the observer moves.
- Terrain/entities evicted outside the desired square.
- Brush changes invalidate affected chunks and their sampling halo instead of the whole world.
- Immutable generation snapshots; obsolete task keys discarded.
- Background world creation, file I/O and fast analytic regional map previews.
- Large map views do not bake or evict the exploration terrain cache.
- Shared 32-tile LRU terrain cache, single-flight region bakes and spatially indexed edits.
- No disk terrain cache or mandatory model download.
- Learned regional relief is baked once; exploration performs no inference.
- Optional detailed inference drops each model session before loading the next; downloads stream through a 64 KiB buffer.
- Detailed regions retain a finite 320 × 320 height grid alongside the coarse climate/relief field.
- Ray mode converts at most two unique meshes and adds at most sixteen proxies per frame.
- Ground/water detail filters subpixel frequencies and skips invisible terrain noise.

The UI reports measured smoothed FPS, the last completed generation time, resident chunks
and active job count. It does not invent CPU/GPU timings, VRAM usage or completion percentages.

## Known scaling limits

The hidden 400 × 300 multiworld WARP check exhausted the hosted software adapter's
memory. The bounded QA route uses 128 × 96 for API/state checks and a separate
640 × 360 packaged-example capture for visible scene review. These are integration
tests; they do not establish a software-rendering or gaming-GPU memory budget.


Brush sampling replays spatially indexed local candidates in their saved order. Snapshots
still clone and re-index edit metadata, so long histories can cause main-thread work.
Compact edit checkpoints and incremental index updates remain future work. The current
100,000-dab limit has not been stress-tested.

Retained cached height arrays occupy about 1.03 MiB (32 × 65 × 65 × 8 bytes). Mutex/map
metadata, single-flight slots, active worker references and simulation buffers are additional.
This is an implementation bound, not a measured process-memory claim. Dense foliage,
ocean surfaces, procedural fragment detail and SSAO can be expensive on weak GPUs.

Terrain LOD changes replace geometry, so popping is possible. A future geomorphing renderer
can improve this. Skirts are a transient crack concealment strategy, not proof of perfect
LOD appearance under every edit and travel speed.

Bevy task cancellation cannot preempt synchronous work that is already executing. Bounding
the queue limits outstanding work, while generation epoch/key checks prevent stale results
from replacing a new world.

## Measurement plan

Record commit, OS/driver, adapter, resolution, seed/settings, camera route, graphics preset,
view radius and vegetation density. Capture median/p95/p99 frame time, startup-to-first-terrain,
generation latency, resident RAM/VRAM and streaming stalls. Compare identical routes and
avoid deriving a performance claim from one screenshot or average FPS alone.

## Detailed inference fixture

The alpha-2 Linux audit recorded VmHWM 4,648,244 kB (about 4.43 GiB) while running
three complete native detailed generations and cancelling a fourth during base
refinement. That is process resident high-water memory for one hosted CPU fixture,
not total machine memory, a hardware minimum or a Windows/GPU budget. Model creation
runs in the background and saved-world exploration uses the baked field. The default
fast model is 22.5 MB; the optional complete detailed set is 2.28 GB. Actual findings
and caveats are recorded in [the audit](docs/ALPHA2_AUDIT.md).

## Current audit and continuation

See [adaptive audit](docs/ADAPTIVE_AUDIT.md) for evidence and pending checks, and
[agent handoff](docs/AGENT_HANDOFF.md) for exact continuation tasks. The standard hosted Mesa Vulkan and Windows WARP routes validate native scene/UI output.
A local SwiftShader offscreen experiment produced pixels but timed out during teardown;
it was not retained as an application test mode or counted as a passing full audit.
