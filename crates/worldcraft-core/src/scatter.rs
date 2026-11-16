use crate::{generation::unit, Biome, ChunkCoord, Generator, CHUNK_SIZE};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Species {
    Broadleaf,
    Pine,
    Palm,
    Rock,
    Grass,
    Shrub,
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

pub fn scatter(generator:&Generator,coord:ChunkCoord,detail:u8)->Vec<Instance>{
    if generator.version()==1 {return legacy_scatter(generator,coord,detail);}
    let mut result=Vec::new();
    let seed=generator.settings().seed;
    let sea=generator.settings().sea_level;
    // World-aligned candidate cells, with a jitter inset that leaves neighboring
    // tree centers at least 6.4 m apart before scale variation.
    for z in 0..8 { for x in 0..8 {
        let gx=coord.x*8+x;let gz=coord.z*8+z;
        let wx=(gx as f64+0.2+unit(seed^201,gx,gz)*0.6)*16.0;
        let wz=(gz as f64+0.2+unit(seed^202,gx,gz)*0.6)*16.0;
        let sample=generator.sample(wx,wz);
        if sample.height<sea+1.5 {continue;}
        let slope=generator.slope(wx,wz);
        let forest=matches!(sample.biome,Biome::Forest|Biome::Rainforest|Biome::Conifer);
        let density=if forest{0.95}else if sample.biome==Biome::Grassland{0.16}else{0.06};
        let cluster=0.78+crate::geology::gradient(seed^204,wx/150.0,wz/150.0)*0.26;
        let chance=unit(seed^203,gx,gz);
        let tree=chance<(density*sample.vegetation*cluster).min(0.97)&&slope<0.85&&
            !matches!(sample.biome,Biome::Ocean|Biome::Beach|Biome::Desert|Biome::Snow|Biome::Volcanic);
        let rock=!tree&&unit(seed^205,gx,gz)<0.10+slope.min(1.0)*0.08&&slope<1.9;
        let species=if tree {match sample.biome {
            Biome::Conifer|Biome::Tundra=>Species::Pine,
            Biome::Rainforest=>Species::Palm,
            _=>Species::Broadleaf,
        }}else if rock{Species::Rock}else{continue};
        result.push(Instance{position:[wx,sample.height,wz],species,
            scale:if rock{0.6+unit(seed^206,gx,gz) as f32*1.9}else{0.72+unit(seed^206,gx,gz) as f32*0.70},
            yaw:(unit(seed^207,gx,gz)*std::f64::consts::TAU) as f32,
            variant:(crate::generation::hash(seed^208,gx,gz)%4) as usize});
    }}
    if detail==0 {
        // 4 m divides the chunk width exactly; candidates remain stable across
        // chunk boundaries and do not depend on which neighbor is loaded first.
        for z in 0..32 {for x in 0..32 {
            let gx=coord.x*32+x;let gz=coord.z*32+z;
            let wx=(gx as f64+0.15+unit(seed^210,gx,gz)*0.7)*4.0;
            let wz=(gz as f64+0.15+unit(seed^211,gx,gz)*0.7)*4.0;
            let sample=generator.sample(wx,wz);
            if sample.height>sea+3.0&&generator.slope(wx,wz)<0.8&&
                !matches!(sample.biome,Biome::Ocean|Biome::Beach|Biome::Desert|Biome::Snow|Biome::Volcanic)&&
                unit(seed^212,gx,gz)<(sample.vegetation*0.86).min(0.98) {
                result.push(Instance{position:[wx,sample.height,wz],
                    scale:0.55+unit(seed^213,gx,gz) as f32*0.70,
                    yaw:(unit(seed^214,gx,gz)*std::f64::consts::TAU) as f32,
                    species:if unit(seed^215,gx,gz)<0.08{Species::Shrub}else{Species::Grass},
                    variant:0});
            }
        }}
    }
    result
}
