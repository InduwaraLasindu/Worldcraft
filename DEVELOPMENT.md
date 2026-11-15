# Development

Keep exploration the primary workflow. Editing and advanced tools must remain optional.
Preserve unrelated repository work. Use feature branches and reviewable commits.
Never claim a planned feature, inferred FPS figure or untested graphics path is verified.

## Checks

    cargo fmt --all -- --check
    cargo test -p worldcraft-core --locked
    cargo clippy -p worldcraft-core --all-targets --locked
    cargo test --profile ci -p worldcraft --locked
    cargo build --profile ci -p worldcraft --locked

The renderer-independent tests cover determinism, negative coordinates, seams, LOD boundaries,
mesh validity, normals, climate/presets, scatter, transaction undo/redo, validation, checksums,
atomic replacement failure behavior and backup/load roundtrips. Desktop tests cover the
triangulated collision sampler and ray picking. The smoke check adds actual window/renderer
execution, background terrain tasks, integrated edits and persistence.

Add meaningful integration regressions when behavior changes. Do not add tests that merely
repeat implementation constants or fabricate performance guarantees.

## Before a real early release

On representative Windows hardware, verify create/open/save/reopen, walking and jumping,
flight, input focus, brush drag/undo/redo, persistent biome/vegetation edits, rendering presets,
resize, fullscreen, repeated world replacement and long-distance travel.
Test Low on weaker hardware and Balanced/High on at least one Vulkan/DX12 adapter.
Capture real visual evidence, record exact commit/settings/hardware and measure frame times.

Use [ROADMAP.md](ROADMAP.md) to track missing scope. Read [ARCHITECTURE.md](ARCHITECTURE.md)
before changing terrain storage, rendering or generation versioning.
