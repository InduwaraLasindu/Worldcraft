# Performance and evidence

No gaming-GPU FPS, memory-budget, VRAM-use or photorealism target is verified yet.
Software Vulkan on a hosted runner is an integration test, not a gaming benchmark.

## Current controls

- Four asynchronous generation tasks maximum.
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

The UI reports measured smoothed FPS, the last completed generation time, resident chunks
and active job count. It does not invent CPU/GPU timings, VRAM usage or completion percentages.

## Known scaling limits

Brush sampling replays spatially indexed local candidates in their saved order. Snapshots
still clone and re-index edit metadata, so long histories can cause main-thread work.
Compact edit checkpoints and incremental index updates remain future work. The current
100,000-dab limit has not been stress-tested.

Retained cached height arrays occupy about 1.03 MiB (32 × 65 × 65 × 8 bytes). Mutex/map
metadata, single-flight slots, active worker references and simulation buffers are additional.
This is an implementation bound, not a measured process-memory claim. Dense foliage,
transparent water, procedural fragment detail and SSAO can be expensive on weak GPUs.

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
