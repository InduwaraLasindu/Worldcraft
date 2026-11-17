//! Small procedural understory prototypes appended directly to 32 m batches.
//! No entities/materials/textures are allocated per blade, flower or fern.
use bevy::prelude::*;
use worldcraft_core::{
    mesh::MeshData,
    scatter::{Instance, Species},
};

fn leaf(m: &mut MeshData, center: Vec3, direction: Vec3, length: f32, width: f32, color: [f32; 4]) {
    let side = Vec3::new(-direction.z, 0.06, direction.x).normalize() * width;
    let along = direction * length;
    let n = along.cross(side).normalize();
    let start = m.positions.len() as u32;
    // A pointed blade/leaflet, rather than an opaque rectangular paddle.
    for p in [center - along, center - side, center + along, center + side] {
        m.vertex(p.to_array(), n.to_array(), color, [0.5, 0.5]);
    }
    m.indices
        .extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 3]);
}

pub fn append(m: &mut MeshData, i: &Instance, root: Vec3, normal: Vec3) {
    let start = m.positions.len();
    let s = i.scale;
    let height = match i.species {
        Species::Reed => 1.8 * s,
        Species::Fern => 0.85 * s,
        _ => 0.65 * s,
    };
    match i.species {
        Species::Fern => {
            for frond in 0..6 {
                let a = i.yaw + frond as f32 * 2.39996;
                let dir = Vec3::new(a.cos(), 0.0, a.sin());
                let side = Vec3::new(-dir.z, 0.0, dir.x);
                let point = |t: f32| {
                    root + dir * (t * 0.85 * s)
                        + Vec3::Y * (0.10 + height * (t * 1.7 - t * t * 1.2))
                };
                crate::tree::tube(m, point(0.0), point(0.5), 0.012 * s, 0.007 * s, 3);
                crate::tree::tube(m, point(0.5), point(1.0), 0.007 * s, 0.003 * s, 3);
                for row in 0..5 {
                    let t = 0.22 + row as f32 * 0.145;
                    let reach = (1.0 - t) * 0.32 * s + 0.035 * s;
                    for sign in [-1.0, 1.0] {
                        let direction = (side * sign + dir * 0.25 + Vec3::Y * 0.10).normalize();
                        leaf(
                            m,
                            point(t) + direction * reach * 0.5,
                            direction,
                            reach * 0.58,
                            reach * 0.19,
                            [
                                0.035 + row as f32 * 0.008,
                                0.14 + row as f32 * 0.012,
                                0.028,
                                1.0,
                            ],
                        );
                    }
                }
            }
        }
        Species::Flower => {
            let palettes = [
                [0.38, 0.18, 0.56, 1.0],
                [0.12, 0.28, 0.56, 1.0],
                [0.65, 0.48, 0.06, 1.0],
                [0.68, 0.68, 0.60, 1.0],
            ];
            let color = palettes[(i.variant >> 2) % 4];
            for stalk in 0..3 {
                let a = i.yaw + stalk as f32 * 2.39996;
                let base = root + Vec3::new(a.cos() * 0.14, 0.0, a.sin() * 0.14) * s;
                let top = base
                    + Vec3::new(
                        a.cos() * 0.08,
                        height * (0.65 + stalk as f32 * 0.12),
                        a.sin() * 0.08,
                    );
                crate::tree::tube(m, base, top, 0.010 * s, 0.004 * s, 3);
                for sign in [-1.0, 1.0] {
                    let direction = Vec3::new(a.cos() * sign, 0.28, a.sin() * sign).normalize();
                    leaf(
                        m,
                        base.lerp(top, 0.42) + direction * 0.08 * s,
                        direction,
                        0.11 * s,
                        0.025 * s,
                        [0.035, 0.13, 0.025, 1.0],
                    );
                }
                let center = m.vertex(
                    top.to_array(),
                    Vec3::Y.to_array(),
                    [0.50, 0.25, 0.025, 1.0],
                    [0.5, 1.0],
                );
                for petal in 0..5 {
                    let a = a + petal as f32 * std::f32::consts::TAU / 5.0;
                    let dir = Vec3::new(a.cos(), 0.15, a.sin());
                    let side = Vec3::new(-dir.z, 0.0, dir.x);
                    crate::tree::card(
                        m,
                        top + dir * 0.065 * s,
                        dir * 0.075 * s,
                        side * 0.040 * s,
                        color,
                    );
                    let n = Vec3::Y.to_array();
                    let a = m.vertex(
                        (top + dir * 0.035 * s + side * 0.025 * s).to_array(),
                        n,
                        [0.62, 0.38, 0.04, 1.0],
                        [0.0, 1.0],
                    );
                    let b = m.vertex(
                        (top + dir * 0.035 * s - side * 0.025 * s).to_array(),
                        n,
                        [0.62, 0.38, 0.04, 1.0],
                        [1.0, 1.0],
                    );
                    m.indices.extend_from_slice(&[center, a, b]);
                }
            }
        }
        Species::Reed => {
            for stalk in 0..5 {
                let a = i.yaw + stalk as f32 * 2.39996;
                let dir = Vec3::new(a.cos(), 0.0, a.sin());
                let base = root + dir * (0.08 + stalk as f32 * 0.07) * s;
                let top = base + Vec3::Y * height * (0.68 + stalk as f32 * 0.065) + dir * 0.10 * s;
                crate::tree::tube(m, base, top, 0.018 * s, 0.008 * s, 4);
                for sign in [-1.0, 1.0] {
                    let mid = base.lerp(top, 0.38);
                    let tip = mid + dir * sign * 0.40 * s + Vec3::Y * 0.38 * s;
                    leaf(
                        m,
                        (mid + tip) * 0.5,
                        (tip - mid).normalize(),
                        mid.distance(tip) * 0.5,
                        0.035 * s,
                        [0.09, 0.20, 0.045, 1.0],
                    );
                }
                let before = m.positions.len();
                crate::tree::tube(
                    m,
                    top - Vec3::Y * 0.22 * s,
                    top + Vec3::Y * 0.06 * s,
                    0.045 * s,
                    0.030 * s,
                    5,
                );
                for color in &mut m.colors[before..] {
                    *color = [0.11, 0.065, 0.025, 1.0];
                }
            }
        }
        _ => return,
    }
    let sx = normal.x / normal.y.max(0.25);
    let sz = normal.z / normal.y.max(0.25);
    for at in start..m.positions.len() {
        // The tube helper is also used by textured tree bark, whose default
        // vertex color is white. Herb stems use their own green pigment.
        if m.colors[at] == [1.0; 4] {
            m.colors[at] = [0.055, 0.15, 0.03, 1.0];
        }
        let p = Vec3::from_array(m.positions[at]);
        // Wind flexibility follows height above the local root, not terrain
        // altitude. Seat the plant on the same local plane used by grass roots.
        m.uv[at][1] = ((p.y - root.y) / height).clamp(0.0, 1.0);
        m.positions[at][1] -= sx * (p.x - root.x) + sz * (p.z - root.z);
        let n = Vec3::from_array(m.normals[at]);
        m.normals[at] = Vec3::new(n.x + sx * n.y, n.y, n.z + sz * n.y)
            .normalize()
            .to_array();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn procedural_understory_has_finite_bounded_geometry_and_root_relative_wind() {
        for species in [Species::Fern, Species::Flower, Species::Reed] {
            let mut mesh = MeshData::default();
            append(
                &mut mesh,
                &Instance {
                    position: [0.0, 1000.0, 0.0],
                    scale: 1.0,
                    yaw: 0.7,
                    species,
                    variant: 8,
                },
                Vec3::new(0.0, 1000.0, 0.0),
                Vec3::new(0.3, 0.9, 0.2).normalize(),
            );
            assert!(!mesh.indices.is_empty() && mesh.indices.len() / 3 <= 240);
            assert!(mesh
                .indices
                .iter()
                .all(|&i| (i as usize) < mesh.positions.len()));
            assert!(mesh
                .positions
                .iter()
                .all(|p| p.iter().all(|v| v.is_finite())));
            assert!(mesh
                .normals
                .iter()
                .all(|n| (Vec3::from_array(*n).length() - 1.0).abs() < 0.001));
            assert!(mesh.uv.iter().all(|uv| (0.0..=1.0).contains(&uv[1])));
        }
    }
}
