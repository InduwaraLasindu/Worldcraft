# Performance and evidence

No gaming-GPU FPS, memory-budget, VRAM-use or photorealism target is verified yet.
Software Vulkan on a hosted runner is an integration test, not a gaming benchmark.

## Machine-local resource budgets (alpha 3)

| Budget | Near radius maximum | Terrain jobs | Uploads/frame | New-job interval | AI threads maximum |
| --- | ---: | ---: | ---: | ---: | ---: |
| Eco | 2 (25 chunks) | 1 | 1 | 150 ms | 1 |
| Balanced | 6 (169 chunks) | 2 | 1 | 35 ms | 2 |
| Performance | 12 (625 chunks) | 4 | 2 | 0 ms | 4 |

Automatic selects Eco for at most four logical CPU threads, less than 9 GiB total
RAM, less than 3 GiB available RAM, unavailable memory information or an actual
integrated/software render adapter. Otherwise it selects Balanced. All modes use
Eco protection below 1 GiB available, recovering above 1.5 GiB. This is a sampled
system-memory heuristic, not a process allocator limit or performance guarantee.

Eco uses Low graphics, simpler crowns, skips grass/shrubs and far terrain, retains
LOD0 collision geometry and caps exploration at 30 FPS. FPS is configurable from
15 to 120; Eco imposes its own 30 FPS ceiling. Home caps at 20 FPS with a reactive
250 ms idle wait and a separate UI camera, and disables the 3D camera. Unfocused
windows disable 3D, pause new terrain work and use reactive 500 ms updates. Existing
synchronous jobs can finish; old epochs stay counted until completion. A settled
streaming plan avoids rebuilding and sorting desired keys every frame.

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
- 32/16/8/4 cells per terrain chunk side.
- Shared tree geometry/materials; merged local grass and shrub geometry.
- Simplified distant crowns and no farthest-ring scatter.
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
