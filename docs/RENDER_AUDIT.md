# Alpha 5 renderer audit

Candidate branch: `codex/worldcraft-render-quality`, based on ecology source
`e21d5bcf11b0ec8d3621c8bd97ea5aafd934fe60` and its documentation follow-up.
The renderer changes preserve generator math, world-file formats and authored edits.

## Implemented

Real Bevy/NGX DLSS SR/DLAA; the official signed AMD SDK 2.4.0 Vulkan API with
active-provider reporting; mutually exclusive native FXAA/SMAA/TAA; CAS; output
scale; shadow/AO/bloom, color/tone mapping, FOV/DOF, sampling, V-sync and API settings.
Machine-local preference defaults preserve old files. Unsupported methods fall
back to native AA. API changes require restart. No ray reconstruction or frame generation.

One 80-byte shared frame carries current/previous wind and observer state for
forward/depth/shadow/motion passes. Effect changes use an applied key rather than
rebuilding every frame. Status formatting follows option/SDK revisions. SDK
contexts are reused across frames and recreated only for output/preset changes.
Worker/chunk/cache budgets and software submission fencing are retained.

AMD descriptors retain their original C++ headers. An independently compiled
upstream-header ABI/enum check accompanies the Rust layout checks. Dynamic DLL
loads use absolute executable paths and pinned size/SHA-256 checks. The packager
includes release-only binaries, pinned provenance and full original notices.

## Verification status before hosted execution

The cloud checkout passes desktop/unit and core/ML tests with the actual NVIDIA
feature enabled, including camera history, native AA switching/prepass ownership,
old saves, deterministic generation, bounded streaming and persistent GPU buffers.
Nine Python regressions cover log rejection, download integrity/fallback, paths
and redistribution allowlists. These local results are not native pixel evidence.

The hosted `Render quality and alpha 5 release` workflow is the release gate.
It builds the same source on Linux/Windows with actual SDK headers/libraries,
runs native shaders and nine AA/lens cases, keeps the original nature/biome,
calibration, idle, ray and saved-world routes, and checks installer/portable/uninstall.
Reports embed `GITHUB_SHA`; release publication verifies reports and inner package
runtime hashes/licenses before creating the version tag. Failed jobs cannot publish.

New pixel evidence and the final source/run are recorded after this candidate
actually passes. Do not inherit the older e21 image approval for the new executable.

## Remaining limits

Software Vulkan/DX12 cannot validate RTX DLSS/DLAA execution or FSR ML on supported
AMD hardware. Unsupported vendor cases are explicit skips. No gaming FPS/VRAM or
temporal travel image-quality certification is claimed. The 80% realism target
remains unmet; distant forests, outer fades, connected water/hydrology and more
natural botanical scale/coverage still need work. This is an alpha test release.
