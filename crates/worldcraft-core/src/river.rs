//! A finite, seeded river valley with a shared head profile at its confluence.
use crate::{geology::gradient, GeneratorSettings, MeshData};

const MOUTH: f64 = -960.0;
const SOURCE: f64 = 1024.0;
const JOIN: f64 = 0.0;
const FALL_FOOT: f64 = 128.0;
const FALL_CREST: f64 = 136.0;

fn smooth(a: f64, b: f64, x: f64) -> f64 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn head(s: &GeneratorSettings, z: f64) -> f64 {
    grade(s, z) + 18.0 * smooth(FALL_FOOT, FALL_CREST, z) * s.vertical_scale
}

fn grade(s: &GeneratorSettings, z: f64) -> f64 {
    s.sea_level + (z - MOUTH).max(0.0) * 0.034 * s.vertical_scale
}

fn arm_head(s: &GeneratorSettings, z: f64, gap: f64) -> f64 {
    let gradual = grade(s, z) + 18.0 * smooth(140.0, 460.0, z) * s.vertical_scale;
    let shared = head(s, z);
    shared + (gradual - shared) * smooth(0.0, 14.0, gap / s.terrain_scale)
}

fn section(s: &GeneratorSettings, z: f64) -> (f64, f64, f64, f64) {
    let z = z / s.terrain_scale;
    let trunk = 66.0 * gradient(s.seed ^ 0x7269, z / 300.0, 0.43)
        + 19.0 * gradient(s.seed ^ 0x7665, z / 107.0, 0.73);
    let tributary = trunk + z.max(JOIN) * (0.46 + 0.10 * gradient(s.seed ^ 0x72, z / 210.0, 0.7));
    let width = (19.0 + 28.0 * (1.0 - smooth(MOUTH, -700.0, z))) * (0.8 + 0.2 * s.river_strength);
    let main_width = width * (1.0 - smooth(856.0, SOURCE, z));
    let arm_width = width * 0.58 * smooth(JOIN, 104.0, z) * (1.0 - smooth(704.0, 912.0, z));
    (
        trunk * s.terrain_scale,
        main_width * s.terrain_scale,
        tributary * s.terrain_scale,
        arm_width * s.terrain_scale,
    )
}

pub(crate) fn carve(s: &GeneratorSettings, x: f64, z: f64, terrain: f64) -> f64 {
    let along = z / s.terrain_scale;
    if !(-1152.0..SOURCE).contains(&along) || x.abs() > 1152.0 * s.terrain_scale {
        return terrain;
    }
    let (main, half, arm, arm_half) = section(s, z);
    let trunk_bank = (x - main).abs() - half;
    let arm_bank = (x - arm).abs() - arm_half;
    let bank = if arm_half > 0.0 {
        trunk_bank.min(arm_bank)
    } else {
        trunk_bank
    };
    let bank = bank / s.terrain_scale;
    let grade = grade(s, along);
    let main_head = head(s, along);
    let mut local_head = main_head;
    if arm_half > 0.0 {
        let tributary = arm_head(s, along, arm - main - arm_half - half);
        local_head += (tributary - main_head)
            * smooth(-14.0, 14.0, (trunk_bank - arm_bank) / s.terrain_scale);
    }
    let water = grade + (local_head - grade) * (1.0 - smooth(8.0, 66.0, bank));
    let depth = (1.4 + 1.8 * (1.0 - smooth(MOUTH, -620.0, along))) * s.vertical_scale;
    // Extend the bottom beyond the water edge by one fine terrain-mesh cell.
    let channel = water - depth + (bank + 4.0).max(0.0) * 0.23 * s.vertical_scale;
    let sides = terrain.max(water + 18.0 * s.vertical_scale);
    let valley = channel + (sides - channel) * smooth(17.0, 190.0, bank);
    let weight = (1.0 - smooth(880.0, SOURCE, along))
        * smooth(-1152.0, -1030.0, along)
        * (1.0 - smooth(110.0, 225.0, bank));
    terrain + (valley - terrain) * weight
}

pub(crate) fn vegetation_cover(s: &GeneratorSettings, x: f64, z: f64, height: f64) -> f64 {
    let along = z / s.terrain_scale;
    if !(MOUTH..SOURCE).contains(&along) || x.abs() > 1152.0 * s.terrain_scale {
        return 1.0;
    }
    let (main, half, arm, arm_half) = section(s, z);
    let growth = |center: f64, width: f64, water: f64| {
        let bank = (x - center).abs() - width;
        let above_water = (height - water) / s.vertical_scale;
        1.0 - (1.0 - smooth(0.0, 5.0, bank / s.terrain_scale))
            * (1.0 - smooth(0.1, 1.4, above_water))
    };
    let mut cover = growth(main, half, head(s, along));
    if arm_half > 0.0 {
        cover = cover.min(growth(
            arm,
            arm_half,
            arm_head(s, along, arm - main - arm_half - half),
        ));
    }
    cover
}

pub(crate) fn spawn(s: &GeneratorSettings) -> ([f64; 2], [f64; 2]) {
    let z = 62.0 * s.terrain_scale;
    let (x, half, _, _) = section(s, z);
    let target_z = (FALL_FOOT + FALL_CREST) * 0.5 * s.terrain_scale;
    let target_x = section(s, target_z).0;
    ([x - half - 23.0 * s.terrain_scale, z], [target_x, target_z])
}

pub fn waterfall_view(s: &GeneratorSettings) -> ([f64; 3], [f64; 3]) {
    let ([x, z], [tx, tz]) = spawn(s);
    (
        [
            x - 14.0 * s.terrain_scale,
            head(s, z / s.terrain_scale) + 62.0,
            z - 55.0 * s.terrain_scale,
        ],
        [tx, head(s, (FALL_FOOT + FALL_CREST) * 0.5), tz],
    )
}

/// Position and half-width of the hydraulic jump below the falling sheet.
pub fn plunge_pool(s: &GeneratorSettings) -> ([f64; 3], f64) {
    let along = FALL_FOOT - 3.0;
    let z = along * s.terrain_scale;
    let (x, half_width, _, _) = section(s, z);
    ([x, head(s, along), z], half_width)
}

/// One static batch. Branches use the same stations for a seamless confluence.
/// The tributary takes a gentler grade after leaving the shared channel.
pub fn water_mesh(s: &GeneratorSettings) -> MeshData {
    surface_mesh(s, false)
}

/// The same surface with a red-channel aeration mask for the plunge pool.
pub fn animated_water_mesh(s: &GeneratorSettings) -> MeshData {
    surface_mesh(s, true)
}

fn surface_mesh(s: &GeneratorSettings, aerated: bool) -> MeshData {
    let mut mesh = MeshData::default();
    for tributary in [false, true] {
        let (start, end) = if tributary {
            (JOIN, 912.0)
        } else {
            (MOUTH, SOURCE)
        };
        let rows = ((end - start) / 4.0) as u32 + 1;
        let first = mesh.positions.len() as u32;
        let mut previous_span = 0.0;
        for row in 0..rows {
            let along = start + f64::from(row) * 4.0;
            let z = along * s.terrain_scale;
            let (main, half, arm, arm_half) = section(s, z);
            let gap = arm - main - arm_half - half;
            let water = |station| {
                if tributary {
                    arm_head(s, station, gap)
                } else {
                    head(s, station)
                }
            };
            let y = water(along) + 0.045;
            let (left, right) = if tributary {
                let right = (arm + arm_half).max(main + half);
                ((arm - arm_half).max(main + half).min(right), right)
            } else {
                (main - half, main + half)
            };
            let rise = water(along + 0.5) - water(along - 0.5);
            let pitch = rise / s.terrain_scale;
            let norm = 1.0 / pitch.hypot(1.0);
            let normal = [0.0, norm as f32, (-pitch * norm) as f32];
            // Ordinary river spans encode cross-channel position for bank foam.
            let flow = if pitch > 0.65 { -1.0 } else { 2.0 };
            let aeration = if !aerated {
                1.0
            } else if tributary {
                0.0
            } else {
                smooth(FALL_FOOT - 45.0, FALL_FOOT - 5.0, along)
                    * (1.0 - smooth(FALL_FOOT - 1.0, FALL_FOOT + 4.0, along))
            };
            for (x, across) in [(left, 0.0), (right, 1.0)] {
                mesh.vertex(
                    [x as f32, y as f32, z as f32],
                    normal,
                    [aeration as f32, 1.0, 1.0, 1.0],
                    [across, flow],
                );
            }
            if row > 0 && (left < right || previous_span > 0.0) {
                let i = first + 2 * (row - 1);
                mesh.indices.extend([i, i + 2, i + 1, i + 1, i + 2, i + 3]);
            }
            previous_span = right - left;
        }
    }
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Generator, Preset, WorldFile};

    #[test]
    fn finite_network_has_a_wet_bed_a_dry_spawn_and_a_waterfall() {
        let s = GeneratorSettings::for_preset(Preset::RiverValley, 42);
        let g = Generator::new(s.clone(), Vec::new());
        let mesh = water_mesh(&s);
        assert!(mesh.indices.len() < 6000 && mesh.uv.iter().any(|uv| uv[1] < 0.0));
        let trunk_vertices = (((SOURCE - MOUTH) / 4.0) as usize + 1) * 2;
        let mut shared_edge = false;
        for arm in mesh.positions[trunk_vertices..].chunks_exact(2) {
            let row = ((f64::from(arm[0][2]) - MOUTH) / 4.0) as usize;
            let bank = mesh.positions[row * 2 + 1];
            assert!(arm[0][0] >= bank[0]);
            if arm[0][0] == bank[0] && arm[1][0] > bank[0] {
                assert_eq!(arm[0][1], bank[1]);
                shared_edge = true;
            }
        }
        assert!(shared_edge, "tributary must join the trunk without overlap");
        assert!(mesh.uv[trunk_vertices..].iter().all(|uv| uv[1] > 0.0));
        assert_eq!(mesh.positions[0][1], (s.sea_level + 0.045) as f32);
        for z in [-800.0, -48.0, 64.0, 124.0, 128.0, 132.0, 136.0, 420.0] {
            let (x, trunk_half, arm, half) = section(&s, z);
            assert!(g.height(x, z) < head(&s, z), "{z}");
            assert_eq!(g.sample(x, z).vegetation, 0.0, "{z}");
            if half > 6.0 {
                assert!(
                    g.height(arm, z) < arm_head(&s, z, arm - x - trunk_half - half),
                    "{z}"
                );
                assert_eq!(g.sample(arm, z).vegetation, 0.0, "{z}");
            }
        }
        let (bank, _) = spawn(&s);
        assert!(g.height(bank[0], bank[1]) > head(&s, bank[1]));
        assert_eq!(g.sample(bank[0], bank[1]).vegetation, s.vegetation);
        let world = WorldFile::new("River Valley".into(), s);
        let restored = Generator::for_world(&world).unwrap();
        assert_eq!(restored.height(0.0, 128.0), g.height(0.0, 128.0));
    }
}
