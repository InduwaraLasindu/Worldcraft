use crate::{generation::unit, Biome, ChunkCoord, Generator, CHUNK_SIZE};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Species {
    Broadleaf,
    Pine,
    Palm,
    Rock,
    Grass,
    Shrub,
    Birch,
    Acacia,
    Fern,
    Flower,
    Reed,
}
impl Species {
    pub const TREES: [Self; 5] = [
        Self::Broadleaf,
        Self::Pine,
        Self::Palm,
        Self::Birch,
        Self::Acacia,
    ];
    pub fn is_tree(self) -> bool {
        matches!(
            self,
            Self::Broadleaf | Self::Pine | Self::Palm | Self::Birch | Self::Acacia
        )
    }
    pub fn is_ground(self) -> bool {
        matches!(
            self,
            Self::Grass | Self::Shrub | Self::Fern | Self::Flower | Self::Reed
        )
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Instance {
    pub position: [f64; 3],
    pub scale: f32,
    pub yaw: f32,
    pub species: Species,
    pub variant: usize,
}
/// One jittered candidate per world-aligned cell keeps placement independent of load order.
fn legacy_scatter(generator: &Generator, coord: ChunkCoord, detail: u8) -> Vec<Instance> {
    let mut result = Vec::new();
    let o = coord.origin();
    let spacing = 16.0;
    let n = (CHUNK_SIZE / spacing) as i64;
    let seed = generator.settings().seed;
    for z in 0..n {
        for x in 0..n {
            let gx = coord.x * n + x;
            let gz = coord.z * n + z;
            let wx = o[0] + (x as f64 + 0.2 + unit(seed ^ 201, gx, gz) * 0.6) * spacing;
            let wz = o[1] + (z as f64 + 0.2 + unit(seed ^ 202, gx, gz) * 0.6) * spacing;
            let s = generator.sample(wx, wz);
            if s.height < generator.settings().sea_level + 1.0 {
                continue;
            }
            let slope = generator.slope(wx, wz);
            let chance = unit(seed ^ 203, gx, gz);
            let forest = matches!(s.biome, Biome::Forest | Biome::Rainforest | Biome::Conifer);
            let density = if forest { 0.78 } else { 0.13 };
            let cluster =
                0.65 + crate::generation::noise(seed ^ 204, wx / 130.0, wz / 130.0) * 0.35;
            let tree = chance < density * s.vegetation * cluster
                && slope < 0.75
                && !matches!(
                    s.biome,
                    Biome::Snow | Biome::Ocean | Biome::Desert | Biome::Volcanic
                );
            let rock = unit(seed ^ 205, gx, gz) < 0.09 && slope < 1.8;
            let species = if tree {
                match s.biome {
                    Biome::Conifer | Biome::Tundra => Species::Pine,
                    Biome::Rainforest => Species::Palm,
                    _ => Species::Broadleaf,
                }
            } else if rock {
                Species::Rock
            } else {
                continue;
            };
            let scale = if species == Species::Rock {
                0.5 + chance as f32 * 2.0
            } else {
                0.65 + unit(seed ^ 206, gx, gz) as f32 * 0.75
            };
            result.push(Instance {
                position: [wx, s.height, wz],
                scale,
                yaw: (unit(seed ^ 207, gx, gz) * std::f64::consts::TAU) as f32,
                species,
                variant: (crate::generation::hash(seed ^ 208, gx, gz) % 4) as usize,
            });
        }
    }
    if detail == 0 {
        let spacing = 6.0;
        let n = (CHUNK_SIZE / spacing).ceil() as i64;
        for z in 0..n {
            for x in 0..n {
                let gx = coord.x * n + x;
                let gz = coord.z * n + z;
                let lx = (x as f64 + unit(seed ^ 210, gx, gz)) * spacing;
                let lz = (z as f64 + unit(seed ^ 211, gx, gz)) * spacing;
                if lx >= CHUNK_SIZE || lz >= CHUNK_SIZE {
                    continue;
                }
                let wx = o[0] + lx;
                let wz = o[1] + lz;
                let s = generator.sample(wx, wz);
                if s.height > generator.settings().sea_level + 2.0
                    && generator.slope(wx, wz) < 0.7
                    && !matches!(
                        s.biome,
                        Biome::Ocean | Biome::Beach | Biome::Desert | Biome::Snow | Biome::Volcanic
                    )
                    && unit(seed ^ 212, gx, gz) < s.vegetation * 0.8
                {
                    result.push(Instance {
                        position: [wx, s.height, wz],
                        scale: 0.5 + unit(seed ^ 213, gx, gz) as f32,
                        yaw: (unit(seed ^ 214, gx, gz) * std::f64::consts::TAU) as f32,
                        species: if unit(seed ^ 215, gx, gz) < 0.12 {
                            Species::Shrub
                        } else {
                            Species::Grass
                        },
                        variant: 0,
                    });
                }
            }
        }
    }
    result
}

pub fn scatter(generator: &Generator, coord: ChunkCoord, detail: u8) -> Vec<Instance> {
    if generator.version() == 1 {
        return legacy_scatter(generator, coord, detail);
    }
    if generator.version() >= 3 {
        return ecological_scatter(generator, coord, detail);
    }
    let mut result = Vec::new();
    let seed = generator.settings().seed;
    let sea = generator.settings().sea_level;
    // World-aligned candidate cells, with a jitter inset that leaves neighboring
    // tree centers at least 6.4 m apart before scale variation.
    for z in 0..8 {
        for x in 0..8 {
            let gx = coord.x * 8 + x;
            let gz = coord.z * 8 + z;
            let wx = (gx as f64 + 0.2 + unit(seed ^ 201, gx, gz) * 0.6) * 16.0;
            let wz = (gz as f64 + 0.2 + unit(seed ^ 202, gx, gz) * 0.6) * 16.0;
            let sample = generator.sample(wx, wz);
            if sample.height < sea + 1.5 {
                continue;
            }
            let slope = generator.slope(wx, wz);
            let forest = matches!(
                sample.biome,
                Biome::Forest | Biome::Rainforest | Biome::Conifer
            );
            let density = if forest {
                0.95
            } else if sample.biome == Biome::Grassland {
                0.16
            } else {
                0.06
            };
            let cluster =
                0.78 + crate::geology::gradient(seed ^ 204, wx / 150.0, wz / 150.0) * 0.26;
            let chance = unit(seed ^ 203, gx, gz);
            let tree = chance < (density * sample.vegetation * cluster).min(0.97)
                && slope < 0.85
                && !matches!(
                    sample.biome,
                    Biome::Ocean | Biome::Beach | Biome::Desert | Biome::Snow | Biome::Volcanic
                );
            let rock =
                !tree && unit(seed ^ 205, gx, gz) < 0.10 + slope.min(1.0) * 0.08 && slope < 1.9;
            let species = if tree {
                match sample.biome {
                    Biome::Conifer | Biome::Tundra => Species::Pine,
                    Biome::Rainforest => Species::Palm,
                    _ => Species::Broadleaf,
                }
            } else if rock {
                Species::Rock
            } else {
                continue;
            };
            result.push(Instance {
                position: [wx, sample.height, wz],
                species,
                scale: if rock {
                    0.6 + unit(seed ^ 206, gx, gz) as f32 * 1.9
                } else {
                    0.72 + unit(seed ^ 206, gx, gz) as f32 * 0.70
                },
                yaw: (unit(seed ^ 207, gx, gz) * std::f64::consts::TAU) as f32,
                variant: (crate::generation::hash(seed ^ 208, gx, gz) % 4) as usize,
            });
        }
    }
    if detail == 0 {
        // 4 m divides the chunk width exactly; candidates remain stable across
        // chunk boundaries and do not depend on which neighbor is loaded first.
        for z in 0..32 {
            for x in 0..32 {
                let gx = coord.x * 32 + x;
                let gz = coord.z * 32 + z;
                let wx = (gx as f64 + 0.15 + unit(seed ^ 210, gx, gz) * 0.7) * 4.0;
                let wz = (gz as f64 + 0.15 + unit(seed ^ 211, gx, gz) * 0.7) * 4.0;
                let sample = generator.sample(wx, wz);
                if sample.height > sea + 3.0
                    && generator.slope(wx, wz) < 0.8
                    && !matches!(
                        sample.biome,
                        Biome::Ocean | Biome::Beach | Biome::Desert | Biome::Snow | Biome::Volcanic
                    )
                    && unit(seed ^ 212, gx, gz) < (sample.vegetation * 0.86).min(0.98)
                {
                    result.push(Instance {
                        position: [wx, sample.height, wz],
                        scale: 0.55 + unit(seed ^ 213, gx, gz) as f32 * 0.70,
                        yaw: (unit(seed ^ 214, gx, gz) * std::f64::consts::TAU) as f32,
                        species: if unit(seed ^ 215, gx, gz) < 0.08 {
                            Species::Shrub
                        } else {
                            Species::Grass
                        },
                        variant: 0,
                    });
                }
            }
        }
    }
    result
}

/// Fixed 8 m canopy and 2 m understory candidate lattices. Every random decision
/// is a world-cell hash; this never searches loaded neighbors or grows a world
/// simulation. Maximum output is 256 canopy/rock + 4096 ground instances/tile.
fn ecological_scatter(generator: &Generator, coord: ChunkCoord, detail: u8) -> Vec<Instance> {
    let seed = generator.settings().seed;
    let sea = generator.settings().sea_level;
    let mut result = Vec::with_capacity(if detail == 0 { 2304 } else { 256 });
    for z in 0..16 {
        for x in 0..16 {
            let gx = coord.x * 16 + x;
            let gz = coord.z * 16 + z;
            let wx = (gx as f64 + 0.22 + unit(seed ^ 0xe201, gx, gz) * 0.56) * 8.0;
            let wz = (gz as f64 + 0.22 + unit(seed ^ 0xe202, gx, gz) * 0.56) * 8.0;
            let sample = generator.sample(wx, wz);
            if sample.height < sea + 1.0 {
                continue;
            }
            let slope = generator.slope(wx, wz);
            let blend = generator.biome_blend(wx, wz, &sample);
            let habitat = blend.habitat();
            let stand = (0.5 + crate::geology::gradient(seed ^ 0xe204, wx / 180.0, wz / 180.0))
                .clamp(0.0, 1.0);
            // Broad gaps and stand age correlate neighboring trees instead of
            // producing unrelated random scales or a uniformly filled plantation.
            let gap = 0.48 + stand * 0.52;
            let tree = slope < 0.90
                && unit(seed ^ 0xe203, gx, gz)
                    < (habitat.canopy as f64 * sample.vegetation * gap).min(0.97);
            let rock =
                !tree && slope < 2.2 && unit(seed ^ 0xe205, gx, gz) < 0.025 + slope.min(1.5) * 0.08;
            if !tree && !rock {
                continue;
            }
            let pick = unit(seed ^ 0xe208, gx, gz) as f32;
            let pine = blend.weight(Biome::Conifer) + blend.weight(Biome::Tundra);
            let birch = blend.weight(Biome::BirchForest);
            let acacia = blend.weight(Biome::Savanna);
            let tropical = ((sample.temperature - 0.70) / 0.15).clamp(0.0, 1.0) as f32;
            let palm = (blend.weight(Biome::Rainforest) * 0.28
                + blend.weight(Biome::Wetland) * 0.08)
                * tropical;
            let species = if rock {
                Species::Rock
            } else if pick < pine {
                Species::Pine
            } else if pick < pine + birch {
                Species::Birch
            } else if pick < pine + birch + acacia {
                Species::Acacia
            } else if pick < pine + birch + acacia + palm {
                Species::Palm
            } else {
                Species::Broadleaf
            };
            result.push(Instance {
                position: [wx, sample.height, wz],
                species,
                scale: if rock {
                    0.5 + unit(seed ^ 0xe206, gx, gz) as f32 * 2.1
                } else {
                    0.62 + stand as f32 * 0.72 + unit(seed ^ 0xe206, gx, gz) as f32 * 0.58
                },
                yaw: (unit(seed ^ 0xe207, gx, gz) * std::f64::consts::TAU) as f32,
                variant: (crate::generation::hash(seed ^ 0xe209, gx, gz) % 4) as usize,
            });
        }
    }
    if detail == 0 {
        for z in 0..64 {
            for x in 0..64 {
                let gx = coord.x * 64 + x;
                let gz = coord.z * 64 + z;
                let wx = (gx as f64 + 0.12 + unit(seed ^ 0xe210, gx, gz) * 0.76) * 2.0;
                let wz = (gz as f64 + 0.12 + unit(seed ^ 0xe211, gx, gz) * 0.76) * 2.0;
                let sample = generator.sample(wx, wz);
                if sample.height < sea + 0.8 {
                    continue;
                }
                let blend = generator.biome_blend(wx, wz, &sample);
                let habitat = blend.habitat();
                let patch = (0.65
                    + crate::geology::gradient(seed ^ 0xe216, wx / 26.0, wz / 26.0) * 0.50)
                    .clamp(0.2, 1.0);
                if unit(seed ^ 0xe212, gx, gz)
                    >= (habitat.ground as f64 * sample.vegetation * patch).min(0.97)
                {
                    continue;
                }
                // Most candidates fail density admission. Only survivors need
                // four exact terrain reads for their slope; placement is identical.
                if generator.slope(wx, wz) > 0.95 {
                    continue;
                }
                let pick = unit(seed ^ 0xe215, gx, gz) as f32;
                let species = if pick < habitat.reeds {
                    Species::Reed
                } else if pick < habitat.reeds + habitat.ferns {
                    Species::Fern
                } else if pick < habitat.reeds + habitat.ferns + habitat.flowers {
                    Species::Flower
                } else if pick > 0.975 {
                    Species::Shrub
                } else {
                    Species::Grass
                };
                let style = if blend.weight(Biome::Savanna) + blend.weight(Biome::Desert) > 0.45 {
                    1
                } else if blend.weight(Biome::Conifer) + blend.weight(Biome::Tundra) > 0.55 {
                    2
                } else if blend.weight(Biome::Rainforest) + blend.weight(Biome::Wetland) > 0.55 {
                    3
                } else {
                    0
                };
                result.push(Instance {
                    position: [wx, sample.height, wz],
                    species,
                    scale: 0.60 + unit(seed ^ 0xe213, gx, gz) as f32 * 0.65,
                    yaw: (unit(seed ^ 0xe214, gx, gz) * std::f64::consts::TAU) as f32,
                    variant: style
                        | (((crate::generation::hash(seed ^ 0xe217, gx, gz) % 4) as usize) << 2),
                });
            }
        }
    }
    result
}
