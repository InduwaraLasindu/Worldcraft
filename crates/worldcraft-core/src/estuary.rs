//! A seeded tidal inlet. The main channel, back channel and mudflats share the
//! sea datum, so the existing ocean meets the banks at every streaming level.
use crate::{geology::gradient, GeneratorSettings};

fn ramp(a: f64, b: f64, x: f64) -> f64 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
fn mix(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

// Distances in the channel coordinate system are metres at landscape scale 1.
fn sections(s: &GeneratorSettings, z: f64) -> (f64, f64, f64, f64) {
    let field = |salt, wavelength| gradient(s.seed ^ salt, z / wavelength, 0.37);
    let center =
        235.0 * field(0xd701, 1050.0) + 64.0 * field(0xd702, 330.0) + 13.0 * field(0xd703, 115.0);
    let mouth = 1.0 - ramp(-2700.0, 480.0, z);
    let width =
        (37.0 + 235.0 * mouth + 15.0 * field(0xd705, 540.0)) * (0.74 + 0.26 * s.river_strength);
    let t = ((z + 780.0) / 1890.0).clamp(0.0, 1.0);
    // Both ends meet the trunk with the same tangent. In between, an island
    // separates a smaller back channel from the main navigation channel.
    let split = (std::f64::consts::PI * t).sin().powi(2);
    let back = center + split * (265.0 + 55.0 * field(0xd707, 630.0));
    (center, width, back, split)
}

/// A point in the center of the inlet in world coordinates.
pub fn channel_x(s: &GeneratorSettings, world_z: f64) -> f64 {
    sections(s, world_z / s.terrain_scale).0 * s.terrain_scale
}

/// Only the estuary preset calls this. Ordinary landscapes skip all channel work.
pub(crate) fn shape(s: &GeneratorSettings, wx: f64, wz: f64, upland: f64) -> f64 {
    let x = wx / s.terrain_scale;
    let z = wz / s.terrain_scale;
    let coast = -2300.0 + 220.0 * gradient(s.seed ^ 0xd709, x / 1300.0, 0.71);
    let shore = z - coast;
    let vs = s.vertical_scale;
    let wrinkles = gradient(s.seed ^ 0xd70b, x / 92.0, z / 92.0);
    let coast_bed = (-2.7 + (shore * 0.029).max(-115.0) + wrinkles * 1.7) * vs;
    if shore < -1200.0 {
        return coast_bed;
    }
    let inland = ramp(2410.0, 3850.0, z);
    let upland = upland.max(6.0 * vs);
    if inland >= 1.0 {
        return upland;
    }
    let (center, width, back, split) = sections(s, z);
    let main_bank = (x - center).abs() - width;
    let side_width = width * (0.23 + 0.15 * split);
    let side_bank = (x - back).abs() - side_width;
    let bank = main_bank.min(side_bank);

    let occupied = ((x - center).abs() / width).min((x - back).abs() / side_width);
    let mud = (2.0 + 1.6 * wrinkles) * vs;
    let bank_height = if bank < 0.0 {
        (-1.2 - (1.0 - occupied.clamp(0.0, 1.0)).powf(0.8) * (5.0 + 0.05 * width)) * vs
    } else {
        mix(-1.2 * vs, mud, ramp(0.0, 21.0 + 8.0 * wrinkles, bank))
            + (bank - 18.0).max(0.0) * vs * 0.071
    };
    let valley = mix(bank_height, upland, ramp(45.0, 420.0, bank));
    let floodplain = mix(
        upland,
        valley,
        (1.0 - inland) * ramp(0.0, 0.35, s.river_strength),
    );

    mix(
        coast_bed.min(floodplain),
        floodplain,
        ramp(-15.0, 235.0, shore),
    )
}

pub(crate) fn raw(s: &GeneratorSettings, x: f64, z: f64) -> f64 {
    let u = x / s.terrain_scale;
    let v = z / s.terrain_scale;
    let ridge = (0.76 - gradient(s.seed ^ 0xd70d, u / 1550.0, v / 1550.0).abs()).max(0.0);
    let hills = gradient(s.seed ^ 0xd70e, u / 380.0, v / 380.0);
    let stone = gradient(s.seed ^ 0xd70f, u / 67.0, v / 67.0);
    shape(
        s,
        x,
        z,
        (62.0 + ridge * ridge * 285.0 * s.mountain_strength + 33.0 * hills + 5.0 * stone)
            * s.vertical_scale,
    )
}

/// Sample signed depth on an 8 m lattice around the observer tile. Border
/// vertices extend this same surface to the horizon without overlapping planes.
pub fn water_surface(
    g: &crate::Generator,
    coord: crate::ChunkCoord,
    extent: f32,
) -> Option<crate::MeshData> {
    const HALF: i32 = 24;
    const ROW: usize = 51;
    let [ox, oz] = coord.origin();
    let sea = g.settings().sea_level;
    let mut axis = Vec::with_capacity(ROW);
    axis.push(-extent);
    axis.extend((-HALF..=HALF).map(|i| 64.0 + (i * 8) as f32));
    axis.push(extent);
    let mut mesh = crate::MeshData::default();
    let mut min_depth = f64::INFINITY;
    let mut max_depth = f64::NEG_INFINITY;
    for (iz, &z) in axis.iter().enumerate() {
        for (ix, &x) in axis.iter().enumerate() {
            let edge = ix.min(iz).min(ROW - 1 - ix).min(ROW - 1 - iz);
            let (depth, weight) = if edge > 0 {
                let d = sea - g.height(ox + f64::from(x), oz + f64::from(z));
                min_depth = min_depth.min(d);
                max_depth = max_depth.max(d);
                (
                    d.clamp(-32.0, 32.0) as f32,
                    ((edge - 1) as f32 / 3.0).min(1.0),
                )
            } else {
                (0.0, 0.0)
            };
            mesh.vertex([x, 0.0, z], [0.0, 1.0, 0.0], [1.0; 4], [depth, weight]);
        }
    }
    if min_depth > 10.0 || max_depth < 0.0 {
        return None;
    }
    mesh.indices.reserve((ROW - 1) * (ROW - 1) * 6);
    for z in 0..ROW - 1 {
        for x in 0..ROW - 1 {
            let a = (z * ROW + x) as u32;
            let b = a + 1;
            let c = a + ROW as u32;
            mesh.indices.extend([a, c, b, b, c, c + 1]);
        }
    }
    Some(mesh)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Generator, Preset};

    #[test]
    fn eroded_main_and_back_channels_connect_to_the_open_sea_across_tile_edges() {
        for (seed, scale) in [(7, 0.6), (42, 1.0), (281, 2.5)] {
            let mut s = GeneratorSettings::for_preset(Preset::Estuary, seed);
            s.terrain_scale = scale;
            let g = Generator::new(s.clone(), vec![]);
            // Follow each wet centerline through both ends of the island and out to sea.
            for z in (-3100..=1300).step_by(32) {
                let (main, _, back, _) = sections(&s, f64::from(z));
                for x in [main, back] {
                    assert!(g.height(x * scale, f64::from(z) * scale) < s.sea_level - 0.45);
                }
            }
            let (main, _, back, _) = sections(&s, 165.0);
            assert!(g.height((main + back) * 0.5 * scale, 165.0 * scale) > s.sea_level + 1.0);
        }
    }

    #[test]
    fn water_surface_matches_edits_and_adjacent_observer_tiles() {
        use crate::{Brush, ChunkCoord, Stroke};
        let s = GeneratorSettings::for_preset(Preset::Estuary, 42);
        let g = Generator::new(s.clone(), vec![]);
        let c = ChunkCoord::at(channel_x(&s, 0.0), 0.0);
        let [ox, oz] = c.origin();
        let a = water_surface(&g, c, 16384.0).unwrap();
        let next = water_surface(&g, ChunkCoord { x: c.x + 1, z: c.z }, 16384.0).unwrap();
        assert_eq!((a.positions.len(), a.indices.len() / 3), (2601, 5000));
        assert!(a
            .uv
            .iter()
            .all(|u| (-32.0..=32.0).contains(&u[0]) && (0.0..=1.0).contains(&u[1])));
        // The inner grid shifts by exactly 16 steps. World-space depths on
        // the overlap must be bit-identical after an anchor change.
        for z in 4..47 {
            for x in 20..47 {
                assert_eq!(a.uv[z * 51 + x][0], next.uv[z * 51 + x - 16][0]);
            }
        }
        let i = 25 * 51 + 25;
        let x = ox + f64::from(a.positions[i][0]);
        let z = oz + f64::from(a.positions[i][2]);
        let edited = g
            .with_edits(vec![Stroke {
                center: [x, z],
                brush: Brush {
                    strength: 5.0,
                    ..Default::default()
                },
            }])
            .unwrap();
        let refreshed = water_surface(&edited, c, 16384.0).unwrap();
        assert!(
            (refreshed.uv[i][0] - ((s.sea_level - edited.height(x, z)).clamp(-32.0, 32.0) as f32))
                .abs()
                < 1e-5
        );
        assert!((a.uv[i][0] - refreshed.uv[i][0]).abs() > 1.0);
        assert!(a.uv[..51].iter().all(|v| v[1] == 0.0));
    }
    #[test]
    fn ordinary_new_world_and_reopening_keep_the_same_dry_bank() {
        let s = GeneratorSettings::for_preset(Preset::Estuary, 17);
        let g = Generator::new(s.clone(), vec![]);
        let mut w = crate::WorldFile::new("Inlet".into(), s);
        w.camera = g.spawn_position();
        assert!(w.camera[1] > w.settings.sea_level + 5.0);
        let reopened: crate::WorldFile =
            serde_json::from_slice(&serde_json::to_vec(&w).unwrap()).unwrap();
        assert_eq!(w, reopened);
        assert_eq!(
            crate::Generator::for_world(&reopened)
                .unwrap()
                .spawn_position(),
            w.camera
        );
    }
}
