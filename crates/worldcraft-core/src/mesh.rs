use crate::{
    streaming::{ChunkCoord, CHUNK_SIZE},
    Generator,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ChunkKey {
    pub coord: ChunkCoord,
    pub lod: u8,
    pub edges: [u8; 4],
    pub revision: u64,
}
#[derive(Clone, Debug, Default)]
pub struct MeshData {
    pub positions: Vec<[f32; 3]>,
    pub morph: Vec<[f32; 4]>,
    pub morph_normals: Vec<[f32; 3]>,
    pub morph_colors: Vec<[f32; 4]>,
    pub normals: Vec<[f32; 3]>,
    pub colors: Vec<[f32; 4]>,
    pub uv: Vec<[f32; 2]>,
    /// Current/parent snow cover and local sand/ash weights in UNORM8.
    pub snow_cover: Vec<[u8; 4]>,
    pub indices: Vec<u32>,
}
impl MeshData {
    pub fn vertex(&mut self, p: [f32; 3], n: [f32; 3], c: [f32; 4], uv: [f32; 2]) -> u32 {
        let i = self.positions.len() as u32;
        self.positions.push(p);
        self.normals.push(n);
        self.colors.push(c);
        self.uv.push(uv);
        i
    }
}
pub fn build_mesh(generator: &Generator, key: ChunkKey) -> MeshData {
    terrain_mesh(generator, key, false)
}
pub(crate) fn terrain_mesh(generator: &Generator, key: ChunkKey, habitat: bool) -> MeshData {
    let lod = key.lod.min(3);
    let cells = 32usize >> lod;
    let step = CHUNK_SIZE / cells as f64;
    let origin = key.coord.origin();
    let mut m = MeshData::default();
    for z in 0..=cells {
        for x in 0..=cells {
            let (lx, lz) = (x as f64 * step, z as f64 * step);
            let (wx, wz) = (origin[0] + lx, origin[1] + lz);
            let mut h = generator.height(wx, wz);
            // Snap only the denser edge to its coarser neighbor's piecewise-linear boundary.
            for (on_edge, edge, along) in [
                (z == 0, 0, lx),
                (x == cells, 1, lz),
                (z == cells, 2, lx),
                (x == 0, 3, lz),
            ] {
                if on_edge && key.edges[edge] > lod {
                    let coarse_step = 4.0 * f64::from(1u32 << key.edges[edge].min(3));
                    let a = (along / coarse_step).floor() * coarse_step;
                    let b = (a + coarse_step).min(CHUNK_SIZE);
                    let t = if b > a { (along - a) / (b - a) } else { 0.0 };
                    let (ax, az, bx, bz) = if edge == 0 || edge == 2 {
                        (origin[0] + a, wz, origin[0] + b, wz)
                    } else {
                        (wx, origin[1] + a, wx, origin[1] + b)
                    };
                    h = generator.height(ax, az) * (1.0 - t) + generator.height(bx, bz) * t;
                }
            }
            let (color, cover, snow, substrate) = generator.surface_color(wx, wz, habitat);
            if habitat {
                let u = (snow * 255.0).round() as u8;
                let [sand, ash] = substrate.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8);
                m.snow_cover.push([u, u, sand, ash]);
            }
            m.vertex(
                [lx as f32, h as f32, lz as f32],
                generator.normal(wx, wz),
                color,
                if habitat {
                    [cover, cover]
                } else {
                    [lx as f32 / 8.0, lz as f32 / 8.0]
                },
            );
        }
    }
    let row = cells + 1;
    for z in 0..cells {
        for x in 0..cells {
            let a = (z * row + x) as u32;
            let b = a + 1;
            let c = a + row as u32;
            let d = c + 1;
            m.indices.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }
    // Skirts also conceal temporary mismatches while adjacent LOD jobs complete.
    let edges: Vec<Vec<usize>> = vec![
        (0..=cells).collect(),
        (0..=cells).map(|z| z * row + cells).collect(),
        (0..=cells).rev().map(|x| cells * row + x).collect(),
        (0..=cells).rev().map(|z| z * row).collect(),
    ];
    for edge in edges {
        for pair in edge.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let mut pa = m.positions[a];
            let mut pb = m.positions[b];
            pa[1] -= 48.0;
            pb[1] -= 48.0;
            if habitat {
                m.snow_cover.extend([m.snow_cover[a], m.snow_cover[b]]);
            }
            let ia = m.vertex(pa, m.normals[a], m.colors[a], m.uv[a]);
            let ib = m.vertex(pb, m.normals[b], m.colors[b], m.uv[b]);
            m.indices
                .extend_from_slice(&[a as u32, ia, b as u32, b as u32, ia, ib]);
        }
    }
    m
}
