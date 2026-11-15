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
- Background file I/O and map preview sampling.
- No persistent terrain cache or mandatory model download.

The UI reports measured smoothed FPS, the last completed generation time, resident chunks
and active job count. It does not invent CPU/GPU timings, VRAM usage or completion percentages.

## Known scaling limits

Brush history is an ordered list replayed during sampling. Very long histories need a
spatial index and checkpoints. Generation snapshots clone edit metadata; cache/invalidation
improvements should be measured before increasing edit limits.

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
