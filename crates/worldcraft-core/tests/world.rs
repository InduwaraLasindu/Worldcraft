use std::path::PathBuf;
use worldcraft_core::*;
use worldcraft_core::{
    config::seed_from_text,
    generation::noise,
    scatter::{scatter, Species},
    streaming::*,
};
fn gen() -> Generator {
    Generator::new(GeneratorSettings::default(), Vec::new())
}
fn key(x: i64, z: i64, lod: u8, edges: [u8; 4]) -> ChunkKey {
    ChunkKey {
        coord: ChunkCoord { x, z },
        lod,
        edges,
        revision: 0,
    }
}
fn temporary(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("worldcraft-{}-{name}.json", std::process::id()))
}
#[test]
fn deterministic_in_any_order() {
    let a = gen();
    let points = [(-4096.3, 17.2), (0.0, 0.0), (1000.5, -256.8), (-1.0, -1.0)];
    let expected: Vec<_> = points.iter().map(|&(x, z)| a.sample(x, z)).collect();
    for (i, &(x, z)) in points.iter().enumerate().rev() {
        assert_eq!(expected[i], gen().sample(x, z));
    }
}
#[test]
fn seeds_change_geography() {
    let a = gen();
    let mut settings = GeneratorSettings::default();
    settings.seed += 1;
    let b = Generator::new(settings, Vec::new());
    assert_ne!(a.height(432.0, -1123.0), b.height(432.0, -1123.0));
}
#[test]
fn text_seeds_are_stable() {
    assert_eq!(seed_from_text("42"), 42);
    assert_eq!(seed_from_text("Sri Lanka"), seed_from_text("Sri Lanka"));
    assert_ne!(seed_from_text("Sri Lanka"), seed_from_text("Worldcraft"));
}
#[test]
fn negative_coordinates_use_floor() {
    assert_eq!(ChunkCoord::at(-0.1, -128.1), ChunkCoord { x: -1, z: -2 });
    assert_eq!(ChunkCoord::at(-128.0, 128.0), ChunkCoord { x: -1, z: 1 });
}
#[test]
fn noise_is_continuous_at_negative_grid_lines() {
    for i in -10..=10 {
        let x = f64::from(i);
        assert!((noise(7, x - 1e-7, -3.21) - noise(7, x + 1e-7, -3.21)).abs() < 1e-5);
    }
}
#[test]
fn neighboring_meshes_share_exact_boundary() {
    let a = build_mesh(&gen(), key(-1, -2, 0, [0; 4]));
    let b = build_mesh(&gen(), key(0, -2, 0, [0; 4]));
    for z in 0..=32 {
        assert_eq!(a.positions[z * 33 + 32][1], b.positions[z * 33][1]);
        assert_eq!(a.normals[z * 33 + 32], b.normals[z * 33]);
    }
}
#[test]
fn fine_edge_matches_coarse_polyline() {
    let a = build_mesh(&gen(), key(-1, 0, 0, [0, 1, 0, 0]));
    let b = build_mesh(&gen(), key(0, 0, 1, [1; 4]));
    for z in 0..=32 {
        let h = if z % 2 == 0 {
            b.positions[(z / 2) * 17][1]
        } else {
            (b.positions[(z / 2) * 17][1] + b.positions[(z / 2 + 1) * 17][1]) * 0.5
        };
        assert!((a.positions[z * 33 + 32][1] - h).abs() < 0.0001);
    }
}
#[test]
fn mesh_indices_and_winding_are_valid() {
    for lod in 0..=3 {
        let m = build_mesh(&gen(), key(1, -1, lod, [lod; 4]));
        assert_eq!(m.positions.len(), m.normals.len());
        assert_eq!(m.positions.len(), m.colors.len());
        assert!(m.indices.iter().all(|i| (*i as usize) < m.positions.len()));
        assert!(m.positions.iter().flatten().all(|v| v.is_finite()));
        let p = |n: usize| m.positions[m.indices[n] as usize];
        let (a, b, c) = (p(0), p(1), p(2));
        let ny = (b[2] - a[2]) * (c[0] - a[0]) - (b[0] - a[0]) * (c[2] - a[2]);
        assert!(ny > 0.0);
    }
}
#[test]
fn normals_have_unit_length() {
    let g = gen();
    for x in -8..8 {
        for z in -8..8 {
            let n = g.normal(f64::from(x) * 73.0, f64::from(z) * 71.0);
            assert!((n.iter().map(|c| c * c).sum::<f32>() - 1.0).abs() < 1e-5);
            assert!(n[1] > 0.0);
        }
    }
}
#[test]
fn streaming_is_bounded_and_near_first() {
    let center = ChunkCoord { x: -2, z: 5 };
    let coords = wanted(center, 4);
    assert_eq!(coords.len(), 81);
    assert_eq!(coords[0], center);
    assert!(coords.iter().all(|c| c.distance(center) <= 4));
    for c in coords {
        for n in [
            ChunkCoord { x: c.x + 1, ..c },
            ChunkCoord { z: c.z + 1, ..c },
        ] {
            assert!(lod_for(c, center).abs_diff(lod_for(n, center)) <= 1);
        }
    }
}
#[test]
fn all_presets_produce_finite_distinct_landscapes() {
    let mut heights = Vec::new();
    for preset in Preset::ALL {
        let settings = GeneratorSettings::for_preset(preset, 1234);
        settings.validate().unwrap();
        let g = Generator::new(settings, Vec::new());
        heights.push(g.height(211.0, -398.0).to_bits());
        for x in -2..=2 {
            assert!(g.sample(f64::from(x) * 1000.0, 79.0).height.is_finite());
        }
    }
    heights.sort();
    heights.dedup();
    assert!(heights.len() >= 4);
}
#[test]
fn scatter_is_repeatable_and_grounded() {
    let g = gen();
    let c = ChunkCoord::at(0.0, 0.0);
    let a = scatter(&g, c, 0);
    assert_eq!(a, scatter(&gen(), c, 0));
    for i in a {
        let [x, y, z] = i.position;
        assert!((y - g.height(x, z)).abs() < 1e-10);
        assert!(y > g.settings().sea_level);
        assert_eq!(ChunkCoord::at(x, z), c);
    }
}
#[test]
fn no_tree_or_rock_appears_underwater() {
    let mut s = GeneratorSettings::default();
    s.sea_level = 1000.0;
    assert!(scatter(&Generator::new(s, Vec::new()), ChunkCoord::default(), 0).is_empty());
}
#[test]
fn vegetation_edits_control_scatter() {
    let brush = Brush {
        kind: BrushKind::Vegetation,
        radius: 512.0,
        density: 0.0,
        ..Brush::default()
    };
    let g = Generator::new(
        GeneratorSettings::default(),
        vec![Stroke {
            center: [64.0, 64.0],
            brush,
        }],
    );
    let no_trees = Generator::new(
        GeneratorSettings {
            vegetation: 0.0,
            ..g.settings().clone()
        },
        vec![],
    );
    assert!(scatter(&no_trees, ChunkCoord::default(), 0)
        .iter()
        .all(|i| i.species == Species::Rock));
    assert!(g.sample(64.0, 64.0).vegetation < 1e-6);
}
#[test]
fn brush_transactions_undo_redo_exactly() {
    let original = gen().height(0.0, 0.0);
    let stroke = Stroke {
        center: [0.0, 0.0],
        brush: Brush::default(),
    };
    let mut h = EditHistory::default();
    h.push(vec![stroke.clone(), stroke]).unwrap();
    assert_eq!(
        Generator::new(GeneratorSettings::default(), h.strokes()).height(0.0, 0.0),
        original + 10.0
    );
    assert!(h.undo());
    assert!(h.strokes().is_empty());
    assert!(h.redo());
    assert_eq!(h.strokes().len(), 2);
    h.undo();
    h.push(vec![Stroke {
        center: [10.0, 0.0],
        brush: Brush::default(),
    }])
    .unwrap();
    assert!(!h.redo());
}
#[test]
fn brush_falloff_and_outside_are_correct() {
    let e = Stroke {
        center: [-10.0, 3.0],
        brush: Brush::default(),
    };
    assert_eq!(e.weight(-10.0, 3.0), 1.0);
    assert_eq!(e.weight(1000.0, 3.0), 0.0);
    assert!(e.weight(0.0, 3.0) > 0.0);
}
#[test]
fn sculpting_is_seamless_across_chunks() {
    let g = Generator::new(
        GeneratorSettings::default(),
        vec![Stroke {
            center: [0.0, -128.0],
            brush: Brush::default(),
        }],
    );
    let a = build_mesh(&g, key(-1, -1, 0, [0; 4]));
    let b = build_mesh(&g, key(0, -1, 0, [0; 4]));
    for z in 0..=32 {
        assert_eq!(a.positions[z * 33 + 32][1], b.positions[z * 33][1]);
    }
}
#[test]
fn settings_reject_nan_and_unbounded_values() {
    let mut s = GeneratorSettings::default();
    s.vegetation = f64::NAN;
    assert!(s.validate().is_err());
    s.vegetation = 50.0;
    assert!(s.validate().is_err());
}
#[test]
fn corrupt_and_newer_files_are_rejected() {
    let mut w = WorldFile::new("Test".into(), GeneratorSettings::default());
    w.generator_version = 999;
    assert!(w.validate().is_err());
    w.generator_version = GENERATOR_VERSION;
    w.settings.seed += 1;
    assert!(w.validate().is_err());
}
#[test]
fn save_load_preserves_edits_and_camera() {
    let path = temporary("roundtrip");
    let _ = std::fs::remove_file(&path);
    let mut w = WorldFile::new("River valley".into(), GeneratorSettings::default());
    w.edits.push(vec![Stroke {
        center: [-128.0, 0.0],
        brush: Brush::default(),
    }]);
    w.camera = [-123456.5, 100.0, 98765.25];
    save_world(&path, &w).unwrap();
    assert_eq!(load_world(&path).unwrap(), w);
    w.name = "New name".into();
    save_world(&path, &w).unwrap();
    assert_eq!(
        load_world(&path.with_extension("json.bak")).unwrap().name,
        "River valley"
    );
    std::fs::remove_file(path.with_extension("json.bak")).unwrap();
    std::fs::remove_file(path).unwrap();
}
#[test]
fn invalid_save_does_not_replace_good_world() {
    let path = temporary("invalid");
    let w = WorldFile::new("Valid".into(), GeneratorSettings::default());
    save_world(&path, &w).unwrap();
    let mut bad = w.clone();
    bad.camera[0] = f64::INFINITY;
    assert!(save_world(&path, &bad).is_err());
    assert_eq!(load_world(&path).unwrap(), w);
    std::fs::remove_file(path).unwrap();
}
#[test]
fn schema_roundtrip_is_stable() {
    let w = WorldFile::new("Test".into(), GeneratorSettings::default());
    let a = serde_json::to_string(&w).unwrap();
    let b: WorldFile = serde_json::from_str(&a).unwrap();
    b.validate().unwrap();
    assert_eq!(a, serde_json::to_string(&b).unwrap());
}

#[test]
fn legacy_v1_golden_geography_is_preserved() {
    let g = Generator::for_version(GeneratorSettings::default(), vec![], 1).unwrap();
    let points: [(f64, f64, f64); 5] = [
        (-4096.3, 17.2, 143.41808508495586),
        (0.0, 0.0, 426.0055319303241),
        (1000.5, -256.8, 140.0512546594648),
        (-1.0, -1.0, 425.9491985168787),
        (12345.25, 9345.5, -29.11969701355249),
    ];
    for (x, z, height) in points {
        assert!(
            (g.height(x, z) - height).abs() < 1e-9,
            "legacy terrain changed at {x},{z}"
        );
    }
    assert_ne!(g.height(0.0, 0.0), gen().height(0.0, 0.0));
}
#[test]
fn legacy_save_roundtrip_keeps_its_generator_and_edits() {
    let path = temporary("legacy-v1");
    let mut world = WorldFile::new("Legacy world".into(), GeneratorSettings::default());
    world.generator_version = 1;
    world.edits = vec![vec![Stroke {
        center: [0.0, 0.0],
        brush: Brush::default(),
    }]];
    save_world(&path, &world).unwrap();
    let loaded = load_world(&path).unwrap();
    assert_eq!(loaded.generator_version, 1);
    let g = Generator::for_version(
        loaded.settings.clone(),
        loaded.edits.into_iter().flatten().collect(),
        loaded.generator_version,
    )
    .unwrap();
    assert!((g.height(0.0, 0.0) - 431.0055319303241).abs() < 1e-9);
    std::fs::remove_file(&path).unwrap();
}
#[test]
fn edit_index_matches_ordered_linear_replay() {
    let settings = GeneratorSettings::for_preset(Preset::Empty, 45);
    let mut edits = Vec::new();
    for i in 0..90 {
        edits.push(Stroke {
            center: [
                ((i * 97) % 700) as f64 - 350.0,
                ((i * 179) % 700) as f64 - 350.0,
            ],
            brush: Brush {
                kind: BrushKind::ALL[i % 7],
                radius: 60.0 + (i % 5) as f64 * 70.0,
                strength: 3.0 + (i % 9) as f64,
                target_height: i as f64 - 40.0,
                density: (i % 20) as f64 / 10.0,
                biome: Biome::ALL[i % 10],
            },
        });
    }
    let g = Generator::new(settings, edits.clone());
    for z in -9..=9 {
        for x in -9..=9 {
            let (x, z) = (x as f64 * 64.0, z as f64 * 64.0);
            let mut height = g.base_height(x, z);
            let mut vegetation = g.settings().vegetation;
            for e in &edits {
                let w = e.weight(x, z);
                if w > 0.0 {
                    height = e.height(height, w);
                }
                if e.brush.kind == BrushKind::Vegetation {
                    vegetation += (e.brush.density - vegetation) * w;
                }
            }
            assert_eq!(g.height(x, z), height);
            assert_eq!(g.sample(x, z).vegetation, vegetation);
            let painted = edits
                .iter()
                .rev()
                .find(|e| e.brush.kind == BrushKind::Biome && e.weight(x, z) > 0.4);
            if let Some(e) = painted {
                assert_eq!(g.sample(x, z).biome, e.brush.biome);
            }
        }
    }
}
#[test]
fn edit_snapshots_reuse_the_terrain_cache() {
    let g = gen();
    let base = g.height(17.5, -19.0);
    let before = g.cache_stats().bakes;
    let e = Stroke {
        center: [17.5, -19.0],
        brush: Brush::default(),
    };
    let edited = g.with_edits(vec![e]).unwrap();
    assert_eq!(edited.height(17.5, -19.0), base + 5.0);
    assert_eq!(edited.version(), g.version());
    assert_eq!(edited.cache_stats().bakes, before);
    assert_eq!(g.cache_stats().bakes, before);
}
#[test]
fn invalid_edits_are_rejected_before_indexing() {
    let mut e = Stroke {
        center: [0.0, 0.0],
        brush: Brush::default(),
    };
    e.brush.radius = f64::INFINITY;
    assert!(Generator::for_version(GeneratorSettings::default(), vec![e], 2).is_err());
    assert!(Generator::for_version(GeneratorSettings::default(), vec![], 0).is_err());
}

#[test]
fn regional_preview_applies_edits_without_filling_the_exploration_cache(){
    let edits=vec![
        Stroke{center:[0.0,0.0],brush:Brush{strength:12.0,..Brush::default()}},
        Stroke{center:[0.0,0.0],brush:Brush{kind:BrushKind::Biome,biome:Biome::Desert,..Brush::default()}},
        Stroke{center:[0.0,0.0],brush:Brush{kind:BrushKind::Vegetation,density:0.15,..Brush::default()}},
    ];
    let g=Generator::new(GeneratorSettings::default(),edits);
    let raw=geology::raw_height(g.settings(),0.0,0.0);
    let preview=g.preview_sample(0.0,0.0);
    assert_eq!(preview.height,raw+12.0);assert_eq!(preview.biome,Biome::Desert);
    assert!((preview.vegetation-0.15).abs()<1e-12);
    for i in -100..100 {g.preview_sample(i as f64*512.0,47000.0);}
    assert_eq!(g.cache_stats().bakes,0);
    let legacy=Generator::for_version(GeneratorSettings::default(),g.edits().to_vec(),1).unwrap();
    assert_eq!(legacy.preview_sample(91.3,-14.5),legacy.sample(91.3,-14.5));
}
