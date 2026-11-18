//! Small original PBR textures, built once and shared by all instances.
//! Explicit mipmaps avoid distant leaf shimmer; no runtime image decoder is needed.
use bevy::{
    asset::RenderAssetUsages,
    image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use worldcraft_core::scatter::Species;
const SIDE: usize = 256;

/// Thirty tapered blades per card; the right quarter is a white opaque
/// lane used by untextured fern/flower/reed geometry in the same ground batch.
/// Both the main and stock shadow/depth pass use this one alpha-tested atlas.
pub fn grass() -> Image {
    let mut pixels = vec![[0.75, 0.75, 0.75, 0.0]; SIDE * SIDE];
    for y in 0..SIDE {
        for x in SIDE * 4 / 5..SIDE {
            pixels[y * SIDE + x] = [1.0; 4];
        }
    }
    for blade in 0..30 {
        let random = |salt: u64| worldcraft_core::generation::unit(0x6a55 ^ salt, blade, 0) as f32;
        // Three tussocks sprout from small crowns. Outer blades splay and
        // droop while central shoots stay upright. Leave air between tussocks.
        let clump = blade / 10;
        let middle = [0.15, 0.37, 0.59][clump as usize];
        let fan = ((blade % 10) as f32 + random(1)) * 0.20 - 1.0;
        let root = middle + fan * 0.046;
        let height = (0.64 + random(2) * 0.33)
            * (1.0 - 0.52 * fan.abs())
            * [0.79, 1.0, 0.88][clump as usize];
        let lean = fan * (0.104 + random(3) * 0.036) + [0.018, -0.027, 0.012][clump as usize];
        let width = 0.0048 + random(4) * 0.004;
        let shade = 0.78 + random(5) * 0.16;
        for y in 0..(height * SIDE as f32).ceil() as usize {
            let t = ((y as f32 + 0.5) / (SIDE as f32 * height)).min(1.0);
            let center = root + lean * t * (0.23 + t * 0.77);
            let half = width * (1.0 - t).powf(0.72);
            let x0 = ((center - half) * SIDE as f32 - 1.0).max(0.0) as usize;
            let x1 = (((center + half) * SIDE as f32 + 1.0) as usize + 1).min(SIDE * 3 / 4);
            for x in x0..x1 {
                let edge = ((x as f32 + 0.5) / SIDE as f32 - center) / width;
                let alpha = ((half - edge.abs() * width) * SIDE as f32 + 0.5).clamp(0.0, 1.0);
                if alpha > pixels[y * SIDE + x][3] {
                    let light = shade + t * 0.07 + (1.0 - edge.abs()) * 0.035;
                    pixels[y * SIDE + x] = [light * (0.93 + 0.07 * t), light, light * 0.87, alpha];
                }
            }
        }
    }
    image(
        pixels
            .iter()
            .map(|p| [byte(p[0]), byte(p[1]), byte(p[2]), byte(p[3])])
            .collect(),
        SIDE,
        true,
        false,
        true,
    )
}

pub fn foliage(species: Species) -> Image {
    let mut pixels = vec![[0.16, 0.24, 0.07, 0.0]; SIDE * SIDE];
    match species {
        Species::Pine => {
            stem(&mut pixels, [0.5, 0.93], [0.5, 0.10], 0.013);
            for row in 0..15 {
                let y = 0.18 + row as f32 * 0.047;
                for side in [-1.0, 1.0] {
                    let reach = 0.13 + (y * 3.0).sin() * 0.20;
                    leaf(
                        &mut pixels,
                        [0.5, y + 0.09],
                        [0.5 + side * reach, y - 0.07],
                        0.014,
                        [0.21, 0.32, 0.13],
                        false,
                    );
                }
            }
        }
        Species::Palm => {
            stem(&mut pixels, [0.5, 0.96], [0.5, 0.04], 0.012);
            for row in 0..12 {
                let y = 0.12 + row as f32 * 0.066;
                for side in [-1.0, 1.0] {
                    leaf(
                        &mut pixels,
                        [0.5, y + 0.12],
                        [0.5 + side * 0.44, y - 0.035],
                        0.026,
                        [0.31, 0.43, 0.14],
                        false,
                    );
                }
            }
        }
        _ => {
            stem(&mut pixels, [0.48, 0.95], [0.55, 0.08], 0.012);
            for (a, b, w) in [
                ([0.50, 0.76], [0.12, 0.59], 0.105),
                ([0.50, 0.67], [0.89, 0.47], 0.108),
                ([0.52, 0.51], [0.15, 0.29], 0.108),
                ([0.53, 0.40], [0.87, 0.19], 0.100),
                ([0.54, 0.28], [0.55, 0.035], 0.090),
            ] {
                leaf(&mut pixels, a, b, w, [0.36, 0.46, 0.17], true);
            }
        }
    }
    // Edge RGB is dilated by using a plausible leaf color even in transparent
    // texels. Mips additionally use alpha-weighted linear RGB, avoiding dark rims.
    image(
        pixels
            .iter()
            .map(|p| [byte(p[0]), byte(p[1]), byte(p[2]), byte(p[3])])
            .collect(),
        SIDE,
        true,
        false,
        true,
    )
}
fn byte(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}
fn stem(pixels: &mut [[f32; 4]], a: [f32; 2], b: [f32; 2], width: f32) {
    let a = Vec2::from_array(a);
    let b = Vec2::from_array(b);
    let d = b - a;
    for y in 0..SIDE {
        for x in 0..SIDE {
            let p = Vec2::new(
                (x as f32 + 0.5) / SIDE as f32,
                (y as f32 + 0.5) / SIDE as f32,
            );
            let t = ((p - a).dot(d) / d.length_squared()).clamp(0.0, 1.0);
            let coverage = ((width - (p - a - d * t).length()) * SIDE as f32 + 0.5).clamp(0.0, 1.0);
            if coverage > pixels[y * SIDE + x][3] {
                pixels[y * SIDE + x] = [0.25, 0.29, 0.10, coverage];
            }
        }
    }
}
fn leaf(
    pixels: &mut [[f32; 4]],
    a: [f32; 2],
    b: [f32; 2],
    width: f32,
    tint: [f32; 3],
    serrated: bool,
) {
    let a = Vec2::from_array(a);
    let d = Vec2::from_array(b) - a;
    let length = d.length();
    let axis = d / length;
    let side = Vec2::new(-axis.y, axis.x);
    for y in 0..SIDE {
        for x in 0..SIDE {
            let p = Vec2::new(
                (x as f32 + 0.5) / SIDE as f32,
                (y as f32 + 0.5) / SIDE as f32,
            ) - a;
            let t = p.dot(axis) / length;
            if !(0.0..=1.0).contains(&t) {
                continue;
            }
            let across = p.dot(side);
            let edge = if serrated {
                1.0 - 0.055 * (t * 90.0).sin().abs()
            } else {
                1.0
            };
            let half = width * (t * std::f32::consts::PI).sin().powf(0.72) * edge;
            let coverage = ((half - across.abs()) * SIDE as f32 + 0.5).clamp(0.0, 1.0);
            if coverage <= pixels[y * SIDE + x][3] {
                continue;
            }
            let vein = 1.0 - ((across.abs() - 0.001) * SIDE as f32).clamp(0.0, 1.0);
            let veinlets = (t * 70.0 - across.abs() * 55.0).sin().abs().powf(18.0) * 0.05;
            let shade =
                0.78 + 0.22 * (1.0 - across.abs() / width).max(0.0) + vein * 0.13 + veinlets;
            pixels[y * SIDE + x] = [tint[0] * shade, tint[1] * shade, tint[2] * shade, coverage];
        }
    }
}

// Periodic value noise allows a continuous seam even when one field warps
// another. Long plates carry smaller cracks, wear and patches of pale lichen.
fn bark_noise(u: f32, v: f32, nx: i64, nz: i64, seed: u64) -> f32 {
    let (x, z) = (u * nx as f32, v * nz as f32);
    let (ix, iz) = (x.floor() as i64, z.floor() as i64);
    let smooth = |t: f32| t * t * (3.0 - 2.0 * t);
    let (fx, fz) = (smooth(x - x.floor()), smooth(z - z.floor()));
    let at = |dx: i64, dz: i64| {
        worldcraft_core::generation::unit(seed, (ix + dx).rem_euclid(nx), (iz + dz).rem_euclid(nz))
            as f32
    };
    let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
    lerp(
        lerp(at(0i64, 0i64), at(1, 0), fx),
        lerp(at(0, 1), at(1, 1), fx),
        fz,
    )
}

pub fn bark() -> (Image, Image) {
    let side = 256;
    let mut heights = vec![0.0; side * side];
    let mut color = Vec::with_capacity(side * side);
    for y in 0..side {
        for x in 0..side {
            let u = x as f32 / side as f32;
            let v = y as f32 / side as f32;
            let warp = bark_noise(u, v, 4, 5, 31);
            let xw = u + 0.074 * (warp - 0.5);
            let plate = bark_noise(xw, v, 16, 5, 63);
            let split = ((plate - 0.47).abs() * 8.2).min(1.0);
            let fiber = bark_noise(xw + 0.006 * plate, v, 74, 16, 82);
            let chips = bark_noise(u, v, 30, 40, 108);
            let h = 0.63 * split + 0.24 * fiber + 0.13 * chips;
            heights[y * side + x] = h;
            let lichen =
                ((bark_noise(u, v, 5, 9, 49) + chips * 0.35 - 0.89) * 3.8).clamp(0.0, 1.0) * split;
            let wood = Vec3::new(0.15, 0.115, 0.084)
                + Vec3::new(0.23, 0.213, 0.18) * (h + 0.18 * (warp - 0.5));
            let rgb = wood.lerp(Vec3::new(0.44, 0.45, 0.36) * (0.82 + chips * 0.30), lichen);
            color.push([byte(rgb.x), byte(rgb.y), byte(rgb.z), 255]);
        }
    }
    let mut normal = Vec::with_capacity(side * side);
    for y in 0..side {
        for x in 0..side {
            let dx = heights[y * side + (x + 1) % side] - heights[y * side + (x + side - 1) % side];
            let dy =
                heights[((y + 1) % side) * side + x] - heights[((y + side - 1) % side) * side + x];
            let n = Vec3::new(-dx * 1.2, -dy * 1.2, 1.0).normalize();
            normal.push([
                byte(n.x * 0.5 + 0.5),
                byte(n.y * 0.5 + 0.5),
                byte(n.z * 0.5 + 0.5),
                255,
            ]);
        }
    }
    (
        image(color, side, false, true, true),
        image(normal, side, false, true, false),
    )
}

/// Original paper-bark albedo and subtle linear normals, shared by every birch.
pub fn birch_bark() -> (Image, Image) {
    let side = 128;
    let mut heights = vec![0.0_f32; side * side];
    let mut colors = Vec::with_capacity(side * side);
    for y in 0..side {
        for x in 0..side {
            let u = x as f32 / side as f32;
            let v = y as f32 / side as f32;
            let scar = worldcraft_core::generation::unit(0xb17c, (x / 16) as i64, (y / 16) as i64)
                < 0.22
                && y % 16 < 2
                && x % 16 < 13;
            let grain = ((u * 61.0 + v * 3.0) * std::f32::consts::TAU).sin() * 0.018;
            let value = if scar { 0.13 + grain } else { 0.80 + grain };
            colors.push([byte(value), byte(value * 0.99), byte(value * 0.92), 255]);
            heights[y * side + x] = grain - if scar { 0.06 } else { 0.0 };
        }
    }
    let mut normals = Vec::with_capacity(side * side);
    for y in 0..side {
        for x in 0..side {
            let dx = heights[y * side + (x + 1) % side] - heights[y * side + (x + side - 1) % side];
            let dy =
                heights[((y + 1) % side) * side + x] - heights[((y + side - 1) % side) * side + x];
            let n = Vec3::new(-dx * 0.65, -dy * 0.65, 1.0).normalize();
            normals.push([
                byte(n.x * 0.5 + 0.5),
                byte(n.y * 0.5 + 0.5),
                byte(n.z * 0.5 + 0.5),
                255,
            ]);
        }
    }
    (
        image(colors, side, false, true, true),
        image(normals, side, false, true, false),
    )
}

fn linear(v: u8) -> f32 {
    let x = v as f32 / 255.0;
    if x <= 0.04045 {
        x / 12.92
    } else {
        ((x + 0.055) / 1.055).powf(2.4)
    }
}
fn srgb(v: f32) -> u8 {
    byte(if v <= 0.0031308 {
        v * 12.92
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    })
}
fn image(mut pixels: Vec<[u8; 4]>, side: usize, cutout: bool, repeat: bool, color: bool) -> Image {
    let mut bytes = Vec::new();
    let mut size = side;
    // Texture allocation is bounded: 256² for foliage, 128² for bark, complete mip chains.
    loop {
        bytes.extend(pixels.iter().flatten().copied());
        if size == 1 {
            break;
        }
        let next = size / 2;
        let mut filtered = Vec::with_capacity(next * next);
        for y in 0..next {
            for x in 0..next {
                let samples = [
                    pixels[(2 * y) * size + 2 * x],
                    pixels[(2 * y) * size + 2 * x + 1],
                    pixels[(2 * y + 1) * size + 2 * x],
                    pixels[(2 * y + 1) * size + 2 * x + 1],
                ];
                let alpha = samples.iter().map(|p| p[3] as f32 / 255.0).sum::<f32>();
                let mut p = [0; 4];
                for c in 0..3 {
                    let sum = samples
                        .iter()
                        .map(|p| {
                            (if color {
                                linear(p[c])
                            } else {
                                p[c] as f32 / 255.0
                            }) * (if cutout { p[3] as f32 / 255.0 } else { 1.0 })
                        })
                        .sum::<f32>();
                    let average = sum / if cutout { alpha.max(0.001) } else { 4.0 };
                    p[c] = if color { srgb(average) } else { byte(average) };
                }
                p[3] = byte(alpha / 4.0);
                filtered.push(p);
            }
        }
        if cutout {
            // Preserve thresholded coverage instead of letting distant crowns vanish.
            let target = pixels.iter().filter(|p| p[3] >= 128).count() as f32 / pixels.len() as f32;
            let mut lo = 0.0;
            let mut hi = 4.0;
            for _ in 0..12 {
                let scale = (lo + hi) * 0.5;
                let coverage = filtered
                    .iter()
                    .filter(|p| p[3] as f32 * scale >= 127.5)
                    .count() as f32
                    / filtered.len() as f32;
                if coverage < target {
                    lo = scale;
                } else {
                    hi = scale;
                }
            }
            for p in &mut filtered {
                // A fully opaque footprint must stay opaque, including the herb lane.
                if p[3] < 255 {
                    p[3] = (p[3] as f32 * hi).clamp(0.0, 255.0).round() as u8;
                }
            }
        }
        pixels = filtered;
        size = next;
    }
    let mut image = Image::new_uninit(
        Extent3d {
            width: side as u32,
            height: side as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        if color {
            TextureFormat::Rgba8UnormSrgb
        } else {
            TextureFormat::Rgba8Unorm
        },
        RenderAssetUsages::default(),
    );
    image.data = Some(bytes);
    image.texture_descriptor.mip_level_count = side.ilog2() + 1;
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: if repeat {
            ImageAddressMode::Repeat
        } else {
            ImageAddressMode::ClampToEdge
        },
        address_mode_v: if repeat {
            ImageAddressMode::Repeat
        } else {
            ImageAddressMode::ClampToEdge
        },
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        ..default()
    });
    image
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn grass_atlas_preserves_the_opaque_herb_lane_and_has_real_blade_holes() {
        let image = grass();
        let bytes = image.data.as_ref().unwrap();
        assert!(bytes.len() < 360_000);
        assert_eq!(image.texture_descriptor.mip_level_count, 9);
        let grass_pixels: Vec<_> = bytes[..SIDE * SIDE * 4]
            .chunks_exact(4)
            .enumerate()
            .filter(|(at, _)| at % SIDE < SIDE * 3 / 4)
            .map(|(_, p)| p)
            .collect();
        let coverage =
            grass_pixels.iter().filter(|p| p[3] >= 128).count() as f32 / grass_pixels.len() as f32;
        assert!((0.04..0.45).contains(&coverage));
        let mut offset = 0;
        for level in 0..5 {
            let side = SIDE >> level;
            let at = offset + (side / 2 * side + side - 1) * 4;
            assert_eq!(
                &bytes[at..at + 4],
                &[255; 4],
                "Herb pigment must sample opaque white at mip {level}"
            );
            offset += side * side * 4;
        }
    }
    #[test]
    fn foliage_mips_retain_cutout_coverage_and_stay_in_a_fixed_memory_budget() {
        for species in [Species::Broadleaf, Species::Pine, Species::Palm] {
            let image = foliage(species);
            let data = image.data.as_ref().unwrap();
            let descriptor = &image.texture_descriptor;
            assert!(descriptor.mip_level_count > 1);
            assert_eq!(descriptor.format, TextureFormat::Rgba8UnormSrgb);
            let mut offset = 0;
            let mut size = SIDE;
            let mut reference: Option<f32> = None;
            for _ in 0..descriptor.mip_level_count {
                let level = &data[offset..offset + size * size * 4];
                let coverage = level.chunks_exact(4).filter(|p| p[3] >= 128).count() as f32
                    / (size * size) as f32;
                if let Some(target) = reference {
                    if size >= 16 {
                        assert!(
                            (coverage - target).abs() < 0.06,
                            "{species:?}: lost foliage coverage at {size}²"
                        );
                    }
                } else {
                    assert!(
                        (0.08..0.70).contains(&coverage),
                        "A foliage texture must contain leaves and holes"
                    );
                    reference = Some(coverage);
                    // Empty border keeps cards from forming rectangular silhouettes.
                    assert!((0..SIDE).all(|x| level[x * 4 + 3] == 0));
                }
                offset += size * size * 4;
                size = (size / 2).max(1);
            }
            assert_eq!(offset, data.len());
            assert!(
                data.len() < 360_000,
                "A shared foliage texture exceeded its allocation budget"
            );
        }
    }
    #[test]
    fn bark_normals_are_linear_and_bark_has_no_transparency() {
        let (color, normal) = bark();
        assert_eq!(normal.texture_descriptor.format, TextureFormat::Rgba8Unorm);
        assert_eq!(
            color.texture_descriptor.format,
            TextureFormat::Rgba8UnormSrgb
        );
        assert!(color.data.unwrap().chunks_exact(4).all(|p| p[3] == 255));
        let side = normal.texture_descriptor.size.width as usize;
        for pixel in normal.data.unwrap()[..side * side * 4].chunks_exact(4) {
            let n = Vec3::new(
                pixel[0] as f32 / 127.5 - 1.0,
                pixel[1] as f32 / 127.5 - 1.0,
                pixel[2] as f32 / 127.5 - 1.0,
            );
            assert!((n.length() - 1.0).abs() < 0.015);
            assert!(n.z > 0.5);
        }
    }
}
