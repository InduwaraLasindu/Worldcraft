A Windows x64 test build for exploring procedurally generated landscapes in Bevy.

- Forest litter, fractured stone and turf now use original procedural color, normal and roughness maps by default. Switch to the previous Poly Haven scans live under **Render settings → Terrain materials**.
- Grass follows the terrain triangles underfoot. Meadow, wetland, conifer and savanna plants have distinct height and spread. Ground patches use 28 grass triangles each, down from 40.
- Tree crowns and branches taper geometrically at their outer draw distance. Distant forest cover follows climate, clearings, vegetation brushes and the terrain LOD transition.
- Cloud layers now vary in thickness across the weather field and form smaller, rounded masses. The existing cloud sample and memory limits still apply.
- The native AA, NVIDIA DLSS/DLAA and AMD FSR 4.1 integration and settings from alpha.5 are included. The active method depends on the device and driver.

Download the installer or extract the entire portable ZIP. The installer is unsigned and installs per user. On an RTX card, choose **Render settings → Display → Vulkan**, restart, then select **DLSS Quality** or **DLAA**. For a like-for-like first look, try the Temperate preset with seed **42**, Savanna **761**, or Alpine **324**. The six PNG assets show the actual default procedural materials.

The evidence archive includes six untouched worlds, a fixed-camera material comparison, eight frames from a moving woodland route, and the Linux render audits. Software Vulkan and Windows DX12/WARP checks cover rendering and packaging. Hardware vendor dispatch, gaming frame rates and a reference-matched realism score still need physical GPU validation or independent visual review. Distant forest beyond the tree cutoff is currently surface shading. Connected rivers and waterfalls are still in development planning.
