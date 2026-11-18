//! Original procedural and CC0 scanned ground surfaces. Only the selected pair is uploaded.
use bevy::{
    asset::RenderAssetUsages,
    image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
const SIDE: u32 = 512;
const LAYERS: u32 = 3;
fn array(bytes: &[u8], format: TextureFormat) -> Image {
    let expected = (0..10)
        .map(|m| (SIDE >> m).max(1).pow(2) * 4 * LAYERS)
        .sum::<u32>() as usize;
    assert_eq!(
        bytes.len(),
        expected,
        "Run python tools/prepare_materials.py before building"
    );
    let mut image = Image::new_uninit(
        Extent3d {
            width: SIDE,
            height: SIDE,
            depth_or_array_layers: LAYERS,
        },
        TextureDimension::D2,
        format,
        RenderAssetUsages::default(),
    );
    image.texture_descriptor.mip_level_count = 10;
    image.data = Some(bytes.to_vec());
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
fn normal(procedural: bool) -> Image {
    let bytes = if procedural {
        include_bytes!("../embedded/procedural-normal-rough.bin")
    } else {
        include_bytes!("../embedded/terrain-normal-rough.bin")
    };
    array(bytes, TextureFormat::Rgba8Unorm)
}
fn color_height(procedural: bool, side: u32, bc: bool) -> (Image, Image) {
    if side > 0 {
        return crate::relief_texture::load(side, bc);
    }
    let bytes = if procedural {
        include_bytes!("../embedded/procedural-color.bin")
    } else {
        include_bytes!("../embedded/terrain-color.bin")
    };
    (
        array(bytes, TextureFormat::Rgba8UnormSrgb),
        crate::relief_texture::neutral(),
    )
}
#[derive(Resource)]
pub struct ActiveSurfaces(pub bool, pub u32, pub bool);
pub fn initial(
    limits: &crate::performance::Performance,
    quality: crate::session::Quality,
    device: Option<&bevy::render::renderer::RenderDevice>,
) -> (ActiveSurfaces, Image, Image, Image) {
    let proc = limits.preferences.render.procedural_surfaces;
    let (side, bc) = crate::relief_texture::effective(limits, quality, device);
    let (color, height) = color_height(proc, side, bc);
    (ActiveSurfaces(proc, side, bc), color, normal(proc), height)
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
    if (active.0, active.1, active.2) == (requested, side, bc) {
        return;
    }
    let (color, height) = color_height(requested, side, bc);
    let (color, height) = (images.add(color), images.add(height));
    let normal = (active.0 != requested).then(|| images.add(normal(requested)));
    for (_, material) in materials.iter_mut() {
        material.extension.terrain_color = color.clone();
        material.extension.terrain_height = height.clone();
        material.extension.parameters.y = side as f32;
        if let Some(normal) = &normal {
            material.extension.terrain_normal = normal.clone();
        }
    }
    *active = ActiveSurfaces(requested, side, bc);
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn embedded_material_arrays_have_complete_linear_normal_mips_with_bounded_allocation() {
        for procedural in [true, false] {
            check_arrays((color_height(procedural, 0, false).0, normal(procedural)));
        }
    }
    fn check_arrays((color, normal): (Image, Image)) {
        assert_eq!(color.texture_descriptor.size.depth_or_array_layers, 3);
        assert_eq!(color.texture_descriptor.mip_level_count, 10);
        assert_eq!(normal.texture_descriptor.format, TextureFormat::Rgba8Unorm);
        let a = color.data.unwrap();
        let b = normal.data.unwrap();
        assert!(a.len() + b.len() < 9 * 1024 * 1024);
        assert_eq!(a.len(), b.len());
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
