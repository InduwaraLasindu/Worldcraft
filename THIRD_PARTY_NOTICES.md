# Third-party notices

Worldcraft's procedural geometry and world-generation code are original to this repository.
No Minecraft/Mojang resources, commercial engine assets, video frames or diffusion model
weights are bundled.

Primary dependencies:
- Bevy — MIT OR Apache-2.0.
- bevy_egui — MIT; egui — MIT OR Apache-2.0.
- serde / serde_json — MIT OR Apache-2.0.
- rfd — MIT.

These are a source-level overview, not a complete binary redistribution notice bundle.
Each transitive dependency retains its own license. A distribution build must include
applicable copyright/license files for the exact Cargo.lock resolution before it is
published as an installable release.

Horizon and Terrain Diffusion were research references, not copied runtime dependencies.
Their code, models, data and third-party resources have separate licensing considerations.

The development packager includes Cargo's exact resolved dependency metadata and copies
LICENSE/LICENCE/COPYING/NOTICE/COPYRIGHT files supplied in each resolved crate. Distribution
review must still check completeness and special third-party attribution before a public installer.
