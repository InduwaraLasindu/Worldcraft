# Alpha 5 renderer audit

Product candidate: `a58a3d45988fcb9bc1dea22a4a7cdbd3035aeebe`, tree
`30df4273e41bcdb307995725abf49b0831ae08c6`.

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

## Rejected first candidate

Source `3c5fc4a946f759617fecc2cb68c62c07f9c24c52` passed compilation/unit tests
but failed its first real Linux launch before creating RenderApp. The new Automatic
API mapping used `backends: None`, which Bevy interprets as rendering disabled.
The correction retains WgpuSettings defaults (including WGPU_BACKEND), and a new
regression checks Automatic and both explicit API choices. No release was published
from the rejected source. Native checks must run again on the correction.

## Remaining limits

Software Vulkan/DX12 cannot validate RTX DLSS/DLAA execution or FSR ML on supported
AMD hardware. Unsupported vendor cases are explicit skips. No gaming FPS/VRAM or
temporal travel image-quality certification is claimed. The 80% realism target
remains unmet; distant forests, outer fades, connected water/hydrology and more
natural botanical scale/coverage still need work. This is an alpha test release.

## Explicit API compatibility patch

Bevy 0.20.1's raw-Vulkan instance path also attempts Vulkan when the requested
backend mask excludes it. The retained upstream bevy_render crate has one small
guard that uses ordinary wgpu initialization for explicit non-Vulkan selection.
Vulkan callbacks and all shaders/device features remain unchanged. Provenance,
crate hash and original licenses are in vendor/bevy_render/WORLDCRAFT_PATCH.md.
Both native platform gates must run on this corrected source.

## Current native checkpoints

- 113 Rust tests and nine Python regressions passed locally before the cloud reset.
- c717 Linux run 44247596562 passed all original routes, nine renderer cases and
  27 nature/biome/renderer captures. Original images/hash records are retained at
  evidence commit 272238c2f8993df9367feeac023035c32c25c21d. Two selected views were
  inspected: TAA woodland and alpine. Checkerboard outer fades and sparse ground
  coverage remain visible. Do not infer an 80% score or relabel them as a58 pixels.
- Final source a58 is being checked in run 44250142452. Its first Windows route
  saved FXAA, SMAA, TAA, cinematic, lens and scaled captures without shader errors,
  with sampled resident memory about 975 MiB, then hit the 12-minute QA deadline.
  This is recorded as a failure; no release was published.
- PR #7 / run 44254855083 retries the same immutable product source with a bounded
  400 × 300 WARP viewport shown without activation, all nine cases and strict
  pixel/ownership/error/memory checks, under a 20-minute deadline. This changes
  the hosted QA harness, not normal application defaults or product source.
- Linux fallback 44251947171 uses the same source and bounded package downloads.
  The publication coordinator checks completed exact-source platform jobs and
  inner package/runtime/license verification before creating the prerelease.

Final release/assets and current native results remain pending at this checkpoint.
