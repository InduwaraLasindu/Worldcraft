# Alpha 2 detailed-terrain audit

This milestone adds a finite native three-model Terrain Diffusion bake. It is an
engineering alpha, with the remaining scope tracked in [the feature matrix](../MASTER_SPEC_COVERAGE.md).

## Verified Linux product

Product source: `c04703b417cb01e5fb0994ed8fd739dbd2c80efa`.
[Successful hosted run](https://github.com/InduwaraLasindu/Worldcraft/actions/runs/43847671644).
All development, model downloads and execution took place on a standard hosted runner.

| Check | Observed result |
| --- | --- |
| Core, ML and desktop regression tests | 45 + 9 + 9 = 63 passed |
| Complete native detailed generations | Three: initial, repeated seed and changed seed |
| Same-seed maximum height difference | 0 m in this CPU fixture |
| Changed-seed mean absolute height difference | 95.569 m in this fixture |
| Saved detailed field | 320 × 320, 30 m spacing, 9,570 m width |
| Detailed elevation range | 24.336–351.813 m in this fixture |
| Coarse/detail join sampled across 2 mm | Maximum difference 0.000210 m; continuous transition |
| Cancellation during actual base refinement | Returned Cancelled without producing a detailed result |
| Persistence | Exact world round trip and identical reopened samples |
| Editing | Sculpt, undo and reopened edited heights passed |
| Native scene | 800 × 600 software Vulkan; 49 resident chunks and 36,754 far-terrain triangles |
| Existing graphics regression | Low/Balanced/High, atmosphere, volumetrics, world switching, origin shift, maps, actual ray-query dispatch and raster cleanup passed |
| Detailed terrain GLB | Official Khronos validator: 0 errors, 0 warnings |
| Detailed terrain PNG | 16 bits; maximum decoded height error 0.0003733 m, within half a quantization step |
| Optional weights | Immutable revision, sizes and SHA-256 verified; 2,276,345,629 bytes total |
| Source/runtime | Native Rust and ONNX Runtime CPU; no Python/Java runtime shipped |

The suite loads the three models sequentially. Linux process peak resident high-water
measurement was 4,648,244 kB (about 4.43 GiB) during these tests. This is one fixture, not a universal
memory requirement or a performance guarantee. Model calls run away from the UI thread;
saved-world exploration runs no inference.

The final screenshot was visually inspected for visible terrain, procedural trees,
cast shadows, haze, holes and a blank viewport. The image below is actual application
output from software Vulkan. Its on-screen FPS is not a gaming-GPU benchmark.

![Native detailed saved world in the hosted software-rendered application](media/alpha2-detailed-linux.png)

Image SHA-256: `62953cc47fcc5af1b3087592e160a6ea174853bb2b433a55f353f96474d720d6`.

## Windows distribution

[Published alpha 2](https://github.com/InduwaraLasindu/Worldcraft/releases/tag/v0.2.0-alpha.2-build.2).
Frozen product source: `2ace1eb0f86df10a8267a9a21487d25ca79557d7`.

The [Windows build and distribution run](https://github.com/InduwaraLasindu/Worldcraft/actions/runs/43852794308)
passed 63 core/ML/desktop tests, three complete detailed generations, live base-stage
cancellation, repeated-seed equality, changed-seed relief, exact save/reopen and edits.
Its Direct3D 12 software route passed eight 128 × 96 API/state captures. Portable and
installed runtime checks, per-user installation and uninstall preserving an untracked
file passed. A separate actual portable executable produced the reviewed 640 × 360
detailed-world scene below, with its native sculpt/undo/save/reopen assertions enabled.

![Actual packaged Windows detailed example on software Direct3D 12](media/alpha2-detailed-windows.png)

Windows image SHA-256: `141248e331cfd4a642c53cf626434da8a096cc6b6bd4e54e3eac68638156bb48`.

The build run's final publication lookup returned HTTP 404 after the verified draft
and all three files had uploaded. The [successful publication recovery](https://github.com/InduwaraLasindu/Worldcraft/actions/runs/43861895570)
used the release ID, verified downloaded asset bytes, the checksum file, x64 executable,
embedded example and frozen source, then published those same files after image review.
No product rebuild or binary substitution occurred during recovery.

The [native HTTP audit](https://github.com/InduwaraLasindu/Worldcraft/actions/runs/43859616781)
cancelled real fast/base transfers, verified retry and SHA-256, removed partial files,
and kept the fast model usable after cancelling the detailed set. The audit used the
same product code and lockfile as the released binary.

| Asset | Bytes | SHA-256 |
| --- | ---: | --- |
| Windows installer | 30,846,183 | `bbb7da7e8883dcaa32063fbe821093daa4d156b2446b50caa5c5875cc1139450` |
| Portable ZIP | 50,571,869 | `2ea4e06a12f082457d32d27d7954c2e94f2a3cddad1a59342ce76ecd14c0453f` |

## Practical limits

The new field is a central detailed region blended into coarse learned terrain and
classical surroundings. Infinite learned streaming, neighboring base-window ensembles,
WorldClim conditioning parity and GPU inference are unfinished. Hydrology, caves,
graphs, asset import, wind/weather, advanced ecosystems and cinematic tools remain
on the roadmap.

Software-rendered tests do not establish broad Windows UI, gaming-GPU performance,
driver coverage or production stability. The installer remains unsigned.
