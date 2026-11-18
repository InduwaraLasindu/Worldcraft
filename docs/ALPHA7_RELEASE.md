WorldCraft 0.2.0-alpha.7 is a native Windows x64 test build.

**Try the new materials**

Open **Render settings → Terrain materials**, select **Procedural**, choose **4096 (Ultra)** and turn **Parallax occlusion** to **Ultra**. Walk close to a boulder and move sideways. Switch POM to Off at the same spot to compare the cracks and ledges. Temperate seed **42** is the woodland used for the attached captures.

- Three original, tileable material sets contain native 4096 × 4096 albedo, perceptual roughness and linear height. Filtered height gradients supply the close surface normals. Litter, rock and turf each have 13 mip levels and their own physical relief scale.
- The GPU reads the selected 1K, 2K or 4K mip tail. The 4K color/roughness and height arrays occupy 96 MiB combined before driver overhead. Automatic chooses 1K or 2K. Low and Eco use the base textures. Adapters without BC support decode a bounded 1K set.
- POM uses a bounded layer search and three refinement steps. Its cost fades with distance and pixel footprint. Forward and deferred terrain share the same material shader.
- Native ray-query lighting now samples shared 256 × 256 albedo and roughness at secondary stone hits. The boulder mesh supplies the ray and shadow silhouette. Terrain secondary hits still use averaged vertex color. Hardware ray tracing can be selected when the adapter supports it.
- Joint planes and layered erosion shape the four shared boulder meshes. Slope and wind exposure break up existing alpine snow cover. Six filtered wave bands shape sun glints across coastal water.
- The DLSS, DLAA, FSR 4.1 and AA settings from alpha.6 carry forward. Vendor execution and frame times on retail GPUs still need physical hardware testing.

Download the per-user installer or extract the whole portable ZIP. The installer is unsigned. The screenshots and evidence archive come from the native app. The landscape captures use Archipelago seed **20261116** and Alpine seed **324**.
