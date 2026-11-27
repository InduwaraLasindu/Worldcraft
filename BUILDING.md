# Building and runtime files

Development uses hosted standard GitHub runners. Do not clone, install development
dependencies or compile on the owner's computer without a new instruction.

## Godot desktop client

Build the extension and stage the platform library beside the Godot project.
From a Linux hosted checkout with Godot 4.7.2 installed:

    cargo build --release --locked -p worldcraft-godot
    mkdir -p game/bin
    cp target/release/libworldcraft_godot.so game/bin/
    godot --headless --path game --editor --import
    godot --path game

On Windows, stage `target/release/worldcraft_godot.dll` in `game/bin`.
The Godot editor opens `game/project.godot`. The exported release uses the
native Rust library beside the executable. The Godot pack ships without
the older material atlas, external art, or the Bevy renderer.

Run the bridge checks with
`godot --headless --path game --script res://tests/integration.gd`.
Install matching export templates and use the `Linux` or `Windows` release
preset to build a native package. The Godot desktop prerelease workflow
builds these targets and checks an exported Linux frame before publishing.

## Previous Bevy client

## Hosted verification and distribution
Cloud verification checks the core/ML libraries on Linux and Windows, builds the
native desktop app, executes model inference, and runs native graphics routes.
Linux uses software Vulkan. The Windows route requests software Direct3D 12 at a 128 × 96 QA viewport with shorter frame-settling intervals.
Actual workflow results determine support; neither is a gaming benchmark.

The Windows job uses one optimized executable for validation and packaging. A bounded coordinator can retry script/installer fixes on the same hosted workspace only when product source and dependencies are unchanged.

The Windows job also builds the optimized release executable, packages ONNX
Runtime 1.32.0 from a checksum-pinned official archive, and collects licenses.
The package also includes checksum-pinned Microsoft DXC 1.9.2609 and its notices,
so normal launches can use the modern Direct3D shader compiler. Installer shortcuts
set the application directory as the working directory. For command-line launches
from another directory, WGPU_DX12_COMPILER=dxc explicitly selects the bundled compiler.
A per-user Inno Setup installer requires no administrator rights and downloads no
models. Hosted package tests validate the portable/installed native runtime and
uninstall while preserving an untracked user file. The candidate is published as a
prerelease only after the configured checks pass. No installation is run
on the owner's computer.

The bounded hosted iteration workspace reuses compiler output only on its
temporary runner. Its control file can stop the session; it is not product
infrastructure, a paid service or a promise of background development.

## Runtime files
Projects use versioned .worldcraft.json files. An overwrite keeps a .json.bak copy.
Recovery snapshots live next to a saved project, or in the app's data directory:

- Windows: %LOCALAPPDATA%/Worldcraft/
- Linux: $XDG_DATA_HOME/worldcraft/ or ~/.local/share/worldcraft/

Open a recovery file through Open. Recent paths and optional model files also
live in the app data directory. The optional fast model occupies 22.5 MB; the complete detailed model set occupies
2.28 GB. Model files are never installed by the installer.
The installer leaves user worlds/models outside its application folder intact.

## Source builds on a separate development machine
Requires current stable Rust and the Windows MSVC build tools, or Linux
X11/XKB/udev/Vulkan libraries and ALSA development headers (`libasound2-dev`
on Debian and Ubuntu). No Python/Java runtime is needed by the app.

    cargo test -p worldcraft-core -p worldcraft-ml --locked
    cargo run -p worldcraft --locked
    cargo build --release -p worldcraft --locked

Classical worlds and the baked AI example need no native inference library.
New learned regions need the packaged lib/onnxruntime.dll (Windows), or
lib/libonnxruntime.so (Linux). WORLDCRAFT_ORT_DYLIB is an explicit development/CI
override. The detailed option downloads its large models only when selected in the app.
Infinite learned streaming, GPU inference, signing and automatic updates remain
unfinished.

## World material preparation

Run the offline baker on the build machine before compiling the desktop app.

    python -m pip install Pillow==11.3.0 numpy==2.3.5
    python tools/prepare_materials.py

Use `python tools/prepare_materials.py --procedural-only` when the optional
Poly Haven CDN is unavailable. The bundled photographic-style materials
and cutout foliage are checked in and work with either build.

The baker verifies nine CC0 source images and writes the base arrays, procedural
relief maps and 21 world material files under `crates/worldcraft/embedded`.
The `.wct` files contain independently compressed mip levels through 4096 px.
The source checkout can load them directly. For a standalone binary, place
these files in `world-materials/` beside the executable. The Windows installer
and portable ZIP both include this directory. Installed games read the maps
offline. Core and ML builds can skip the material baker.

## NVIDIA / AMD enabled builds

The render-quality workflow prepares pinned official inputs with
`python tools/ci/bootstrap_upscalers.py` and builds with
`cargo build --release -p worldcraft --features nvidia-dlss --locked`.
It exports DLSS_SDK, VULKAN_SDK and WORLDCRAFT_UPSCALER_ROOT through GITHUB_ENV.
Outside Actions, use the absolute environment paths printed by the helper.
Bindgen requires libclang; the independent AMD ABI check uses a C++ compiler.
`WORLDCRAFT_CHECK_FSR_ABI=1` enables that check against upstream headers.
The default source build remains available without NVIDIA build inputs.

Before launching an enabled executable, run
`python tools/ci/bootstrap_upscalers.py --root <SDK-root> --copy-to target/release`.
The helper verifies hashes before copying only the release redistributables.
The Windows packager copies these runtimes, the pinned manifest and full notices.
No SDK, compiler, Python runtime or model download is required by the installed app.

DLSS/DLAA require a supported NVIDIA Vulkan device/driver. AMD's signed FSR 4.1
runtime integration currently targets Windows x64 Vulkan. It queries providers
from the actual device, and reports an analytical compatibility provider when
selected by the SDK. Software routes establish fallback/native integration,
not vendor hardware certification. The release job requires both platform jobs,
exact-source native reports, package hashes and runtime/license verification;
only an explicitly marked alpha-5 commit can publish the prerelease.

The alpha.5 final Windows QA uses a bounded 400 × 300 WARP viewport and a
30-minute all-case deadline based on measured software settling time. Installer/
portable artifacts are retained before the long route; scene and saved-world
checks remain required. Actual success/source scope is recorded in docs/RENDER_AUDIT.md.
