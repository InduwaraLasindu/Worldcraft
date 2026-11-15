# Architecture decision record: native exploration foundation

## Decision

Use Rust 2021, a Cargo workspace, Bevy 0.16.1 (a pinned integration baseline),
wgpu through Bevy's renderer, and bevy_egui 0.34.1. The application uses winit
for its native window and Bevy ECS for scene/resource ownership.

This is a maintainability choice. Starting an independent Vulkan renderer, render graph,
asset loader, shader system and editor simultaneously would delay testing the actual world
experience. Bevy is an open-source Rust engine, not a commercial runtime. Its renderer can
use Vulkan on supported systems and Direct3D 12 on Windows. Hardware ray tracing is
not enabled or claimed. A future specialized renderer must preserve the core world API.

## Boundaries

| Component | Responsibility |
| --- | --- |
| worldcraft-core | Settings, deterministic generation, biomes, edits, terrain meshes, scatter, world files |
| worldcraft/session | Active world, history, camera world coordinates, generation snapshots |
| worldcraft/streaming | Prioritized tasks, LOD keys, scene replacement, residency |
| worldcraft/assets | Original procedural vegetation and rock geometry |
| worldcraft/scene | Camera, sky, sunlight, atmosphere and render quality |
| worldcraft/control | Walking/flying, picking, brush interaction |
| worldcraft/storage | File dialogs and asynchronous save/load/recovery |
| worldcraft/ui | Beginner workflow, optional inspector, map previews |
| worldcraft/verify | Native hosted integration and image capture |

The core has no graphics/window dependencies and can be tested without a GPU.
Background workers receive immutable Arc generation snapshots. Only the main thread owns
ECS commands and GPU mesh registration. Outdated task results are discarded.

## World representation

Continuous heightfields cover the surface. A chunk is 128 m square, with 32/16/8/4
cells per side. Fine boundaries interpolate the neighboring coarse boundary. Skirts
hide temporary transitions while asynchronous replacements complete.

Persistent coordinates use f64 with an explicit +/-1 billion metre validity bound.
GPU geometry uses local f32 coordinates; the render origin moves in 256 m increments.
This is a large finite numeric domain, not mathematical infinity.

A world stores rules and explicit edits, not every generated region. The generator version
and settings fingerprint are validated on load. Changing the generator requires an explicit
version/migration decision. No cross-device bitwise GPU reproducibility is promised.

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

No dynamic plugin ABI, learned inference, model downloader, SDF mesher or node editor is
implemented in this milestone.

## Reference assessment

[Horizon](https://github.com/horizon-rust/horizon) and its
[native client notes](https://github.com/horizon-rust/horizon/blob/main/docs/native-client.md)
were reviewed for the Rust-native exploration experience, threaded streaming, fixed-step
player behavior and honest platform validation. Worldcraft does not copy Minecraft's blocks,
rules, world format, proprietary resources, or Horizon's vanilla-parity machinery.

The Bevy versioned examples for [atmosphere](https://github.com/bevyengine/bevy/blob/v0.16.1/examples/3d/atmosphere.rs),
[SSAO](https://github.com/bevyengine/bevy/blob/v0.16.1/examples/3d/ssao.rs) and
[async tasks](https://github.com/bevyengine/bevy/blob/v0.16.1/examples/async_tasks/async_compute.rs)
informed the integration. Dependencies retain their own licenses.
