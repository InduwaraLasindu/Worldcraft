# Building and runtime files

Development uses hosted standard GitHub runners. Do not clone, install development
dependencies or compile on the owner's computer without a new instruction.

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
uninstall while preserving an untracked user file. The candidate is attached to
a draft prerelease only after the configured checks pass. No installation is run
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
X11/XKB/udev/Vulkan libraries. No Python/Java runtime is needed by the app.

    cargo test -p worldcraft-core -p worldcraft-ml --locked
    cargo run -p worldcraft --locked
    cargo build --release -p worldcraft --locked

Classical worlds and the baked AI example need no native inference library.
New learned regions need the packaged lib/onnxruntime.dll (Windows), or
lib/libonnxruntime.so (Linux). WORLDCRAFT_ORT_DYLIB is an explicit development/CI
override. The detailed option downloads its large models only when selected in the app.
Infinite learned streaming, GPU inference, signing and automatic updates remain
unfinished.
