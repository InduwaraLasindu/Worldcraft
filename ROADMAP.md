# Roadmap

## Current alpha
Native exploration, classical/thermal terrain, bounded near/far LOD, procedural
nature, PBR atmosphere and volumetrics, experimental ray queries, seven editing
brushes, durable worlds, native Terrain Diffusion coarse and finite detailed three-model bakes and terrain
exports. See MASTER_SPEC_COVERAGE.md for precise limits.

## Next natural-world work
1. Extend the verified Terrain3D map bridge to a sliding region ring, with
   rebased region locations and collision parity during world travel.
2. Establish shared watershed ownership across padded regions. Apply hydraulic
   erosion, stream channels and connected water surfaces from that shared graph.
3. Feed geological and water masks to surface materials and procedural scatter.
4. Add sparse cave and overhang meshes with compatible surface holes and collision.
5. Add an optional native Terrain Diffusion macro backend, building on the
   existing finite learned-region integration.

## Production work
Measured Windows gaming-GPU routes, frame-time distributions and memory budgets;
ray denoising/upscalers and supported-backend validation; long edit histories and
incremental checkpoints; key bindings/accessibility; cinematic tooling; signed
installers, clean-machine coverage and safe update delivery.

No unimplemented item should appear as a working product control. An alpha build
must not be described as the completed master specification or a stable release.

## Expanded cinematic program

The active performance/continuous-terrain milestone and eleven subsequent content,
rendering, simulation, sound, authoring and release milestones are specified in
[docs/ENGINE_PLAN.md](docs/ENGINE_PLAN.md), including measurable acceptance gates.
Continue through [docs/AGENT_HANDOFF.md](docs/AGENT_HANDOFF.md); do not infer completion
from a roadmap entry. Geometry-derived audio is a separate unfinished system.
