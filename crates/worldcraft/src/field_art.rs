//! Original photographic-style terrain maps and cutout foliage.
use bevy::prelude::Image;

macro_rules! art {
    ($name:literal) => {
        include_bytes!(concat!("../art/", $name, ".wct"))
    };
}

pub fn terrain(bc: bool) -> (Image, Image) {
    let side = if bc { 512 } else { 256 };
    (
        crate::texture_pack::bundled(
            &[
                art!("litter-color"),
                art!("granite-color"),
                art!("meadow-color"),
                art!("sand-color"),
                art!("snow-color"),
                art!("gravel-color"),
                art!("basalt-color"),
                art!("soil-color"),
            ],
            side,
            true,
            bc,
            true,
        ),
        crate::texture_pack::bundled(
            &[
                art!("litter-surface"),
                art!("granite-surface"),
                art!("meadow-surface"),
                art!("sand-surface"),
                art!("snow-surface"),
                art!("gravel-surface"),
                art!("basalt-surface"),
                art!("soil-surface"),
            ],
            side,
            false,
            bc,
            true,
        ),
    )
}

pub fn nature(name: &str, side: u32, bc: bool) -> Option<Image> {
    let data: &[u8] = match name {
        "broadleaf" => art!("oak-spray"),
        "pine" => art!("spruce-spray"),
        "birch" => art!("birch-spray"),
        "grass" => art!("grass-tufts"),
        "palm" => art!("palm-frond"),
        "acacia" => art!("acacia-spray"),
        "bark" => art!("oak-bark"),
        "bark-normal" => art!("oak-bark-normal"),
        "birch-bark" => art!("birch-bark"),
        "birch-normal" => art!("birch-bark-normal"),
        _ => return None,
    };
    Some(crate::texture_pack::bundled(
        &[data],
        side,
        !name.ends_with("-normal"),
        bc,
        name.contains("bark") || name == "birch-normal",
    ))
}
