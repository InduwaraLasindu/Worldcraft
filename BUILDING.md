# Building

Development for this task runs on hosted GitHub runners. Do not install Rust, clone the
repository, or build it on the owner's computer unless they later request local development.

## Hosted checks

The Cloud verification workflow uses standard ubuntu-latest/windows-latest public-repository
runners. It does not request paid/larger runners, enable spending or change billing.
It does not create Actions caches. Build outputs stay on hosted machines. After both platform builds and the native Linux check
pass, the Windows job prepares a private draft development release with a ZIP and SHA-256 checksum.
It does not publish a stable release or install anything on the owner's computer.
The branch preparation job commits Cargo.lock and rustfmt formatting only on the named
development branch. Other branches/PRs get read/test behavior without preparation writes.

The desktop jobs compile the native application and its tests on Linux and Windows.
Linux also opens the native window through Xvfb with Mesa software Vulkan, tests streamed
terrain/edit persistence, captures a real frame and exits. That image is committed to the
development branch as QA evidence. It is not a fabricated render or hardware benchmark.

## Optional source build on a development machine

Requires stable Rust and the Windows MSVC C++ build/link tools. No Python runtime is needed.

    cargo test -p worldcraft-core --locked
    cargo run -p worldcraft --locked
    cargo build --release -p worldcraft --locked

For Linux builds, install X11/XKB/udev development libraries and a Vulkan driver/loader.
See the workflow for its exact package list. File dialogs use the system desktop portal.

## Runtime files

World files are versioned JSON with the suggested suffix .worldcraft.json.
User-selected paths are honored. A manual overwrite creates a .json.bak previous-save copy.

Recovery files live beside an existing project with a .recovery.worldcraft.json suffix.
For a world not saved yet, they use:

- Windows: %LOCALAPPDATA%/Worldcraft/
- Linux: $XDG_DATA_HOME/worldcraft/ or ~/.local/share/worldcraft/

Use Open to restore one. Recent-world paths are stored in a small recent.json in that folder.
The app does not persist generated terrain caches in this milestone.

A polished installer, code signing, automatic updates and clean-machine installation
verification are future distribution work. A compiled Windows binary alone does not establish
that every driver, display configuration and permission state has been tested.
