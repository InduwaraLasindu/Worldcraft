use worldcraft_core::{scatter::scatter, ChunkCoord, Generator, GeneratorSettings, Preset};

// Micrometre quantization tolerates libm's last-bit platform variation while
// protecting the durable cell identities, positions, species and transforms.
fn placement_digest(version: u32, seed: u64, coord: ChunkCoord) -> (usize, u64) {
    let mut settings = GeneratorSettings::for_preset(Preset::Temperate, seed);
    settings.sea_level = -50.0;
    settings.temperature = 0.70;
    settings.rainfall = 0.75;
    let generator = Generator::for_version(settings, vec![], version).unwrap();
    let instances = scatter(&generator, coord, 0);
    let mut hash = 0xcbf29ce484222325_u64;
    for instance in &instances {
        let values = [
            instance.position[0],
            instance.position[1],
            instance.position[2],
            instance.scale as f64,
            instance.yaw as f64,
            instance.species as u8 as f64,
            instance.variant as f64,
        ];
        for value in values {
            for byte in ((value * 1_000_000.0).round() as i64).to_le_bytes() {
                hash = (hash ^ byte as u64).wrapping_mul(0x100000001b3);
            }
        }
    }
    (instances.len(), hash)
}

#[test]
fn legacy_placement_snapshots() {
    for (version, seed, coord, expected) in [
        (1, 42, ChunkCoord { x: 0, z: 0 }, (297, 4407458026946473776)),
        (
            1,
            20261118,
            ChunkCoord { x: -3, z: 2 },
            (327, 8884498289828232866),
        ),
        (
            2,
            42,
            ChunkCoord { x: 0, z: 0 },
            (731, 16701453523898949501),
        ),
        (
            2,
            20261118,
            ChunkCoord { x: -3, z: 2 },
            (769, 4938313177942329821),
        ),
    ] {
        assert_eq!(placement_digest(version, seed, coord), expected);
    }
}

#[test]
fn ecological_generation_preserves_the_previous_relief() {
    for preset in [
        Preset::Temperate,
        Preset::Alpine,
        Preset::Tropical,
        Preset::Volcanic,
    ] {
        let settings = GeneratorSettings::for_preset(preset, 20261118);
        let old = Generator::for_version(settings.clone(), vec![], 2).unwrap();
        let new = Generator::for_version(settings, vec![], 3).unwrap();
        for (x, z) in [(0.0, 0.0), (-512.25, 128.5), (1200.0, -1400.0)] {
            assert_eq!(old.height(x, z), new.height(x, z));
        }
    }
}

#[test]
fn biome_paint_controls_species_and_understory_without_unbounded_scatter() {
    use worldcraft_core::{scatter::Species, Biome, Brush, BrushKind, Stroke};
    let mut settings = GeneratorSettings::for_preset(Preset::Temperate, 42);
    settings.sea_level = -50.0;
    let base = Generator::for_version(settings, vec![], 3).unwrap();
    for (biome, tree, plant) in [
        (Biome::BirchForest, Species::Birch, Species::Flower),
        (Biome::Savanna, Species::Acacia, Species::Grass),
        (Biome::Wetland, Species::Broadleaf, Species::Reed),
        (Biome::Forest, Species::Broadleaf, Species::Fern),
    ] {
        let g = base
            .with_edits(vec![Stroke {
                center: [64.0, 64.0],
                brush: Brush {
                    kind: BrushKind::Biome,
                    biome,
                    radius: 512.0,
                    ..Brush::default()
                },
            }])
            .unwrap();
        let coord = ChunkCoord { x: 0, z: 0 };
        let instances = scatter(&g, coord, 0);
        assert!(instances.len() <= 4352);
        assert!(
            instances.iter().any(|i| i.species == tree),
            "Missing biome tree: {biome:?}"
        );
        assert!(
            instances.iter().any(|i| i.species == plant),
            "Missing understory: {biome:?}"
        );
        assert_eq!(instances, scatter(&g, coord, 0));
        assert!(scatter(&g, coord, 1).iter().all(|i| !i.species.is_ground()));
    }
}

#[test]
fn one_mixed_world_has_distinct_connected_climate_regions() {
    let g = Generator::new(GeneratorSettings::for_preset(Preset::Temperate, 42), vec![]);
    let mut histogram = [0_usize; 14];
    for z in -25..=25 {
        for x in -25..=25 {
            let s = g.preview_sample(x as f64 * 800.0, z as f64 * 800.0);
            histogram[s.biome as usize] += 1;
        }
    }
    assert!(
        histogram.iter().filter(|&&n| n > 0).count() >= 6,
        "Insufficient biome variety: {histogram:?}"
    );
}
