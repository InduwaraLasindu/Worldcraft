//! Bounded geometric clipmap rings for distant geography.
//! Near edges use the exact streamed field. Only the distant samples use the
//! analytic preview, with a 128 m transition band before the coarser rings.
use crate::{streaming::lod_for, ChunkCoord, Generator, MeshData, CHUNK_SIZE};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub x0: f64,
    pub z0: f64,
    pub x1: f64,
    pub z1: f64,
}
#[derive(Clone, Copy, Debug)]
pub struct Ring {
    pub inner: Bounds,
    pub outer: Bounds,
    pub step: f64,
}
pub fn rings(center: ChunkCoord, radius: i64, distance: f64) -> Vec<Ring> {
    let radius = radius.clamp(1, 12);
    let mut inner = Bounds {
        x0: (center.x - radius) as f64 * CHUNK_SIZE,
        z0: (center.z - radius) as f64 * CHUNK_SIZE,
        x1: (center.x + radius + 1) as f64 * CHUNK_SIZE,
        z1: (center.z + radius + 1) as f64 * CHUNK_SIZE,
    };
    let edge = ChunkCoord {
        x: center.x + radius + 1,
        z: center.z,
    };
    let mut step = 4.0 * f64::from(1u32 << lod_for(edge, center));
    let mut out = Vec::new();
    let distance = distance.clamp(2048.0, 65536.0);
    for level in 0..12 {
        let margin = if level == 0 { 128.0 } else { step * 16.0 };
        let align = step * 2.0;
        let outer = Bounds {
            x0: ((inner.x0 - margin) / align).floor() * align,
            z0: ((inner.z0 - margin) / align).floor() * align,
            x1: ((inner.x1 + margin) / align).ceil() * align,
            z1: ((inner.z1 + margin) / align).ceil() * align,
        };
        out.push(Ring { inner, outer, step });
        if (outer.x1 - outer.x0).min(outer.z1 - outer.z0) >= distance * 2.0 {
            break;
        }
        inner = outer;
        step *= 2.0;
    }
    out
}
#[derive(Clone, Copy)]
struct Vertex {
    h: f64,
    n: [f32; 3],
    c: [f32; 4],
    cover: f32,
    snow: f32,
}
fn approx(g: &Generator, x: f64, z: f64, habitat: bool) -> Vertex {
    let s = g.preview_sample(x, z);
    let dx = (g.preview_sample(x + 2.0, z).height - g.preview_sample(x - 2.0, z).height) * 0.25;
    let dz = (g.preview_sample(x, z + 2.0).height - g.preview_sample(x, z - 2.0).height) * 0.25;
    let len = (dx * dx + dz * dz + 1.0).sqrt();
    let r = ((dx.hypot(dz) - 0.5) / 0.7).clamp(0.0, 1.0) as f32;
    let (base, cover, snow) = if g.version() >= 3 {
        let blend = g.biome_blend(x, z, &s);
        (
            blend.color(),
            if habitat {
                g.canopy_cover(x, z, &s, &blend, dx.hypot(dz))
            } else {
                0.0
            },
            blend.weight(crate::Biome::Snow),
        )
    } else {
        (
            s.biome.color(),
            0.0,
            u8::from(s.biome == crate::Biome::Snow) as f32,
        )
    };
    let base = g.shoreline_pigment(&s, base, x, z);
    let rock = [0.29, 0.27, 0.23];
    let mut c = [1.0; 4];
    for i in 0..3 {
        c[i] = base[i] * (1.0 - r) + rock[i] * r;
    }
    Vertex {
        h: s.height,
        cover,
        snow,
        n: [(-dx / len) as f32, (1.0 / len) as f32, (-dz / len) as f32],
        c,
    }
}
fn lerp(a: Vertex, b: Vertex, t: f64) -> Vertex {
    let mut n = [0.0; 3];
    let mut c = [0.0; 4];
    for i in 0..3 {
        n[i] = a.n[i] * (1.0 - t as f32) + b.n[i] * t as f32;
    }
    for i in 0..4 {
        c[i] = a.c[i] * (1.0 - t as f32) + b.c[i] * t as f32;
    }
    Vertex {
        h: a.h * (1.0 - t) + b.h * t,
        cover: a.cover + (b.cover - a.cover) * t as f32,
        snow: a.snow + (b.snow - a.snow) * t as f32,
        n,
        c,
    }
}
fn field(g: &Generator, x: f64, z: f64, near: Option<Bounds>, habitat: bool) -> Vertex {
    let mut v = approx(g, x, z, habitat);
    if let Some(b) = near {
        let distance = (b.x0 - x)
            .max(x - b.x1)
            .max(b.z0 - z)
            .max(z - b.z1)
            .max(0.0);
        let t = (distance / 128.0).clamp(0.0, 1.0);
        if t < 1.0 {
            let (c, cover, snow, _) = g.surface_color(x, z, habitat);
            v = lerp(
                Vertex {
                    h: g.height(x, z),
                    n: g.normal(x, z),
                    c,
                    cover,
                    snow,
                },
                v,
                t,
            );
        }
    }
    v
}
pub fn build_vista(g: &Generator, center: ChunkCoord, radius: i64, distance: f64) -> MeshData {
    build(g, center, radius, distance, false)
}
pub fn build_continuous_vista(
    g: &Generator,
    center: ChunkCoord,
    radius: i64,
    distance: f64,
) -> MeshData {
    build(g, center, radius, distance, true)
}
fn build(
    g: &Generator,
    center: ChunkCoord,
    radius: i64,
    distance: f64,
    continuous: bool,
) -> MeshData {
    let mut mesh = MeshData::default();
    let origin = center.origin();
    let mut plan = rings(center, radius, distance);
    if continuous {
        let minimum = (radius.clamp(2, 12) - 1) as f64 * CHUNK_SIZE;
        let lod = (0..3)
            .find(|&l| minimum < crate::continuous::end_distance(l))
            .unwrap_or(3);
        let mut inner = plan[0].inner;
        let mut step = 4.0 * f64::from(1u32 << lod);
        plan.clear();
        for level in 0..12 {
            let margin = if level == 0 { 128.0 } else { step * 16.0 };
            let align = step * 2.0;
            let outer = Bounds {
                x0: ((inner.x0 - margin) / align).floor() * align,
                z0: ((inner.z0 - margin) / align).floor() * align,
                x1: ((inner.x1 + margin) / align).ceil() * align,
                z1: ((inner.z1 + margin) / align).ceil() * align,
            };
            plan.push(Ring { inner, outer, step });
            if (outer.x1 - outer.x0).min(outer.z1 - outer.z0)
                >= distance.clamp(2048.0, 65536.0) * 2.0
            {
                break;
            }
            inner = outer;
            step *= 2.0;
        }
    }
    for (level, ring) in plan.iter().enumerate() {
        let b = ring.outer;
        let s = ring.step;
        let nx = ((b.x1 - b.x0) / s).round() as usize;
        let nz = ((b.z1 - b.z0) / s).round() as usize;
        let mut grid = vec![None; (nx + 1) * (nz + 1)];
        for z in 0..nz {
            for x in 0..nx {
                let wx = b.x0 + x as f64 * s;
                let wz = b.z0 + z as f64 * s;
                if wx >= ring.inner.x0
                    && wz >= ring.inner.z0
                    && wx + s <= ring.inner.x1
                    && wz + s <= ring.inner.z1
                {
                    continue;
                }
                let mut ids = [0; 4];
                for (corner, (dx, dz)) in [(0, 0), (1, 0), (0, 1), (1, 1)].into_iter().enumerate() {
                    let ix = x + dx;
                    let iz = z + dz;
                    let slot = iz * (nx + 1) + ix;
                    ids[corner] = *grid[slot].get_or_insert_with(|| {
                        let px = b.x0 + ix as f64 * s;
                        let pz = b.z0 + iz as f64 * s;
                        let near = (level == 0).then_some(ring.inner);
                        let mut v = field(g, px, pz, near, continuous);
                        // The denser outer edge follows the next ring's polyline.
                        if iz == 0 || iz == nz || ix == 0 || ix == nx {
                            let horizontal = iz == 0 || iz == nz;
                            let along = if horizontal { px } else { pz };
                            let coarse = s * 2.0;
                            let a = (along / coarse).floor() * coarse;
                            let t = (along - a) / coarse;
                            if t > 0.0 {
                                let sample = |at| {
                                    if horizontal {
                                        field(g, at, pz, near, continuous)
                                    } else {
                                        field(g, px, at, near, continuous)
                                    }
                                };
                                v = lerp(sample(a), sample(a + coarse), t);
                            }
                        }
                        let position =
                            [(px - origin[0]) as f32, v.h as f32, (pz - origin[1]) as f32];
                        let mut parent_cover = v.cover;
                        if continuous {
                            let mut parent_snow = v.snow;
                            let mut target = position;
                            let mut normal = v.n;
                            let mut color = v.c;
                            let mut end = 0.0;
                            color[3] = 0.0;
                            if level == 0 {
                                let parent = s * 2.0;
                                let tx = (px / parent).floor() * parent;
                                let tz = (pz / parent).floor() * parent;
                                let pv = field(g, tx, tz, near, continuous);
                                target = [
                                    (tx - origin[0]) as f32,
                                    pv.h as f32,
                                    (tz - origin[1]) as f32,
                                ];
                                parent_cover = pv.cover;
                                parent_snow = pv.snow;
                                normal = pv.n;
                                color = pv.c;
                                let b = ring.inner;
                                let outside = (b.x0 - px)
                                    .max(px - b.x1)
                                    .max(b.z0 - pz)
                                    .max(pz - b.z1)
                                    .max(0.0);
                                color[3] = (1.0 - outside / 128.0).clamp(0.0, 1.0) as f32;
                                let lod = (s / 4.0).log2().round() as u8;
                                if lod < 3 {
                                    end = crate::continuous::end_distance(lod) as f32;
                                }
                            }
                            mesh.snow_cover.push([
                                (v.snow * 255.0).round() as u8,
                                (parent_snow * 255.0).round() as u8,
                                0,
                                0,
                            ]);
                            mesh.morph.push([target[0], target[1], target[2], end]);
                            mesh.morph_normals.push(normal);
                            mesh.morph_colors.push(color);
                        }
                        mesh.vertex(
                            position,
                            v.n,
                            v.c,
                            if continuous {
                                [v.cover, parent_cover]
                            } else {
                                [(px - origin[0]) as f32 / 8.0, (pz - origin[1]) as f32 / 8.0]
                            },
                        )
                    });
                }
                let [a, b, c, d] = ids;
                mesh.indices.extend_from_slice(&[a, c, b, b, c, d]);
            }
        }
    }
    mesh
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_mesh, streaming::edge_lods, ChunkKey, GeneratorSettings};
    #[test]
    fn clipmap_rings_cover_the_annulus_without_overlaps_or_gaps() {
        for center in [ChunkCoord { x: 0, z: 0 }, ChunkCoord { x: -7, z: 5 }] {
            for radius in [2, 4, 8, 12] {
                let rs = rings(center, radius, 32768.0);
                assert!(rs.len() < 12);
                for pair in rs.windows(2) {
                    assert_eq!(pair[0].outer, pair[1].inner);
                    assert_eq!(pair[0].step * 2.0, pair[1].step);
                }
                for r in rs {
                    assert!(r.outer.x0 < r.inner.x0 && r.outer.x1 > r.inner.x1);
                    for a in [r.inner.x0, r.inner.x1, r.outer.x0, r.outer.x1] {
                        assert_eq!(a.rem_euclid(r.step), 0.0);
                    }
                }
            }
        }
    }
    #[test]
    fn distant_mesh_is_finite_indexed_and_bounded() {
        let g = Generator::new(GeneratorSettings::default(), vec![]);
        let m = build_vista(&g, ChunkCoord { x: -1, z: 1 }, 3, 16384.0);
        assert!(m.positions.len() < 100_000);
        assert!(m.indices.len() < 600_000);
        assert!(m.positions.iter().flatten().all(|v| v.is_finite()));
        assert!(m.indices.iter().all(|&i| (i as usize) < m.positions.len()));
        for tri in m.indices.chunks_exact(3) {
            let a = m.positions[tri[0] as usize];
            let b = m.positions[tri[1] as usize];
            let c = m.positions[tri[2] as usize];
            assert!((b[2] - a[2]) * (c[0] - a[0]) - (b[0] - a[0]) * (c[2] - a[2]) > 0.0);
        }
    }
    #[test]
    fn the_near_border_matches_streamed_terrain_at_negative_coordinates() {
        let g = Generator::new(GeneratorSettings::default(), vec![]);
        let center = ChunkCoord { x: -3, z: -2 };
        for radius in [2, 3, 6, 7] {
            let far = build_vista(&g, center, radius, 2048.0);
            let o = center.origin();
            let step = rings(center, radius, 2048.0)[0].step;
            for (dx, dz, edge) in [
                (0, -radius, 0),
                (radius, 0, 1),
                (0, radius, 2),
                (-radius, 0, 3),
            ] {
                let coord = ChunkCoord {
                    x: center.x + dx,
                    z: center.z + dz,
                };
                let key = ChunkKey {
                    coord,
                    lod: lod_for(coord, center),
                    edges: edge_lods(coord, center),
                    revision: 0,
                };
                let near = build_mesh(&g, key);
                let n = coord.origin();
                let cells = 32usize >> key.lod;
                let near_step = CHUNK_SIZE / cells as f64;
                for along in 0..=(CHUNK_SIZE / step) as usize {
                    let at = along as f64 * step;
                    let (lx, lz) = match edge {
                        0 => (at, 0.0),
                        1 => (CHUNK_SIZE, at),
                        2 => (at, CHUNK_SIZE),
                        _ => (0.0, at),
                    };
                    let (wx, wz) = (n[0] + lx, n[1] + lz);
                    let found = far
                        .positions
                        .iter()
                        .find(|p| p[0] == (wx - o[0]) as f32 && p[2] == (wz - o[1]) as f32)
                        .expect("the far ring must contain every shared boundary vertex");
                    let index = (lz / near_step).round() as usize * (cells + 1)
                        + (lx / near_step).round() as usize;
                    assert!(
                        (near.positions[index][1] - found[1]).abs() < 0.0001,
                        "radius {radius}, edge {edge}, sample {along}"
                    );
                }
            }
        }
    }
}
