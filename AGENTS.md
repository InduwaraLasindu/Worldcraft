# Worldcraft working rules

- Development and testing must happen in hosted/cloud environments. The owner explicitly
  asked to keep project checkouts, development dependencies and build output off their
  slow, storage-constrained computer. Do not use the owner's desktop for development
  unless they explicitly change this instruction.
- Work in InduwaraLasindu/Worldcraft. Preserve unrelated work; use feature branches and
  pull requests. Do not force-push.
- Exploration comes first. World creation must lead directly to an explorable landscape.
  Editing and advanced controls are optional.
- Keep the application native and Rust-based. Do not replace it with a website or add
  a Python runtime. Keep generation independent from rendering.
- Keep streaming bounded, reject stale jobs and preserve explicit edits across unload/save.
- Only advertise implemented features. Do not call this milestone photorealistic,
  ray traced, learned, physically eroded or a finished release.
- Use standard free public-repository CI runners. Do not enable paid services,
  larger runners, purchases or spending changes.
- Run meaningful core/integration checks. Verify actual rendering when possible and
  distinguish software Vulkan checks from Windows GPU/user-interaction testing.
- Read ARCHITECTURE.md, GENERATION.md and ROADMAP.md before changing system boundaries.
