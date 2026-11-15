# Worldcraft

A native Rust application for creating and exploring continuous procedural landscapes.
Exploration is the main experience; world editing is optional.

Development has started from an empty repository. The renderer-independent world core
is being built and checked on hosted Linux and Windows runners. This branch is an
engineering milestone, not a finished release.

The core includes seeded geography, climate-based biomes, stitched terrain LOD,
deterministic environmental placement, durable brush transactions, undo/redo, and a
versioned save format. The desktop application follows in the next commit.

No Python, Java, paid APIs, model downloads, or commercial engine runtime is required.
