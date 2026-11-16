# Adaptive renderer audit

Updated 2026-11-16. Candidate branch: `codex/worldcraft-adaptive-quality`.
Status: implementation and validation in progress; not a published release.

## Evidence so far

| Check | Result | Scope |
|---|---|---|
| Core + ML tests | Passed: 25 core unit + 28 world integration + 9 ML unit | Includes classifier/governor, parent grid/attributes and negative-coordinate boundaries |
| Desktop API integration | Passed, ci profile | Native Rust/Bevy compile; does not compile GPU shader variants at runtime |
| Desktop tests | Passed: 11 | Resource policy, legacy worlds, collisions, editing and storage |
| Native executable | Compiled, ci profile | Actual rendering still requires a usable graphics/display environment |
| Launch/Eco/general rendered checks | Pending hosted Linux/Windows verification | Do not claim actual image/driver validation until run completes |
| Gaming-GPU frame rate / VRAM | Unmeasured | No universal 30 FPS certification |

The current workspace has no active display. Creating a local X listener is denied
by its network policy, so actual rendering is delegated to the repository's standard
free hosted CI graphics route. This is a test-environment limit, not an application fix.

## What to examine in rendered results

- Nonblack scaled scene beneath a sharp, correctly ordered UI.
- Editing/picking in the same place at native, 85%, 67% and 50% render scale.
- Full-resolution probe actually renders before choosing a quality tier; Home stops 3D
  again once it finishes. Probe/report data must be measured, not invented FPS.
- Forward, depth and shadow variants of terrain_vertex.wesl compile.
- Parent-grid endpoint, negative-coordinate edge and normal/color matches.
- No near/far seam during observer movement; distant terrain still covers its extent.
- Eco/automatic transitions preserve old tiles until replacements arrive.
- Rapid world switching stays within job limits; Home/unfocused work remains throttled.
- Experimental ray lighting remains gated; check static-proxy/morphed-raster differences.

## Remaining limits

Reference-quality forests/materials, flowing rivers/lakes/waterfalls, advanced snowy
mountains, weather/clouds and ray-based nature sound are not implemented by this
performance iteration. See ENGINE_PLAN.md for their concrete staged implementation.
Continuous vegetation transitions and broad real-hardware testing also remain unfinished.
