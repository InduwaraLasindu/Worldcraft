//! Optional Terrain3D 1.1 adapter. Godot objects stay on the main thread.
use godot::{
    classes::{ClassDb, Node3D, Object},
    prelude::*,
};

const REGION_RANGE: std::ops::Range<i32> = -16..16;

pub(crate) fn encode_control(
    base: u8,
    overlay: u8,
    blend: u8,
    angle: u8,
    scale: u8,
    hole: bool,
    navigation: bool,
    automatic: bool,
) -> u32 {
    (u32::from(base & 31) << 27)
        | (u32::from(overlay & 31) << 22)
        | (u32::from(blend) << 14)
        | (u32::from(angle & 15) << 10)
        | (u32::from(scale & 7) << 7)
        | (u32::from(hole) << 2)
        | (u32::from(navigation) << 1)
        | u32::from(automatic)
}

#[derive(GodotClass)]
#[class(base=RefCounted, init)]
struct WorldcraftTerrain3D {}

#[godot_api]
impl WorldcraftTerrain3D {
    #[func]
    fn available(&self) -> bool {
        ClassDb::singleton().class_exists("Terrain3D")
    }

    /// The returned signed integer carries the original 32-bit control word.
    #[func]
    fn pack_control(
        &self,
        base: u8,
        overlay: u8,
        blend: u8,
        angle: u8,
        scale: u8,
        hole: bool,
        navigation: bool,
        automatic: bool,
    ) -> i32 {
        encode_control(
            base, overlay, blend, angle, scale, hole, navigation, automatic,
        ) as i32
    }

    /// Uploads one region at an integer Terrain3D location. Empty string means success.
    #[func]
    fn upload_region(
        &self,
        mut terrain: Gd<Node3D>,
        location: Vector2i,
        heights: PackedFloat32Array,
        control: PackedInt32Array,
        colors: PackedByteArray,
    ) -> GString {
        if !terrain.is_class("Terrain3D") || !terrain.is_inside_tree() {
            return "Terrain3D must be initialized in the scene tree".into();
        }
        let version = terrain.call("get_version", &[]).to::<GString>();
        if version != GString::from("1.1.0") {
            return GString::from(&format!("Terrain3D 1.1.0 is required. Loaded {version}"));
        }
        let side = terrain.call("get_region_size", &[]).to::<i32>();
        if !matches!(side, 64 | 128 | 256 | 512 | 1024 | 2048)
            || !REGION_RANGE.contains(&location.x)
            || !REGION_RANGE.contains(&location.y)
        {
            return "Invalid Terrain3D region size or location".into();
        }
        let pixels = (side * side) as usize;
        if heights.len() != pixels
            || control.len() != pixels
            || colors.len() != pixels * 4
            || heights.as_slice().iter().any(|height| !height.is_finite())
        {
            return "Terrain3D region needs finite RF heights, RF control words, and RGBA8 colors of the configured size".into();
        }
        let Ok(mut data) = terrain.call("get_data", &[]).try_to::<Gd<Object>>() else {
            return "Terrain3D data is unavailable".into();
        };
        let spacing = terrain.call("get_vertex_spacing", &[]).to::<f32>();
        // Terrain3D floors the position onto its vertex grid. Sample just inside
        // the first cell so a fractional vertex spacing cannot round it outside.
        let origin = Vector3::new(
            ((location.x * side) as f32 + 0.25) * spacing,
            0.0,
            ((location.y * side) as f32 + 0.25) * spacing,
        );

        let mut classes = ClassDb::singleton();
        let rf = classes.class_get_integer_constant("Image", "FORMAT_RF");
        let rgba8 = classes.class_get_integer_constant("Image", "FORMAT_RGBA8");
        let height_bytes: Vec<u8> = heights
            .as_slice()
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        // Keep the original bits, including words that would be NaNs as float values.
        let control_bytes: Vec<u8> = control
            .as_slice()
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        let payloads = [
            (rf, PackedByteArray::from(height_bytes.as_slice())),
            (rf, PackedByteArray::from(control_bytes.as_slice())),
            (rgba8, colors),
        ];
        let Ok(mut region) = classes
            .instantiate("Terrain3DRegion")
            .try_to::<Gd<Object>>()
        else {
            return "Could not allocate a Terrain3D region".into();
        };
        // The plugin's get_maps supplies a Godot Array[Image] without requiring
        // the typed Image binding from gdext's full engine API.
        let maps = region.call("get_maps", &[]);
        for (index, (format, bytes)) in payloads.into_iter().enumerate() {
            let image = classes.class_call_static(
                "Image",
                "create_from_data",
                &[
                    side.to_variant(),
                    side.to_variant(),
                    false.to_variant(),
                    format.to_variant(),
                    bytes.to_variant(),
                ],
            );
            if image.is_nil() {
                return "Could not create Terrain3D map image".into();
            }
            maps.call("set", &[(index as i64).to_variant(), image]);
        }
        let args = [
            maps,
            origin.to_variant(),
            0.0_f32.to_variant(),
            1.0_f32.to_variant(),
        ];
        if let Err(error) = data.try_call("import_images", &args) {
            return GString::from(&format!("Terrain3D import_images failed: {error}"));
        }
        if !data
            .call("has_region", &[location.to_variant()])
            .to::<bool>()
        {
            return "Terrain3D did not activate the imported region".into();
        }
        GString::new()
    }
}

#[cfg(test)]
mod tests {
    use super::encode_control;

    #[test]
    fn terrain3d_control_word_matches_upstream() {
        assert_eq!(encode_control(0, 0, 0, 0, 0, false, false, false), 0);
        assert_eq!(encode_control(1, 0, 0, 0, 0, false, false, false), 1 << 27);
        assert_eq!(encode_control(0, 1, 0, 0, 0, false, false, false), 1 << 22);
        assert_eq!(encode_control(0, 0, 1, 0, 0, false, false, false), 1 << 14);
        assert_eq!(encode_control(0, 0, 0, 1, 0, false, false, false), 1 << 10);
        assert_eq!(encode_control(0, 0, 0, 0, 1, false, false, false), 1 << 7);
        assert_eq!(encode_control(0, 0, 0, 0, 0, true, false, false), 1 << 2);
        assert_eq!(encode_control(0, 0, 0, 0, 0, false, true, false), 1 << 1);
        assert_eq!(encode_control(0, 0, 0, 0, 0, false, false, true), 1);
        assert_eq!(
            encode_control(255, 255, 255, 255, 255, true, true, true),
            0xffff_ff87
        );
    }
}
