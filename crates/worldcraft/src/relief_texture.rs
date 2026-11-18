//! Three tileable 4K PBR maps. A quality change uploads only the selected mip tails.
use crate::{
    performance::{Mode, Performance},
    session::Quality,
};
use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::{
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        renderer::RenderDevice,
    },
};

const SIDE: u32 = 4096;
const LAYERS: u32 = 3;
static COLOR: &[u8] = include_bytes!("../embedded/procedural-pbr-bc3.bin");
static HEIGHT: &[u8] = include_bytes!("../embedded/procedural-height-bc4.bin");

pub fn effective(
    limits: &Performance,
    quality: Quality,
    device: Option<&RenderDevice>,
) -> (u32, bool) {
    let quality = limits.quality(quality);
    let Some(device) = device else {
        return (0, false);
    };
    let bc = device
        .features()
        .contains(wgpu::Features::TEXTURE_COMPRESSION_BC);
    if !limits.preferences.render.procedural_surfaces
        || quality == Quality::Low
        || limits.mode() == Mode::Eco
    {
        return (0, bc);
    }
    let cap = device
        .limits()
        .max_texture_dimension_2d
        .min(if bc { SIDE } else { 1024 });
    let side = limits
        .preferences
        .render
        .surface_resolution
        .side(quality == Quality::High, limits.software_gpu);
    (side.min(1 << cap.ilog2()), bc)
}

fn mip_len(side: u32, block: usize) -> usize {
    (side.div_ceil(4) as usize).pow(2) * block
}
fn chain_len(side: u32, block: usize) -> usize {
    (0..=side.ilog2()).map(|m| mip_len(side >> m, block)).sum()
}
fn image(side: u32, format: TextureFormat, data: Vec<u8>) -> Image {
    let mut img = Image::new_uninit(
        Extent3d {
            width: side,
            height: side,
            depth_or_array_layers: LAYERS,
        },
        TextureDimension::D2,
        format,
        RenderAssetUsages::RENDER_WORLD,
    );
    img.texture_descriptor.mip_level_count = side.ilog2() + 1;
    img.sampler = super::terrain_texture::sampler();
    img.data = Some(data);
    img
}
pub fn neutral() -> Image {
    image(1, TextureFormat::R8Unorm, vec![128; LAYERS as usize])
}
pub fn load(side: u32, bc: bool) -> (Image, Image) {
    (
        unpack(COLOR, side, true, bc),
        unpack(HEIGHT, side, false, bc),
    )
}
fn unpack(bytes: &[u8], side: u32, color: bool, bc: bool) -> Image {
    let block = if color { 16 } else { 8 };
    let full = chain_len(SIDE, block);
    let tail = chain_len(side, block);
    assert_eq!(bytes.len(), full * LAYERS as usize);
    let mut data = Vec::with_capacity(if bc {
        tail * LAYERS as usize
    } else {
        (0..=side.ilog2())
            .map(|m| ((side >> m) as usize).pow(2))
            .sum::<usize>()
            * LAYERS as usize
            * if color { 4 } else { 1 }
    });
    for layer in bytes.chunks_exact(full) {
        let mut level = &layer[full - tail..];
        if bc {
            data.extend_from_slice(level);
            continue;
        }
        for mip in 0..=side.ilog2() {
            let size = side >> mip;
            let (src, rest) = level.split_at(mip_len(size, block));
            decode_level(src, size as usize, color, &mut data);
            level = rest;
        }
    }
    let format = match (color, bc) {
        (true, true) => TextureFormat::Bc3RgbaUnormSrgb,
        (true, false) => TextureFormat::Rgba8UnormSrgb,
        (false, true) => TextureFormat::Bc4RUnorm,
        (false, false) => TextureFormat::R8Unorm,
    };
    image(side, format, data)
}

/// Scalar BC4 and BC3's alpha block share this interpolation table.
fn scalar(block: &[u8], i: usize) -> u8 {
    let bits = u64::from_le_bytes(block[..8].try_into().unwrap());
    let k = ((bits >> (16 + 3 * i)) & 7) as u16;
    let (a, b) = (block[0] as u16, block[1] as u16);
    (match k {
        0 => a,
        1 => b,
        _ if a > b => ((8 - k) * a + (k - 1) * b) / 7,
        6 => 0,
        7 => 255,
        _ => ((6 - k) * a + (k - 1) * b) / 5,
    }) as u8
}
fn rgb565(bytes: &[u8]) -> [u16; 3] {
    let c = u16::from_le_bytes(bytes[..2].try_into().unwrap());
    let (r, g, b) = (c >> 11, (c >> 5) & 63, c & 31);
    [r << 3 | r >> 2, g << 2 | g >> 4, b << 3 | b >> 2]
}
fn decode_level(src: &[u8], side: usize, color: bool, out: &mut Vec<u8>) {
    let channels = if color { 4 } else { 1 };
    let block = if color { 16 } else { 8 };
    let start = out.len();
    out.resize(start + side * side * channels, 0);
    for (index, chunk) in src.chunks_exact(block).enumerate() {
        let x0 = (index % side.div_ceil(4)) * 4;
        let y0 = (index / side.div_ceil(4)) * 4;
        let mut palette = [[0u8; 3]; 4];
        let mut selectors = 0;
        if color {
            let (a, b) = (rgb565(&chunk[8..]), rgb565(&chunk[10..]));
            for k in 0..3 {
                palette[0][k] = a[k] as u8;
                palette[1][k] = b[k] as u8;
                palette[2][k] = ((2 * a[k] + b[k]) / 3) as u8;
                palette[3][k] = ((a[k] + 2 * b[k]) / 3) as u8;
            }
            selectors = u32::from_le_bytes(chunk[12..16].try_into().unwrap());
        }
        for y in 0..4.min(side - y0) {
            for x in 0..4.min(side - x0) {
                let i = 4 * y + x;
                let dest = start + ((y0 + y) * side + x0 + x) * channels;
                if color {
                    let c = palette[((selectors >> (2 * i)) & 3) as usize];
                    out[dest..dest + 3].copy_from_slice(&c);
                }
                out[dest + channels - 1] = scalar(chunk, i);
            }
        }
    }
}

/// Solari samples standard 2D material maps at triangle hits. Keep one 256²
/// stone pair shared by every boulder. Alpha on the color map is always opaque.
pub fn ray_stone(procedural: bool) -> (Image, Image) {
    let side = 256_u32;
    let base_len = |s: u32| {
        (0..=s.ilog2())
            .map(|m| 4 * ((s >> m) as usize).pow(2))
            .sum::<usize>()
    };
    let bytes = base_len(side);
    let mut color = Vec::with_capacity(bytes);
    let mut rough = Vec::with_capacity(bytes);
    if procedural {
        let end = 2 * chain_len(SIDE, 16);
        let mut src = &COLOR[end - chain_len(side, 16)..end];
        for m in 0..=side.ilog2() {
            let size = side >> m;
            let (level, rest) = src.split_at(mip_len(size, 16));
            decode_level(level, size as usize, true, &mut color);
            src = rest;
        }
        for c in color.chunks_exact_mut(4) {
            rough.extend_from_slice(&[255, c[3], 0, 255]);
            c[3] = 255;
        }
    } else {
        let end = 2 * base_len(512);
        let src = &include_bytes!("../embedded/terrain-color.bin")[end - bytes..end];
        let nr = &include_bytes!("../embedded/terrain-normal-rough.bin")[end - bytes..end];
        color.extend_from_slice(src);
        for c in nr.chunks_exact(4) {
            rough.extend_from_slice(&[255, c[3], 0, 255]);
        }
    }
    let make = |format, data| {
        let mut img = image(side, format, data);
        img.texture_descriptor.size.depth_or_array_layers = 1;
        img
    };
    (
        make(TextureFormat::Rgba8UnormSrgb, color),
        make(TextureFormat::Rgba8Unorm, rough),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn each_quality_uses_only_the_selected_layers_and_mips() {
        for side in [1024, 2048, 4096] {
            let (c, h) = load(side, true);
            let (c, h) = (c.data.unwrap(), h.data.unwrap());
            assert_eq!(
                c.len() + h.len(),
                3 * (chain_len(side, 16) + chain_len(side, 8))
            );
            assert!(c.len() + h.len() <= 96 * 1024 * 1024 + 256);
            for (src, tail, block) in [(COLOR, &c, 16), (HEIGHT, &h, 8)] {
                for (layer, part) in src
                    .chunks_exact(chain_len(SIDE, block))
                    .zip(tail.chunks_exact(chain_len(side, block)))
                {
                    assert!(layer.ends_with(part));
                }
            }
        }
    }
    #[test]
    fn secondary_stone_uses_opaque_pigment_and_separate_dielectric_roughness() {
        for proc in [false, true] {
            let (color, rough) = ray_stone(proc);
            for img in [&color, &rough] {
                assert_eq!(img.texture_descriptor.size.depth_or_array_layers, 1);
                assert_eq!(img.texture_descriptor.size.width, 256);
                assert_eq!(img.texture_descriptor.mip_level_count, 9);
            }
            let color = color.data.unwrap();
            let rough = rough.data.unwrap();
            assert_eq!(color.len(), rough.len());
            assert!(color.chunks_exact(4).all(|c| c[3] == 255));
            assert!(rough.chunks_exact(4).all(|c| c[2] == 0 && c[3] == 255));
            assert!(
                rough.chunks_exact(4).map(|c| c[1]).min()
                    < rough.chunks_exact(4).map(|c| c[1]).max()
            );
        }
    }
    #[test]
    fn non_bc_decode_crops_small_mips_and_preserves_scalar_endpoints() {
        let mut test = vec![0u8; 16];
        test[0] = 201;
        test[1] = 7;
        test[2] = 1;
        test[8] = 0;
        test[9] = 248;
        let mut pixels = Vec::new();
        decode_level(&test, 2, true, &mut pixels);
        assert_eq!(pixels.len(), 16);
        assert_eq!(&pixels[..8], &[255, 0, 0, 7, 255, 0, 0, 201]);
        let (c, h) = load(32, false);
        assert_eq!(c.texture_descriptor.format, TextureFormat::Rgba8UnormSrgb);
        assert_eq!(h.data.unwrap().len(), 3 * (1024 + 256 + 64 + 16 + 4 + 1));
    }
}
