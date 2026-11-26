//! Original procedural and CC0 scanned ground surfaces. Only the selected pair is uploaded.
use bevy::{
    asset::RenderAssetUsages,
    image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
pub(super) const SIDE: u32 = 1024;
const LAYERS: u32 = 3;
fn array(bytes: &[u8], format: TextureFormat, side: u32) -> Image {
    let expected = (0..=SIDE.ilog2())
        .map(|m| (SIDE >> m).max(1).pow(2) * 4 * LAYERS)
        .sum::<u32>() as usize;
    assert_eq!(
        bytes.len(),
        expected,
        "Run python tools/prepare_materials.py before building"
    );
    let slopes = format == TextureFormat::Rgba8Unorm;
    let layers = LAYERS + u32::from(slopes);
    let mut image = Image::new_uninit(
        Extent3d {
            width: side,
            height: side,
            depth_or_array_layers: layers,
        },
        TextureDimension::D2,
        format,
        RenderAssetUsages::default(),
    );
    image.texture_descriptor.mip_level_count = side.ilog2() + 1;
    let layer_bytes = expected / LAYERS as usize;
    let skip = if side == SIDE {
        0
    } else {
        (SIDE * SIDE * 4) as usize
    };
    let mut data = Vec::with_capacity((layer_bytes - skip) * layers as usize);
    for layer in bytes.chunks_exact(layer_bytes) {
        data.extend_from_slice(&layer[skip..]);
    }
    if slopes {
        let sediment = include_bytes!("../embedded/shore-sediment.bin");
        assert_eq!(sediment.len(), layer_bytes);
        data.extend_from_slice(&sediment[skip..]);
    }
    image.data = Some(data);
    image.sampler = sampler();
    image
}
pub fn sampler() -> ImageSampler {
    ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        ..default()
    })
}
fn normal(procedural: bool, base: u32) -> Image {
    let bytes = if procedural {
        include_bytes!("../embedded/procedural-normal-rough.bin")
    } else {
        include_bytes!("../embedded/terrain-normal-rough.bin")
    };
    array(bytes, TextureFormat::Rgba8Unorm, base)
}
fn color_height(procedural: bool, side: u32, bc: bool, base: u32) -> (Image, Image) {
    if side > 0 {
        return crate::relief_texture::load(side, bc);
    }
    let bytes = if procedural {
        include_bytes!("../embedded/procedural-color.bin")
    } else {
        include_bytes!("../embedded/terrain-color.bin")
    };
    (
        array(bytes, TextureFormat::Rgba8UnormSrgb, base),
        crate::relief_texture::neutral(),
    )
}
#[derive(Resource)]
pub struct ActiveSurfaces(pub bool, pub u32, pub bool, pub u32);
fn base_resolution(
    limits: &crate::performance::Performance,
    quality: crate::session::Quality,
) -> u32 {
    if limits.quality(quality) == crate::session::Quality::High
        && limits.mode() != crate::performance::Mode::Eco
    {
        SIDE
    } else {
        SIDE / 2
    }
}
pub fn initial(
    limits: &crate::performance::Performance,
    quality: crate::session::Quality,
    device: Option<&bevy::render::renderer::RenderDevice>,
) -> (ActiveSurfaces, Image, Image, Image) {
    let proc = limits.preferences.render.procedural_surfaces;
    let (side, bc) = crate::relief_texture::effective(limits, quality, device);
    let base = base_resolution(limits, quality);
    let (color, height) = color_height(proc, side, bc, base);
    (
        ActiveSurfaces(proc, side, bc, base),
        color,
        normal(proc, base),
        height,
    )
}
pub fn update(
    limits: Res<crate::performance::Performance>,
    graphics: Res<crate::session::Graphics>,
    device: Option<Res<bevy::render::renderer::RenderDevice>>,
    mut active: ResMut<ActiveSurfaces>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<crate::surface::SurfaceMaterial>>,
) {
    let requested = limits.preferences.render.procedural_surfaces;
    let (side, bc) = crate::relief_texture::effective(&limits, graphics.quality, device.as_deref());
    let base = base_resolution(&limits, graphics.quality);
    if (active.0, active.1, active.2, active.3) == (requested, side, bc, base) {
        return;
    }
    let (color, height) = color_height(requested, side, bc, base);
    let (color, height) = (images.add(color), images.add(height));
    let normal =
        (active.0 != requested || active.3 != base).then(|| images.add(normal(requested, base)));
    for (_, material) in materials.iter_mut() {
        material.extension.terrain_color = color.clone();
        material.extension.terrain_height = height.clone();
        material.extension.parameters.y = side as f32;
        if let Some(normal) = &normal {
            material.extension.terrain_normal = normal.clone();
        }
    }
    *active = ActiveSurfaces(requested, side, bc, base);
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn embedded_material_arrays_have_complete_linear_normal_mips_with_bounded_allocation() {
        for procedural in [true, false] {
            for side in [512, 1024] {
                check_arrays(
                    (
                        color_height(procedural, 0, false, side).0,
                        normal(procedural, side),
                    ),
                    side,
                );
            }
        }
    }
    fn check_arrays((color, normal): (Image, Image), side: u32) {
        assert_eq!(color.texture_descriptor.size.depth_or_array_layers, 3);
        assert_eq!(color.texture_descriptor.mip_level_count, side.ilog2() + 1);
        assert_eq!(normal.texture_descriptor.format, TextureFormat::Rgba8Unorm);
        let a = color.data.unwrap();
        let b = normal.data.unwrap();
        assert!(a.len() + b.len() < 38 * (side as usize).pow(2));
        assert_eq!(b.len() * 3, a.len() * 4);
        for pixel in b.chunks_exact(4).step_by(97) {
            let n = Vec3::new(
                pixel[0] as f32 / 127.5 - 1.0,
                pixel[1] as f32 / 127.5 - 1.0,
                pixel[2] as f32 / 127.5 - 1.0,
            );
            assert!((n.length() - 1.0).abs() < 0.02, "Non-unit packed normal");
        }
    }
}
