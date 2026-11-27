//! The world material pack stores each BC3 mip independently. Small presets
//! seek past the 4K payloads and never inflate or upload them.
use bevy::{
    asset::RenderAssetUsages,
    image::{ImageAddressMode, ImageSampler},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use flate2::read::ZlibDecoder;
use std::{
    fs::File,
    io::{self, Cursor, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    sync::OnceLock,
};

const TOP: u32 = 4096;
pub fn directory() -> Option<&'static Path> {
    static ROOT: OnceLock<Option<PathBuf>> = OnceLock::new();
    ROOT.get_or_init(|| {
        let installed = std::env::current_exe()
            .ok()?
            .parent()?
            .join("world-materials");
        [
            installed,
            Path::new(env!("CARGO_MANIFEST_DIR")).join("embedded"),
        ]
        .into_iter()
        .find(|p| p.join("sediment.wct").is_file())
    })
    .as_deref()
}

pub fn load(names: &[&str], side: u32, srgb: bool, bc: bool, repeat: bool) -> io::Result<Image> {
    let root = directory().ok_or_else(|| io::Error::other("World material pack unavailable"))?;
    let last: u32 = if names == ["skyline"] { 4 } else { 1 };
    let files = names
        .iter()
        .map(|name| File::open(root.join(format!("{name}.wct"))));
    load_mips(files, names.len(), TOP, last, side, srgb, bc, repeat)
}

pub fn bundled(layers: &[&[u8]], side: u32, srgb: bool, bc: bool, repeat: bool) -> Image {
    load_mips(
        layers.iter().map(|layer| Ok(Cursor::new(layer))),
        layers.len(),
        512,
        1,
        side.clamp(1, 512),
        srgb,
        bc,
        repeat,
    )
    .expect("Bundled art has a complete mip chain")
}

#[allow(clippy::too_many_arguments)]
fn load_mips<T: Read + Seek>(
    files: impl Iterator<Item = io::Result<T>>,
    layers: usize,
    top: u32,
    last: u32,
    side: u32,
    srgb: bool,
    bc: bool,
    repeat: bool,
) -> io::Result<Image> {
    let mips = side.ilog2() - last.ilog2() + 1;
    let bytes = if bc { 1 } else { 4 };
    let mut data = Vec::with_capacity(layers * side as usize * side as usize * bytes * 4 / 3 + 256);
    for file in files {
        let mut f = file?;
        let levels = top.ilog2() - last.ilog2() + 1;
        let mut sizes = vec![0u8; 4 * levels as usize];
        f.read_exact(&mut sizes)?;
        for (i, row) in sizes.chunks_exact(4).enumerate() {
            let length = u32::from_le_bytes(row.try_into().unwrap());
            let size = top >> i;
            if size > side {
                f.seek(SeekFrom::Current(i64::from(length)))?;
                continue;
            }
            let source = (&mut f).take(length.into());
            let mut z = ZlibDecoder::new(source);
            let expected = (size.div_ceil(4) as usize).pow(2) * 16;
            if bc {
                let start = data.len();
                data.resize(start + expected, 0);
                z.read_exact(&mut data[start..])?;
            } else {
                let mut blocks = vec![0; expected];
                z.read_exact(&mut blocks)?;
                crate::relief_texture::decode_level(&blocks, size as usize, true, &mut data);
            }
            if z.read(&mut [0])? != 0 || z.total_in() != u64::from(length) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Invalid WorldCraft texture mip",
                ));
            }
        }
    }
    let format = match (bc, srgb) {
        (true, true) => TextureFormat::Bc3RgbaUnormSrgb,
        (true, false) => TextureFormat::Bc3RgbaUnorm,
        (false, true) => TextureFormat::Rgba8UnormSrgb,
        (false, false) => TextureFormat::Rgba8Unorm,
    };
    let mut img = Image::new_uninit(
        Extent3d {
            width: side,
            height: side,
            depth_or_array_layers: layers as u32,
        },
        TextureDimension::D2,
        format,
        RenderAssetUsages::RENDER_WORLD,
    );
    img.texture_descriptor.mip_level_count = mips;
    img.sampler = crate::terrain_texture::sampler();
    if !repeat {
        if let ImageSampler::Descriptor(s) = &mut img.sampler {
            s.address_mode_u = ImageAddressMode::ClampToEdge;
            s.address_mode_v = ImageAddressMode::ClampToEdge;
        }
    }
    img.data = Some(data);
    Ok(img)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selected_mips_and_array_layers_match_the_4k_sources() {
        for names in [
            &["grass"][..],
            &["skyline"][..],
            &[
                "proc-normal-0",
                "proc-normal-1",
                "proc-normal-2",
                "sediment",
            ][..],
        ] {
            let full = load(names, 4096, true, true, false).unwrap();
            let small = load(names, 1024, true, true, false).unwrap();
            let levels = if names == ["skyline"] { 11 } else { 13 };
            assert_eq!(full.texture_descriptor.mip_level_count, levels);
            assert_eq!(small.texture_descriptor.size.width, 1024);
            let a = full.data.unwrap();
            let b = small.data.unwrap();
            for (source, mips) in a
                .chunks_exact(a.len() / names.len())
                .zip(b.chunks_exact(b.len() / names.len()))
            {
                assert!(source.ends_with(mips));
            }
        }
        for family in ["broadleaf", "birch", "acacia", "pine", "palm"] {
            let cutout = load(&[family], 512, true, false, false)
                .unwrap()
                .data
                .unwrap();
            assert!(
                cutout[..512 * 4].chunks_exact(4).all(|p| p[3] < 128),
                "{family} has clipped upper foliage"
            );
        }
        let herb = load(&["grass"], 512, true, false, false)
            .unwrap()
            .data
            .unwrap();
        let lane = (256 * 512 + 502) * 4;
        assert_eq!(&herb[lane..lane + 4], &[255; 4]);
    }
}
