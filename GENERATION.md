# Generation

Generation version: 1. Units: metres. Seed: u64, or stable FNV-1a conversion of a text seed.

## Classical backend

1. Coordinate hashing produces repeatable scalar lattice values.
2. Quintic interpolation and several fBm octaves create smooth fields.
3. Domain warping offsets coordinates before geography sampling.
4. Regional continent/foothill fields and a ridge mask create elevation.
5. Analytic valley masks depress narrow channels.
6. Temperature varies by regional field and altitude; moisture varies regionally.
7. Biome rules classify the fields; slope blends rock into surface colors.
8. Explicit brush operations replay in saved order.
9. Coordinate-jittered candidates are accepted by biome, height, slope and density.
10. Meshes are generated only for resident chunks.

There is no global mutable random stream. Loading regions in a different order does not
change their terrain or scatter. Negative coordinates use floor division. Sampling across
chunk boundaries uses the same global function.

## Honest limitations

Valleys are continuous noise-derived masks, not downhill-flowing drainage networks.
Ocean planes occupy sea level. There are no lake levels, river water surfaces,
tributary joins, waterfalls, erosion simulation, rain shadows or real ecosystem simulation.
Temperature/rainfall are normalized artistic controls, not calibrated Earth climate.

Biome labels are discrete and materials use colored geometry with slope blending.
Imported PBR textures, triplanar microdetail and ecological transition bands are future work.
Terrain is a heightfield; there are no cave openings or overhangs yet.

Smooth brushes blend toward a stored local mean height; they do not run a diffusion
solver. Terrace brushes quantize height with radial falloff. Biome painting and vegetation
painting are independent of underlying elevation.

## Learned terrain evaluation

Reviewed [Terrain Diffusion](https://github.com/xandergos/terrain-diffusion),
[InfiniteDiffusion](https://arxiv.org/abs/2512.08309), the
[project page](https://xandergos.github.io/terrain-diffusion/) and the linked
[Minecraft integration](https://github.com/xandergos/terrain-diffusion-mc).

The useful principles are seed-indexed random access, hierarchical context/detail,
bounded caches and generating only requested regions. These motivate the classical
backend's stateless coordinates and versioned inputs; this does **not** make it a
diffusion generator.

The reference runtime uses Python/PyTorch and model-specific preprocessing/cascades.
Worldcraft must not add a Python runtime. A future proof of concept should compare native
ONNX Runtime bindings against Candle/Burn after verifying export coverage, tensor operators,
conditioning and sampling parity. First validate fixed patches, then neighboring/negative
patches, arbitrary request order, cache eviction and cross-device numerical tolerances.

No model has been ported, exported, downloaded, redistributed or tested here. Source,
model weights and training data licenses must be checked individually. The reference's
source MIT license alone does not establish rights to every weight file.
