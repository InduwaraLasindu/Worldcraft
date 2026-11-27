# Worldcraft 0.2.0-alpha.10

This desktop alpha uses Godot Forward+ and the Rust Worldcraft generator.
Choose one of eleven presets or reuse a seed. The game starts at River Valley.
Existing world files, including baked learned regions and saved edits, can be
opened from the Load dialog.

On Windows 10 or 11, run **Worldcraft-Setup-0.2.0-alpha.10.exe** and launch
Worldcraft from the Start menu. The installer works per user. For the portable
build, extract the entire Windows ZIP and open **Worldcraft.exe** in the
extracted folder. Use a current graphics driver with Vulkan support for
Forward+ rendering.

## Procedural scenery

- A seeded ground-cover layer places flowers, moss, scree, brush, deadfall,
  succulents and driftwood alongside the existing trees and undergrowth. Habitat,
  coast height, slope and river waterlines determine what grows at each spot.
- River Valley seed `42` streams more than 10,000 scenery objects in nine tiles.
  The status bar shows the number of loaded objects. Counting uses placement
  records, so a tree's trunk and crown count as one tree.
- Rust seats the new objects on the drawn terrain triangles and prepares the
  instance buffers off the main thread. Godot uploads one buffer per prototype
  and tile. Eco and distant tiles select a sparse subset of the same positions.
- The release includes a screenshot gallery rendered from the exported game.

## Landscape rendering

- River Valley has animated foam and a turbulent plunge pool. The waterfall's
  spray and fog follow its seeded location and receive the scene's lighting.
- Terrain uses the world's sand, volcanic ash, snow and canopy weights.
  Exposed rock shows bedding and fractures, and forest soil has layered litter
  and gravel. Distant slopes use canopy shading beyond the full tree meshes.
- Conifers have more varied branch silhouettes. Trees have exposed roots at
  close range, and ferns, shrubs, grass and reeds have distinct silhouettes.
- Cloud volume is integrated at half resolution and composited with the sky.
  The high wispy layer and lower banks share the seeded weather front.

## Settings and controls

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

This release is an environment rendering preview. Hardware ray-traced lighting,
DLSS integration, terrain export and in-app learned model generation remain
outside the Godot client. Software Vulkan captures measure visual integration.
Gaming frame-rate certification requires physical graphics hardware.

## Rendering references

- [Godot material and subsurface documentation](https://docs.godotengine.org/en/stable/tutorials/3d/standard_material_3d.html)
- [Godot volumetric fog and fog volumes](https://docs.godotengine.org/en/stable/tutorials/3d/volumetric_fog.html)
- [Godot freely moving global illumination](https://docs.godotengine.org/en/stable/tutorials/3d/global_illumination/using_sdfgi.html)
- [Godot MultiMesh batching and linear instance buffers](https://docs.godotengine.org/en/stable/tutorials/performance/using_multimesh.html)
- [Godot sky shader subpasses](https://docs.godotengine.org/en/stable/tutorials/shaders/shader_reference/sky_shader.html)
