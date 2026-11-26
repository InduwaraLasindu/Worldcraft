//! Distant trees share one cutout atlas. Streamed tiles use existing tree identities.
//! Outside them, the same candidate lattice is seated on the rendered clipmap triangles.
use bevy::{
    asset::RenderAssetUsages,
    image::{ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use std::collections::BTreeMap;
use worldcraft_core::{
    generation::{hash, unit},
    scatter::{canopy_position, canopy_species, skyline_cell, Instance, Species},
    ChunkCoord, Generator, MeshData, CHUNK_SIZE,
};

// One tile of slack covers every eye position in the clipmap's anchor tile.
// The material tapers cards to zero by 1536 m measured from the actual eye.
const BUILD_RADIUS: f32 = 1536.0 + CHUNK_SIZE as f32;

pub fn atlas() -> Image {
    let mut image = Image::new_uninit(
        Extent3d {
            width: 512,
            height: 512,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.texture_descriptor.mip_level_count = 8;
    image.data = Some(include_bytes!("../embedded/skyline-rgba.bin").to_vec());
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        ..default()
    });
    image
}

#[derive(Default)]
pub struct Batch {
    cards: MeshData,
    roots: Vec<[f32; 3]>,
    bounds: Option<(Vec3, Vec3)>,
}
impl Batch {
    fn tree(&mut self, x: f32, y: f32, z: f32, species: Species, scale: f32, variant: usize) {
        // Widths and heights include the small border inside each 128px atlas tile.
        let (family, width, height) = match species {
            Species::Broadleaf => (0, 11.5, 14.0),
            Species::Pine => (1, 9.5, 15.4),
            Species::Birch => (2, 8.1, 14.7),
            Species::Acacia => (3, 13.0, 9.4),
            Species::Palm => (4, 11.0, 14.0),
            _ => return,
        };
        let w = width * scale * 0.5;
        let h = height * scale;
        let root = Vec3::new(x, y - h * 0.03, z);
        let min = root - Vec3::new(w, 0.0, w);
        let max = root + Vec3::new(w, h, w);
        let (lo, hi) = self.bounds.get_or_insert((min, max));
        *lo = lo.min(min);
        *hi = hi.max(max);
        let tile = if species == Species::Pine && variant % 4 == 3 {
            15
        } else {
            family * 3 + variant % 3
        };
        let tile_uv = Vec2::new((tile % 4) as f32, (tile / 4) as f32);
        let base = self.cards.positions.len() as u32;
        for (side, top) in [(-1.0, 0.0), (1.0, 0.0), (1.0, 1.0), (-1.0, 1.0)] {
            let uv = (tile_uv + Vec2::new(0.5 + side * 0.499, 0.999 - top * 0.998)) / 4.0;
            self.cards.vertex(
                (root + Vec3::new(side * w, h * top, 0.0)).to_array(),
                Vec3::new(side * 0.46, 0.65 + top * 0.28, 0.40)
                    .normalize()
                    .to_array(),
                [1.0; 4],
                uv.to_array(),
            );
            self.roots.push(root.to_array());
        }
        self.cards
            .indices
            .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    pub fn into_mesh(mut self) -> Option<Mesh> {
        let (lo, hi) = self.bounds?;
        // The dynamic billboard can rotate out of its authored XY plane.
        for bound in [lo, hi] {
            self.cards
                .vertex(bound.to_array(), [0.0, 1.0, 0.0], [1.0; 4], [0.0; 2]);
            self.roots.push(bound.to_array());
        }
        // A 256 m sector contains at most 512 admitted cells. Keep the
        // silhouette mesh lean and omit the constant white vertex color.
        debug_assert!(self.cards.positions.len() <= 2050);
        let indices = self.cards.indices.into_iter().map(|i| i as u16).collect();
        let mut m = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.cards.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.cards.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.cards.uv)
        .with_inserted_indices(Indices::U16(indices));
        m.insert_attribute(crate::surface::OUTER_CENTER, self.roots);
        Some(m)
    }
}

pub fn near(trees: &[Instance], coord: ChunkCoord, seed: u64) -> Batch {
    let mut batch = Batch::default();
    let [ox, oz] = coord.origin();
    for p in trees.iter().filter(|p| p.species.is_tree()) {
        let [x, y, z] = p.position;
        if skyline_cell(seed, (x / 8.0).floor() as i64, (z / 8.0).floor() as i64) {
            batch.tree(
                (x - ox) as f32,
                y as f32,
                (z - oz) as f32,
                p.species,
                p.scale,
                p.variant,
            );
        }
    }
    batch
}

pub fn horizon(g: &Generator, center: ChunkCoord, mesh: &MeshData) -> Vec<Batch> {
    if g.version() < 3 {
        return vec![];
    }
    let mut batches = BTreeMap::new();
    let mut climates = BTreeMap::new();
    let seed = g.settings().seed;
    let [ox, oz] = center.origin();
    let middle = CHUNK_SIZE as f32 * 0.5;
    // Each clipmap quad uses [a,c,b,b,c,d]. Interpolate on its actual
    // diagonal, including the stitched outer edge, rather than querying a
    // different height field that could leave trees floating above a ridge.
    for q in mesh.indices.chunks_exact(6) {
        let ids = [q[0], q[2], q[1], q[5]].map(|i| i as usize);
        let [a, b, c, d] = ids.map(|i| mesh.positions[i]);
        if a[0] > middle + BUILD_RADIUS
            || a[2] > middle + BUILD_RADIUS
            || d[0] < middle - BUILD_RADIUS
            || d[2] < middle - BUILD_RADIUS
        {
            continue;
        }
        let covers = ids.map(|i| mesh.uv[i][0]);
        if covers.iter().all(|w| *w <= 0.0) {
            continue;
        }
        let x0 = ox + f64::from(a[0]);
        let z0 = oz + f64::from(a[2]);
        let x1 = ox + f64::from(d[0]);
        let z1 = oz + f64::from(d[2]);
        let span = f64::from(b[0] - a[0]);
        for gz in (z0 / 8.0).floor() as i64..(z1 / 8.0).ceil() as i64 {
            for gx in (x0 / 8.0).floor() as i64..(x1 / 8.0).ceil() as i64 {
                if !skyline_cell(seed, gx, gz) {
                    continue;
                }
                let [x, z] = canopy_position(seed, gx, gz);
                if x < x0 || x >= x1 || z < z0 || z >= z1 {
                    continue;
                }
                let local = Vec2::new((x - ox) as f32, (z - oz) as f32);
                if (local - Vec2::splat(middle)).length_squared() > BUILD_RADIUS * BUILD_RADIUS {
                    continue;
                }
                let u = ((x - x0) / span) as f32;
                let v = ((z - z0) / span) as f32;
                let interpolate = |[a, b, c, d]: [f32; 4]| {
                    if u + v <= 1.0 {
                        a + (b - a) * u + (c - a) * v
                    } else {
                        d + (c - d) * (1.0 - u) + (b - d) * (1.0 - v)
                    }
                };
                let cover = interpolate(covers);
                if unit(seed ^ 0xe203, gx, gz) >= f64::from(cover) {
                    continue;
                }
                let y = interpolate([a[1], b[1], c[1], d[1]]);
                if f64::from(y) < g.settings().sea_level + 1.0 {
                    continue;
                }
                let tile = ChunkCoord::at(x, z);
                let (blend, temperature) = climates.entry(tile).or_insert_with(|| {
                    let o = tile.origin();
                    let (cx, cz) = (o[0] + CHUNK_SIZE * 0.5, o[1] + CHUNK_SIZE * 0.5);
                    let climate = g.preview_sample(cx, cz);
                    (g.biome_blend(cx, cz, &climate), climate.temperature)
                });
                let species =
                    canopy_species(blend, *temperature, unit(seed ^ 0xe208, gx, gz) as f32);
                let stand = (0.5
                    + worldcraft_core::geology::gradient(seed ^ 0xe204, x / 180.0, z / 180.0))
                .clamp(0.0, 1.0);
                let scale = 0.62 + stand as f32 * 0.72 + unit(seed ^ 0xe206, gx, gz) as f32 * 0.58;
                let variant = (hash(seed ^ 0xe209, gx, gz) % 4) as usize;
                let sector = (tile.x.div_euclid(2), tile.z.div_euclid(2));
                batches
                    .entry(sector)
                    .or_insert_with(Batch::default)
                    .tree(local.x, y, local.y, species, scale, variant);
            }
        }
    }
    batches.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sparse_tiles_retain_the_same_real_trees_and_fixed_geometry_budget() {
        let g = Generator::new(
            worldcraft_core::GeneratorSettings::for_preset(worldcraft_core::Preset::Temperate, 42),
            vec![],
        );
        let seed = g.settings().seed;
        let mut admitted = 0;
        for coord in [
            ChunkCoord { x: -14, z: -14 },
            ChunkCoord { x: -13, z: -15 },
            ChunkCoord { x: 1, z: 0 },
        ] {
            let all = worldcraft_core::scatter::scatter(&g, coord, 1);
            let exact: Vec<_> = all
                .iter()
                .filter(|p| {
                    p.species.is_tree()
                        && skyline_cell(
                            seed,
                            (p.position[0] / 8.0).floor() as i64,
                            (p.position[2] / 8.0).floor() as i64,
                        )
                })
                .copied()
                .collect();
            assert_eq!(worldcraft_core::scatter::skyline_scatter(&g, coord), exact);
            let batch = near(&all, coord, seed);
            assert_eq!(batch.cards.indices.len(), 6 * exact.len());
            assert!(exact.len() <= 128);
            admitted += exact.len();
        }
        assert!(admitted > 0);
    }
    #[test]
    fn clipmap_cards_follow_both_triangle_planes_and_clear_water_and_clearings() {
        let mut settings = worldcraft_core::GeneratorSettings::default();
        settings.sea_level = -100.0;
        let g = Generator::new(settings, vec![]);
        let origin = ChunkCoord { x: -3, z: 1 };
        let mut mesh = MeshData::default();
        for p in [
            [0., 20., 0.],
            [64., 52., 0.],
            [0., 36., 64.],
            [64., 100., 64.],
        ] {
            mesh.vertex(p, [0., 1., 0.], [1.; 4], [1., 1.]);
        }
        mesh.indices = vec![0, 2, 1, 1, 2, 3];
        let batches = horizon(&g, origin, &mesh);
        let batch = &batches[0];
        assert_eq!(batch.roots.len(), 32 * 4);
        for i in (0..batch.roots.len()).step_by(4) {
            let [x, y, z] = batch.roots[i];
            let (u, v) = (x / 64.0, z / 64.0);
            let height = if u + v <= 1.0 {
                20.0 + 32.0 * u + 16.0 * v
            } else {
                100.0 + (36.0 - 100.0) * (1.0 - u) + (52.0 - 100.0) * (1.0 - v)
            };
            let h = batch.cards.positions[i + 2][1] - y;
            assert!((y + 0.03 * h - height).abs() < 0.0001);
        }
        let mut ocean = mesh.clone();
        for p in &mut ocean.positions {
            p[1] = -200.0;
        }
        assert!(horizon(&g, origin, &ocean).is_empty());
        mesh.uv.fill([0.0; 2]);
        assert!(horizon(&g, origin, &mesh).is_empty());
    }
}
