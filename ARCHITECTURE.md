# Architecture decision record: native exploration foundation

## Decision

The current playable client lives under `game/`. Godot Forward+ owns
presentation, procedural shaders, particles and native lighting. The
`worldcraft-godot` Rust extension owns the bounded terrain worker, mesh
packets, saved-world identity, climate queries, and edit transactions. It
shares `worldcraft-core` with the previous Bevy client below.

See [Godot alpha.8](docs/GODOT_RELEASE.md) for feature and hardware bounds.

## Previous Bevy client

Use Rust 2021, a Cargo workspace, Bevy 0.20.1 (a pinned integration baseline),
wgpu through Bevy's renderer, and bevy_egui 0.43.1. The application uses winit
for its native window and Bevy ECS for scene/resource ownership.

This is a maintainability choice. Starting an independent Vulkan renderer, render graph,
asset loader, shader system and editor simultaneously would delay testing the actual world
experience. Bevy is an open-source Rust engine, not a commercial runtime. Its renderer can
use Vulkan on supported systems and Direct3D 12 on Windows. Experimental ray-query lighting is capability-gated; hardware performance is not claimed. A future specialized renderer must preserve the core world API.

## Boundaries

| Component | Responsibility |
| --- | --- |
| worldcraft-core | Settings, generation, saved learned regions, edits, terrain/vista meshes, scatter, world files and exports |
| worldcraft-ml | Pinned streamed model assets and native CPU coarse or sequential three-model bakes |
| worldcraft/session | Active world, history, camera world coordinates, generation snapshots |
| worldcraft/streaming | Prioritized tasks, LOD keys, scene replacement, residency |
| worldcraft/assets | Original procedural vegetation/rock geometry, locally anchored ground batches |
| worldcraft/foliage | Crown topology boundaries, outer fading and radius updates |
| worldcraft/scene | Camera, sunlight, atmosphere, volumetrics and render quality |
| worldcraft/vista | One bounded far-mesh job and horizon ocean |
| worldcraft/raytrace | Capability gates, bounded ray proxies and mode switching |
| worldcraft/surface | Embedded terrain/water PBR extension and stable world-space detail |
| worldcraft/control | Walking/flying, picking, brush interaction |
| worldcraft/storage | Background world creation, file dialogs and asynchronous save/load/recovery |
| worldcraft/ui | Beginner workflow, optional inspector, map previews |
| worldcraft/verify | Native hosted integration and image capture |

The core has no graphics/window dependencies and can be tested without a GPU.
Background workers receive immutable Arc generation snapshots. Only the main thread owns
ECS commands and GPU mesh registration. Outdated task results are discarded.

## World representation

Continuous heightfields cover the surface. A chunk is 128 m square, with 32/16/8/4
cells per side as internal topology budgets. The adaptive renderer uses a continuous
horizontal observer-distance morph toward globally aligned parent vertices (including
normal/color targets) before coarsening. The same rebased observer drives forward and
depth/shadow passes. The first distant collar follows that morph and fades toward the
analytic horizon. Skirts conceal temporary asynchronous mismatches. The renderer keeps
old tiles until budget replacements are ready; late refinement/teleports and vegetation
transitions still need movement-route audits. Legacy mesh/export functions retain their
original edge interpolation, independently of runtime render quality.

Persistent coordinates use f64 with an explicit +/-1 billion metre validity bound.
GPU geometry uses local f32 coordinates; the render origin moves in 256 m increments.
This is a large finite numeric domain, not mathematical infinity.

A world stores rules and explicit edits, not every generated region. The generator version
and settings fingerprint are validated on load. New worlds use generator 4.
Editing and re-saving retain the saved terrain and scatter recipe.
Version 2 shares a bounded, settings-specific eroded-height cache across immutable edit
snapshots. Explicit edits use a spatial index while retaining operation order.
No cross-device bitwise GPU reproducibility is promised.

## Physics scope

The first controller collides with the same triangulated LOD0 terrain used near the player.
Walking uses bounded substeps, gravity, ground detection, jumping and step limits.
Trees/rocks use conservative horizontal exclusion cylinders. This is not a general rigid-body
physics engine; capsules, slope sliding and full object/cave collision remain future work.

## Future boundaries

Volumetric caves should occupy sparse local SDF regions, with explicit transitions into the
heightfield. Learned models should supply regional elevation/climate fields through a native
inference backend. Graphs should produce the same versioned generation inputs. Structures,
paths and imported glTF assets should extend placement rules without altering the seed identity.

No dynamic plugin ABI, full diffusion cascade, SDF mesher or node editor is implemented.
The learned backend bakes a coarse regional hybrid or finite detailed three-model region
into the world file. Optional detail lives in a separate saved height field; format 3
stores it while older project formats retain their generator and geography. Coarse,
base and decoder sessions are dropped between stages. No inference runs in terrain
streaming, brush replay or saved-world reopening.

## Reference assessment

[Horizon](https://github.com/horizon-rust/horizon) and its
[native client notes](https://github.com/horizon-rust/horizon/blob/main/docs/native-client.md)
were reviewed for the Rust-native exploration experience, threaded streaming, fixed-step
player behavior and honest platform validation. Worldcraft does not copy Minecraft's blocks,
rules, world format, proprietary resources, or Horizon's vanilla-parity machinery.

The Bevy versioned examples for [atmosphere](https://github.com/bevyengine/bevy/blob/v0.20.1/examples/3d/atmosphere.rs),
[SSAO](https://github.com/bevyengine/bevy/blob/v0.20.1/examples/3d/ssao.rs) and
[async tasks](https://github.com/bevyengine/bevy/blob/v0.20.1/examples/async_tasks/async_compute.rs)
informed the integration. Dependencies retain their own licenses.

## Adaptive renderer continuation

The short launch probe and pure frame-time controller live in calibration.rs and
core/adaptive.rs. Machine-local graphics/radius/resolution never alter generator identity.
A scaled SceneTarget renders the whole 3D pipeline; egui composites it under the native
resolution UI. Brush picking converts window logical coordinates to that camera viewport.
See docs/AGENT_HANDOFF.md for entry points and docs/ENGINE_PLAN.md for the larger cinematic
world program. The next systems must preserve these ownership and compatibility boundaries.
