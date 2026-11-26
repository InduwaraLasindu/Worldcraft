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
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug)]
pub struct ActiveSurfaces(pub bool, pub u32, pub bool, pub u32);
impl ActiveSurfaces {
    fn requested(
        limits: &crate::performance::Performance,
        quality: crate::session::Quality,
        device: Option<&bevy::render::renderer::RenderDevice>,
    ) -> Self {
        use crate::session::Quality;
        let quality = limits.quality(quality);
        let (relief, bc) = crate::relief_texture::effective(limits, quality, device);
        let requested = limits
            .preferences
            .render
            .surface_resolution
            .side(quality == Quality::High, limits.software_gpu);
        let compact = quality == Quality::Low || limits.mode() == crate::performance::Mode::Eco;
        let cap = device.map_or(512, |dev| dev.limits().max_texture_dimension_2d);
        let base = if compact {
            512
        } else if crate::texture_pack::directory().is_some() {
            requested
                .min(if bc { 4096 } else { 1024 })
                .min(1 << cap.ilog2())
        } else if quality == Quality::High {
            SIDE
        } else {
            512
        };
        Self(
            limits.preferences.render.procedural_surfaces,
            relief,
            bc,
            base,
        )
    }
}
pub fn initial(
    limits: &crate::performance::Performance,
    quality: crate::session::Quality,
    device: Option<&bevy::render::renderer::RenderDevice>,
) -> (ActiveSurfaces, Image, Image, Image) {
    let target = ActiveSurfaces::requested(limits, quality, device);
    let proc = target.0;
    let (color, height) = color_height(proc, 0, target.2, 512);
    (
        ActiveSurfaces(proc, 0, target.2, 512),
        color,
        normal(proc, 512),
        height,
    )
}

const NATURE_MAPS: [&str; 11] = [
    "broadleaf",
    "birch",
    "acacia",
    "pine",
    "palm",
    "grass",
    "skyline",
    "bark",
    "bark-normal",
    "birch-bark",
    "birch-normal",
];

pub fn nature_map(name: &str, side: u32, bc: bool) -> std::io::Result<Image> {
    crate::texture_pack::load(
        &[name],
        side,
        !name.ends_with("-normal"),
        bc,
        name.contains("bark") || name == "birch-normal",
    )
}
struct Prepared {
    color: Image,
    normal: Image,
    height: Image,
    vegetation: Vec<Image>,
}
impl Prepared {
    fn load(key: ActiveSurfaces, resize_nature: bool) -> std::io::Result<Self> {
        let ActiveSurfaces(proc, side, bc, base) = key;
        let packed = crate::texture_pack::directory().is_some();
        let (color, height) = if !proc && packed && base > 512 {
            (
                crate::texture_pack::load(
                    &["scan-color-0", "scan-color-1", "scan-color-2"],
                    base,
                    true,
                    bc,
                    true,
                )?,
                crate::relief_texture::neutral(),
            )
        } else {
            color_height(proc, side, bc, base.min(SIDE))
        };
        let normal = if packed && base > 512 {
            let names = if proc {
                [
                    "proc-normal-0",
                    "proc-normal-1",
                    "proc-normal-2",
                    "sediment",
                ]
            } else {
                [
                    "scan-normal-0",
                    "scan-normal-1",
                    "scan-normal-2",
                    "sediment",
                ]
            };
            crate::texture_pack::load(&names, base, false, bc, true)?
        } else {
            normal(proc, base.min(SIDE))
        };
        let mut vegetation = Vec::new();
        if packed && resize_nature {
            for name in NATURE_MAPS {
                vegetation.push(nature_map(name, base, bc)?);
            }
        }
        Ok(Self {
            color,
            normal,
            height,
            vegetation,
        })
    }
}
#[derive(Default)]
pub struct Pending {
    job: Option<(ActiveSurfaces, bevy::tasks::Task<std::io::Result<Prepared>>)>,
    failed: Option<ActiveSurfaces>,
}
pub fn update(
    limits: Res<crate::performance::Performance>,
    graphics: Res<crate::session::Graphics>,
    device: Option<Res<bevy::render::renderer::RenderDevice>>,
    nature: Res<crate::assets::Nature>,
    mut active: ResMut<ActiveSurfaces>,
    mut pending: Local<Pending>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<crate::surface::SurfaceMaterial>>,
) {
    use bevy::tasks::{block_on, futures_lite::future, AsyncComputeTaskPool};
    let target = ActiveSurfaces::requested(&limits, graphics.quality, device.as_deref());
    if let Some((key, job)) = pending.job.as_mut() {
        let Some(result) = block_on(future::poll_once(job)) else {
            return;
        };
        let completed = *key;
        pending.job = None;
        if completed != target {
            return;
        }
        let prepared = match result {
            Ok(p) => p,
            Err(e) => {
                warn!("Cannot load world material pack: {e}");
                pending.failed = Some(target);
                return;
            }
        };
        let (color, normal, height) = (
            images.add(prepared.color),
            images.add(prepared.normal),
            images.add(prepared.height),
        );
        for (_, mat) in materials.iter_mut() {
            mat.extension.terrain_color = color.clone();
            mat.extension.terrain_normal = normal.clone();
            mat.extension.terrain_height = height.clone();
            mat.extension.parameters.y = target.1 as f32;
        }
        let slots = [
            (&nature.leaf, false),
            (&nature.birch_leaf, false),
            (&nature.acacia_leaf, false),
            (&nature.needles, false),
            (&nature.fronds, false),
            (&nature.ground, false),
            (&nature.skyline, false),
            (&nature.bark, false),
            (&nature.bark, true),
            (&nature.birch_bark, false),
            (&nature.birch_bark, true),
        ];
        for ((handle, is_normal), image) in slots.into_iter().zip(prepared.vegetation) {
            let mat = materials.get_mut(handle).expect("shared nature material");
            let target = if is_normal {
                &mat.base.normal_map_texture
            } else {
                &mat.base.base_color_texture
            };
            images
                .insert(target.as_ref().expect("nature image").id(), image)
                .expect("live nature image");
        }
        *active = target;
        pending.failed = None;
        return;
    }
    if *active == target || pending.failed == Some(target) {
        return;
    }
    pending.failed = None;
    let resize_nature = active.3 != target.3 || active.2 != target.2;
    pending.job = Some((
        target,
        AsyncComputeTaskPool::get().spawn(async move { Prepared::load(target, resize_nature) }),
    ));
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
