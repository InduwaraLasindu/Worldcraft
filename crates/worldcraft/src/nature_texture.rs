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

pub fn bark() -> (Image, Image) {
    let side = 128;
    let mut heights = vec![0.0f32; side * side];
    for y in 0..side {
        for x in 0..side {
            let u = x as f32 / side as f32;
            let v = y as f32 / side as f32;
            let warp = (v * std::f32::consts::TAU * 3.0).sin() * 0.12;
            let groove = ((u * std::f32::consts::TAU * 11.0 + warp).sin() * 0.5 + 0.5).powf(0.32);
            let grain = ((u * 71.0 + v * 7.0) * std::f32::consts::TAU).sin() * 0.045
                + ((u * 29.0 - v * 13.0) * std::f32::consts::TAU).cos() * 0.035;
            heights[y * side + x] = groove + grain;
        }
    }
    let mut color = Vec::new();
    let mut normal = Vec::new();
    for y in 0..side {
        for x in 0..side {
            let h = heights[y * side + x];
            color.push([
                byte(0.12 + h * 0.20),
                byte(0.10 + h * 0.17),
                byte(0.075 + h * 0.13),
                255,
            ]);
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
                p[3] = (p[3] as f32 * hi).clamp(0.0, 255.0).round() as u8;
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
        for pixel in normal.data.unwrap()[..128 * 128 * 4].chunks_exact(4) {
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
