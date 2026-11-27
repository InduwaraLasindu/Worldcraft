# Worldcraft 0.2.0-alpha.8

This desktop alpha uses Godot Forward+ and the Rust Worldcraft generator.
Choose one of eleven presets or reuse a seed. The game starts at River Valley.
Existing world files, including baked learned regions and saved edits, can be
opened from the Load dialog.

On Windows 10 or 11, run **Worldcraft-Setup-0.2.0-alpha.8.exe** and launch
Worldcraft from the Start menu. The installer works per user. For the portable
build, extract the entire Windows ZIP and open **Worldcraft.exe** in the
extracted folder. Use a current graphics driver with Vulkan support for
Forward+ rendering.

- The seeded terrain, rock, bark, foliage, water and sky materials are evaluated
  in Godot shaders. A shared seamless noise texture generates on a background
  thread at the selected resolution from 512 to 4096 pixels.
- Godot's shadow maps, leaf transmission and local volumetric fog create
  sunlight shafts. The Global illumination switch enables SDFGI. Ultra also
  adds screen-space indirect lighting and reflections.
- High and Ultra use Forward+. Eco reduces the streamed radius, cloud samples,
  and lighting cost. FSR 2 and the optional adaptive resolution control use
  the same Godot path on compatible NVIDIA and AMD GPUs.
- WASD explores, Tab toggles flight, and F2 hides the UI. Use Edit to paint
  terrain and habitat changes or undo and redo a stroke. Press F to recapture
  the mouse while editing.

This release is a rendering migration preview. Hardware ray-traced lighting,
DLSS integration, terrain export and in-app learned model generation remain
outside the Godot client. Software Vulkan captures measure visual integration.
Gaming frame-rate certification requires physical graphics hardware.

## Rendering references

- [Godot material and subsurface documentation](https://docs.godotengine.org/en/stable/tutorials/3d/standard_material_3d.html)
- [Godot volumetric fog and fog volumes](https://docs.godotengine.org/en/stable/tutorials/3d/volumetric_fog.html)
- [Godot freely moving global illumination](https://docs.godotengine.org/en/stable/tutorials/3d/global_illumination/using_sdfgi.html)
- [Godot MultiMesh batching and linear instance buffers](https://docs.godotengine.org/en/stable/tutorials/performance/using_multimesh.html)
