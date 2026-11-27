//! Seeded small scenery seated on the same triangles the desktop client draws.
use crate::{
    generation::{hash, unit},
    Biome, ChunkCoord, Generator, MeshData, Preset,
};

/// Codes 0–10 are reserved for the established tree and understory scatter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Cover {
    Grass = 4,
    Fern = 8,
    Flower = 9,
    Reed = 10,
    Scree = 11,
    Branch = 12,
    DryGrass = 13,
    Deadfall = 14,
    Moss = 15,
    Succulent = 16,
    Driftwood = 17,
}

impl Cover {
    fn upright(self) -> bool {
        matches!(
            self,
            Self::Grass | Self::Fern | Self::Flower | Self::Reed | Self::DryGrass | Self::Succulent
        )
    }

    fn size(self, value: f32) -> f32 {
        match self {
            Self::Scree => 0.38 + value * value * 1.3,
            Self::Branch => 0.45 + value * 0.95,
            Self::Deadfall | Self::Driftwood => 0.67 + value * 0.98,
            Self::Moss => 0.56 + value * 0.85,
            Self::Succulent => 0.5 + value * 0.95,
            _ => 0.62 + value * 0.78,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Prop {
    pub local: [f32; 3],
    pub up: [f32; 3],
    pub kind: Cover,
    pub scale: f32,
    pub yaw: f32,
    pub variant: u8,
}

const PATCHES: i64 = 16;
const SUBCELLS: i64 = 4;
const CELLS: i64 = PATCHES * SUBCELLS;
const PATCH_SIZE: f64 = 8.0;
const SPACING: f64 = PATCH_SIZE / SUBCELLS as f64;

pub fn scatter(
    generator: &Generator,
    coord: ChunkCoord,
    lod: u8,
    detail: u8,
    mesh: &MeshData,
) -> Vec<Prop> {
    let settings = generator.settings();
    let mut out = Vec::new();
    if settings.preset == Preset::Empty {
        return out;
    }
    let [ox, oz] = coord.origin();
    let seed = settings.seed ^ 0x5343_454e_4552_59;
    let grid = 32 >> lod.min(3);
    let river = settings.preset == Preset::RiverValley && generator.version() >= 4;
    let edited = !generator.edits().is_empty();
    for pz in 0..PATCHES {
        for px in 0..PATCHES {
            let wx = ox + (px as f64 + 0.5) * PATCH_SIZE;
            let wz = oz + (pz as f64 + 0.5) * PATCH_SIZE;
            let sample = generator.sample(wx, wz);
            let biome = generator.biome_blend(wx, wz, &sample);
            let wood = f64::from(
                biome.weight(Biome::Forest)
                    + biome.weight(Biome::Conifer)
                    + biome.weight(Biome::BirchForest)
                    + biome.weight(Biome::Rainforest),
            );
            let dry = f64::from(biome.weight(Biome::Desert) + biome.weight(Biome::Savanna) * 0.72);
            let bare = f64::from(
                biome.weight(Biome::Snow)
                    + biome.weight(Biome::Volcanic)
                    + biome.weight(Biome::Tundra) * 0.8,
            );
            let shore = f64::from(biome.weight(Biome::Beach) + biome.weight(Biome::Wetland) * 0.38);
            let patch = crate::geology::gradient(seed, wx / 36.0, wz / 36.0);
            let density = (0.55 + 0.52 * patch - 0.16 * dry - 0.08 * bare).clamp(0.24, 0.88);
            // The sparse outer ring and economy mode are subsets of the same cells.
            let slot = (hash(seed ^ 8, coord.x * PATCHES + px, coord.z * PATCHES + pz) & 15) as i64;
            let stride = match detail {
                0 => 1,
                2 => 4,
                _ => 16,
            };
            for i in 0..SUBCELLS * SUBCELLS {
                if (i + slot).rem_euclid(stride) != 0 {
                    continue;
                }
                let ix = px * SUBCELLS + i % SUBCELLS;
                let iz = pz * SUBCELLS + i / SUBCELLS;
                let gx = coord.x * CELLS + ix;
                let gz = coord.z * CELLS + iz;
                let random = |salt| unit(seed ^ salt, gx, gz);
                if random(1) >= density {
                    continue;
                }
                let x = ((ix as f64 + 0.1 + random(2) * 0.8) * SPACING) as f32;
                let z = ((iz as f64 + 0.1 + random(3) * 0.8) * SPACING) as f32;
                let (y, up) = seat(mesh, grid, x, z);
                let (world_x, world_z) = (ox + f64::from(x), oz + f64::from(z));
                let sea_height = f64::from(y) - settings.sea_level;
                if sea_height <= 0.24
                    || up[1] < 0.57
                    || (river
                        && crate::river::vegetation_cover(settings, world_x, world_z, f64::from(y))
                            < 0.53)
                {
                    continue;
                }
                let pick = random(4);
                let kind = if sea_height < 8.5 && shore > 0.26 {
                    if pick < 0.009 {
                        Cover::Driftwood
                    } else if pick <= shore * 0.22 && sea_height > 1.2 {
                        Cover::Reed
                    } else {
                        Cover::Scree
                    }
                } else if up[1] < 0.79 || random(5) < bare * 0.92 {
                    if pick < 0.13 && up[1] > 0.85 && sample.moisture > 0.38 {
                        Cover::Moss
                    } else {
                        Cover::Scree
                    }
                } else if random(6) < dry {
                    if pick < 0.62 {
                        Cover::Scree
                    } else if pick < 0.94 || sample.temperature < 0.57 {
                        Cover::DryGrass
                    } else {
                        Cover::Succulent
                    }
                } else if random(7) < wood {
                    if pick < 0.006 && up[1] > 0.95 {
                        Cover::Deadfall
                    } else if pick < 0.23 {
                        Cover::Branch
                    } else if pick < 0.48 {
                        Cover::Moss
                    } else if pick < 0.61 && sample.moisture > 0.48 {
                        Cover::Fern
                    } else {
                        Cover::Grass
                    }
                } else if pick < 0.14 {
                    Cover::Scree
                } else if pick < 0.24 && sample.temperature < 0.36 {
                    Cover::DryGrass
                } else if pick < 0.34 && sample.temperature > 0.26 {
                    Cover::Flower
                } else {
                    Cover::Grass
                };
                if kind.upright() {
                    let cover = if edited {
                        generator.sample(world_x, world_z).vegetation
                    } else if river {
                        settings.vegetation
                    } else {
                        sample.vegetation
                    };
                    if up[1] < 0.76 || random(9) > (cover * 1.15).min(1.0) {
                        continue;
                    }
                }
                out.push(Prop {
                    local: [x, y, z],
                    up: if kind.upright() { [0.0, 1.0, 0.0] } else { up },
                    kind,
                    scale: kind.size(random(10) as f32),
                    yaw: (random(11) * std::f64::consts::TAU) as f32,
                    variant: (hash(seed ^ 12, gx, gz) & 1) as u8,
                });
            }
        }
    }
    out
}

fn seat(mesh: &MeshData, cells: usize, x: f32, z: f32) -> (f32, [f32; 3]) {
    let step = 128.0 / cells as f32;
    let (x, z) = (x / step, z / step);
    let (ix, iz) = (x as usize, z as usize);
    let (u, v) = (x - ix as f32, z - iz as f32);
    let row = cells + 1;
    let a = iz * row + ix;
    let [a, b, c, d] = [a, a + 1, a + row, a + row + 1].map(|i| mesh.positions[i][1]);
    let (height, dx, dz) = if u + v <= 1.0 {
        (a + (b - a) * u + (c - a) * v, b - a, c - a)
    } else {
        (d + (c - d) * (1.0 - u) + (b - d) * (1.0 - v), d - c, d - b)
    };
    let length = step.hypot(dx.hypot(dz));
    (height, [-dx / length, step / length, -dz / length])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{mesh::build_habitat_mesh, ChunkKey, GeneratorSettings};

    #[test]
    fn valley_scenery_is_seeded_bounded_and_seated() {
        let generator = Generator::new(
            GeneratorSettings::for_preset(Preset::RiverValley, 42),
            vec![],
        );
        let [x, _, z] = generator.spawn_position();
        let center = ChunkCoord {
            x: (x / 128.0).floor() as i64,
            z: (z / 128.0).floor() as i64,
        };
        let mut total = 0;
        for dz in -1..=1 {
            for dx in -1..=1 {
                let coord = ChunkCoord {
                    x: center.x + dx,
                    z: center.z + dz,
                };
                let key = ChunkKey {
                    coord,
                    lod: 0,
                    edges: [0; 4],
                    revision: 0,
                };
                let mesh = build_habitat_mesh(&generator, key);
                let dense = scatter(&generator, coord, 0, 0, &mesh);
                assert!(dense.len() <= (CELLS * CELLS) as usize);
                total += dense.len();
                if coord == center {
                    assert_eq!(dense, scatter(&generator, coord, 0, 0, &mesh));
                    let sparse = scatter(&generator, coord, 0, 1, &mesh);
                    assert!(sparse.len() <= (PATCHES * PATCHES) as usize);
                    assert!(!sparse.is_empty() && sparse.iter().all(|prop| dense.contains(prop)));
                    let [ox, oz] = coord.origin();
                    for prop in &dense {
                        let [x, y, z] = prop.local;
                        assert_eq!(y, seat(&mesh, 32, x, z).0);
                        assert!(
                            crate::river::vegetation_cover(
                                generator.settings(),
                                ox + f64::from(x),
                                oz + f64::from(z),
                                f64::from(y),
                            ) >= 0.53
                        );
                    }
                }
            }
        }
        assert!(
            total > 10_000,
            "{total} added props around River Valley spawn"
        );
    }
}
