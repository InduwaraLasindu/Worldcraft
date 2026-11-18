# WorldCraft alpha.7 material and landscape audit

Renderer source `8d27b3f1ffcd8d5da96b2a6abb7b74b9495cdc29` builds on alpha.6.

| Native 3840 × 2160 capture | World | Visible detail |
| --- | --- | --- |
| [Woodland boulder](media/relief/WorldCraft-alpha7-woodland-4k.jpg) | Temperate, seed 42 | Joint faces, mineral grains and parallax at walking distance |
| [Sunset coastline](media/relief/WorldCraft-alpha7-coast-4k.jpg) | Archipelago, seed 20261116 | Directional swell crossed by short wind ripples and a broken sun reflection |
| [Alpine snow belt](media/relief/WorldCraft-alpha7-alpine-4k.jpg) | Alpine, seed 324 | Snow held between dark rock ribs on the 1,174 m summit near X 3465, Z 203 |

All three are native game renders from untouched worlds. A hosted offscreen capture harness used the production world, material shaders and camera transforms at 3840 × 2160, with Native resolution, 4K textures and Ultra POM. The published stills use JPEG compression. The image ledger records the hashes of the original PNG frames and JPEG copies. [4K capture ledger](evidence/alpha7-4k.json)

## Surface behavior

Three fixed material seeds produce forest litter, stone and turf during packaging. Each 4K set stores sRGB color and linear perceptual roughness together in BC3 and linear height in BC4. The material derives close normals from filtered height differences and keeps the base normal field for distant shading. A four-meter stone tile has approximately 0.98 mm per texel.

| Active resolution | Combined color, roughness and height mip chains for all three materials | POM |
| --- | ---: | --- |
| 1024 | 6 MiB | Off, Balanced or Ultra |
| 2048 | 24 MiB | Off, Balanced or Ultra |
| 4096 | 96 MiB | Off, Balanced or Ultra |
| Adapters without BC formats | About 20 MiB at 1024 in decoded form | Same control |
| Eco, Low or optional scans | About 8 MiB for the base surface pair | Disabled |

These are texture payload sizes before driver overhead. The detailed modes also share about 4 MiB of base normals. Balanced caps the view march at 12 layers and Ultra at 24. Both refine the first crossing three times and stop marching completely beyond 42 m. Height and roughness data are linear throughout the mip chain.

The primary render uses the material in forward and deferred lighting. Secondary stone hits sample two shared 256² material maps using mesh UVs. Collision, secondary triangle intersections and cast-shadow silhouettes follow the boulder mesh. Terrain secondary hits use averaged vertex color. [Release controls and settings](ALPHA7_RELEASE.md)

## Visual iteration

The first alpine camera showed wooded foothills. The final route verifies snow weight at the selected summit and carries the generator's actual snow coverage through both mesh LOD endpoints. The material uses wind exposure, irregular rock joints, chips and slope to distribute the snow. A second coastal pass breaks long regular wave bands into shorter glints. Daylight-dependent fill and dusk exposure lift shaded surfaces.

The fixed-camera audit holds camera, sunlight, wind and reconstruction constant. It repeats the POM-off boulder exposure, switches Ultra on, moves laterally, tests the forest floor, and compares 4K with 1K. The comparison measures a fixed crop spanning 15–74% horizontally and 46–92% vertically. A pixel counts as changed when any 8-bit color channel moves by more than four levels.
