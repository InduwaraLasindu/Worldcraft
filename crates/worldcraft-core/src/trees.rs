//! Seeded tree anatomy. The client instances a small bank of index meshes with
//! independent stature, crown spread and clear-trunk height at every placement.
use crate::{generation::hash, MeshData};
use glam::{Vec2, Vec3};
use std::f32::consts::{PI, TAU};

pub const HEIGHTS: [f32; 5] = [12.5, 16.8, 11.6, 13.4, 9.2];
pub const RECIPE_COUNT: u16 = 2048;
pub const ATLAS_SIZE: [usize; 2] = [1024, 512];
const TILE: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shape {
    pub prototype: u8,
    pub height: f32,
    pub spread: f32,
    pub clearance: f32,
}
pub fn shape(id: u16) -> Shape {
    Shape {
        prototype: ((id >> 9) & 3) as u8,
        height: 0.77 + (id & 7) as f32 * 0.068,
        spread: 0.73 + ((id >> 3) & 7) as f32 * 0.079,
        clearance: ((id >> 6) & 7) as f32 / 7.0,
    }
}
pub fn recipe(seed: u64, family: usize, x: f64, z: f64) -> u16 {
    (hash(
        seed ^ 0x7463_726f_776e ^ family as u64,
        (x * 4.0).floor() as i64,
        (z * 4.0).floor() as i64,
    ) & u64::from(RECIPE_COUNT - 1)) as u16
}

struct Random(u64);
impl Random {
    fn unit(&mut self) -> f32 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        ((z ^ (z >> 31)) >> 40) as f32 / 16777216.0
    }
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.unit()
    }
}

fn add(mesh: &mut MeshData, p: Vec3, n: Vec3, uv: Vec2, shade: f32, flag: f32) -> u32 {
    mesh.vertex(
        p.to_array(),
        n.to_array(),
        [shade, shade, shade, flag],
        uv.to_array(),
    )
}

/// One shared ring at each bend supplies continuous normals and taper.
fn sweep(mesh: &mut MeshData, path: &[Vec3], radii: &[f32], sides: usize, phase: f32) {
    let base = mesh.positions.len() as u32;
    let mut axis = Vec3::X;
    let mut length = 0.0;
    for (i, p) in path.iter().copied().enumerate() {
        let prev = path[i.saturating_sub(1)];
        let next = path[(i + 1).min(path.len() - 1)];
        let tangent = (next - prev).normalize();
        let projected = axis - tangent * axis.dot(tangent);
        axis = if projected.length_squared() > 0.01 {
            projected.normalize()
        } else {
            tangent.any_orthonormal_vector()
        };
        let bitangent = tangent.cross(axis);
        length += p.distance(prev);
        for k in 0..=sides {
            let u = k as f32 / sides as f32;
            let (sin, cos) = (u * TAU).sin_cos();
            let normal = axis * cos + bitangent * sin;
            let ridges =
                (u * TAU * 5.0 + phase).sin() * 0.15 + (u * TAU * 3.0 + phase * 2.0).cos() * 0.06;
            let radius = radii[i] * (1.0 + ridges * if i < 3 { 1.0 } else { 0.5 });
            add(
                mesh,
                p + normal * radius,
                normal,
                Vec2::new(u * 3.0, length * 0.72),
                1.0,
                1.0,
            );
        }
        if i > 0 {
            let row = (sides + 1) as u32;
            for k in 0..sides as u32 {
                let a = base + (i - 1) as u32 * row + k;
                mesh.indices
                    .extend([a, a + 1, a + row, a + 1, a + row + 1, a + row]);
            }
        }
    }
}

fn dir(a: f32) -> Vec3 {
    Vec3::new(a.cos(), 0.0, a.sin())
}
fn quad_branch(a: Vec3, b: Vec3, c: Vec3, t: f32) -> Vec3 {
    a.lerp(b, t).lerp(b.lerp(c, t), t)
}

fn curved_limb(
    mesh: &mut MeshData,
    curve: [Vec3; 3],
    width: [f32; 2],
    sides: usize,
    segments: usize,
    phase: f32,
) {
    let [start, bend, tip] = curve;
    let mut path = [Vec3::ZERO; 5];
    let mut radii = [0.0; 5];
    for i in 0..=segments {
        let t = i as f32 / segments as f32;
        path[i] = quad_branch(start, bend, tip, t);
        radii[i] = width[1] + (width[0] - width[1]) * (1.0 - t).powf(1.17);
    }
    sweep(mesh, &path[..=segments], &radii[..=segments], sides, phase);
}

struct Grove {
    wood: MeshData,
    leaves: MeshData,
    rng: Random,
    family: usize,
    coarse: bool,
}

impl Grove {
    fn sprig(&mut self, root: Vec3, axis: Vec3, width: f32, length: f32, shade: f32, draw: bool) {
        let along = axis.normalize();
        let cross = along.cross(Vec3::Y).normalize_or(Vec3::X);
        let across =
            cross * self.rng.range(0.55, 0.99) + along.cross(cross) * self.rng.range(-0.71, 0.71);
        let u = across.normalize() * width * 0.5;
        let v = along * length;
        let n = u.cross(v).normalize().lerp(Vec3::Y, 0.23).normalize();
        let slot = (self.rng.unit() * 8.0) as usize;
        if !draw {
            return;
        }
        let row = [1, 0, 0, 2, 3][self.family];
        let uv = |x: f32, y: f32| {
            Vec2::new(
                (slot as f32 * 128.0 + 3.0 + x * 122.0) / 1024.0,
                (row as f32 * 128.0 + 3.0 + (1.0 - y) * 122.0) / 512.0,
            )
        };
        let p = root + v * 0.52;
        let start = self.leaves.positions.len() as u32;
        for (point, tex) in [
            (root - u * 0.69, uv(0.0, 0.0)),
            (root + u * 0.69, uv(1.0, 0.0)),
            (p - u, uv(0.0, 0.52)),
            (p + u, uv(1.0, 0.52)),
            (root + v + n * width * 0.15 - u * 0.57, uv(0.0, 1.0)),
            (root + v + n * width * 0.15 + u * 0.57, uv(1.0, 1.0)),
        ] {
            add(&mut self.leaves, point, n, tex, shade, 1.0);
        }
        self.leaves
            .indices
            .extend([0, 1, 2, 1, 3, 2, 2, 3, 4, 3, 5, 4].map(|i| i + start));
    }

    fn pine(&mut self, stem: impl Fn(f32) -> Vec3, radius: f32, profile: u8) {
        let rotation = self.rng.range(0.0, TAU);
        for tier in 0..11 {
            let t = 0.23 + tier as f32 * 0.065 + self.rng.range(-0.014, 0.014);
            let length =
                (1.0 - (t - 0.23) / 0.78).powf(0.84) * (3.85 + f32::from(profile) * 0.17) + 0.29;
            let spokes = if tier < 5 { 4 } else { 3 };
            for s in 0..spokes {
                let a = rotation
                    + tier as f32 * 1.11
                    + s as f32 / spokes as f32 * TAU
                    + self.rng.range(-0.34, 0.34);
                let d = dir(a);
                let root = stem(t + self.rng.range(-0.015, 0.015));
                let span = length * self.rng.range(0.72, 1.29);
                let mid = root + d * span * 0.54 - Vec3::Y * span * self.rng.range(0.055, 0.20);
                let tip = root + d * span + Vec3::Y * span * self.rng.range(-0.013, 0.25);
                let bark = (radius * (1.0 - t) * 0.43).max(0.014);
                curved_limb(
                    &mut self.wood,
                    [root, mid, tip],
                    [bark, 0.008],
                    if self.coarse { 3 } else { 4 },
                    if self.coarse { 2 } else { 4 },
                    a,
                );
                // Secondary sprays angle out and upward at irregular stations,
                // leaving gaps between older branches and a dense live tip.
                for j in 0..6 {
                    let along = 0.32 + j as f32 * 0.108;
                    let p = quad_branch(root, mid, tip, along);
                    let side = if j % 2 == 0 { 1.0 } else { -1.0 };
                    let bough = dir(a + side * self.rng.range(0.43, 1.16))
                        + Vec3::Y * self.rng.range(0.12, 0.48);
                    let reach = (span * (0.48 - along * 0.23) + 0.28) * self.rng.range(0.72, 1.24);
                    let shade = 0.62 + along * 0.35;
                    if !self.coarse {
                        sweep(
                            &mut self.wood,
                            &[p, p + bough * reach * 0.82],
                            &[bark * 0.23, 0.005],
                            3,
                            a,
                        );
                    }
                    self.sprig(
                        p + bough * reach * 0.27,
                        bough + Vec3::Y * 0.38,
                        reach * 0.72,
                        reach * 0.83,
                        shade,
                        !self.coarse,
                    );
                    self.sprig(p, bough, reach * 0.83, reach * 1.32, shade * 0.92, true);
                }
                self.sprig(
                    quad_branch(root, mid, tip, 0.91),
                    d + Vec3::Y * 0.42,
                    length * 0.26 + 0.19,
                    length * 0.4 + 0.39,
                    0.98,
                    true,
                );
            }
        }
        for i in 0..5 {
            let a = i as f32 * 2.40;
            self.sprig(
                stem(0.905 + i as f32 * 0.017),
                dir(a) * 0.54 + Vec3::Y * 0.81,
                0.48,
                0.90,
                0.96,
                true,
            );
        }
    }

    fn broadleaf(&mut self, stem: impl Fn(f32) -> Vec3, radius: f32, profile: u8) {
        let birch = self.family == 3;
        let acacia = self.family == 4;
        let h = HEIGHTS[self.family];
        let count = if birch {
            9
        } else if acacia {
            7
        } else {
            8
        };
        for i in 0..count {
            let level = i as f32 / (count - 1) as f32;
            let t = if acacia {
                0.26 + 0.39 * level
            } else {
                0.32 + 0.44 * level
            };
            let a = i as f32 * 2.39996 + self.rng.range(-0.51, 0.51);
            let d = dir(a);
            let root = stem(t);
            let r = self.rng.range(0.80, 1.26) * (1.0 + f32::from(profile) * 0.05);
            let reach = (if birch {
                1.7
            } else if acacia {
                4.4
            } else {
                3.5
            }) * r
                * (1.10 - level * 0.51);
            let level_y = if acacia {
                0.83 + level * 0.06
            } else {
                0.69 + level * 0.25
            };
            let tip = root + d * reach + Vec3::Y * (level_y * h - root.y);
            let elbow = root.lerp(tip, 0.49) + Vec3::Y * (if acacia { 0.53 } else { 0.14 });
            let start = (radius * (0.68 - level * 0.37)).max(0.030);
            curved_limb(
                &mut self.wood,
                [root, elbow, tip],
                [start, 0.018],
                if self.coarse { 4 } else { 6 },
                if self.coarse { 2 } else { 4 },
                a,
            );
            for j in 0..5 {
                let fraction = 0.39 + j as f32 * 0.137;
                let p = quad_branch(root, elbow, tip, fraction);
                let b = a + self.rng.range(0.44, 1.43) * if j % 2 == 0 { 1.0 } else { -1.0 };
                let out = dir(b);
                let span = (if birch { 0.91 } else { 1.45 }) * self.rng.range(0.82, 1.29);
                let rise = if birch {
                    0.27
                } else if acacia {
                    0.44
                } else {
                    0.93
                };
                let far = p + out * span + Vec3::Y * rise;
                let joint = p.lerp(far, 0.49) + Vec3::Y * 0.13;
                curved_limb(
                    &mut self.wood,
                    [p, joint, far],
                    [start * 0.30, 0.006],
                    4,
                    if self.coarse { 2 } else { 4 },
                    b,
                );
                for k in 0..6 {
                    let alpha = k as f32 / 5.0;
                    let leaf_dir = dir(b + self.rng.range(-1.51, 1.51))
                        + Vec3::Y
                            * self.rng.range(
                                if birch { -0.67 } else { -0.20 },
                                if acacia { 0.31 } else { 0.75 },
                            );
                    let at = (0.47 + alpha * 0.49 + self.rng.range(-0.02, 0.02)).min(1.0);
                    let point = quad_branch(p, joint, far, at);
                    let width = if birch {
                        0.69
                    } else if acacia {
                        0.89
                    } else {
                        1.08
                    };
                    let scale = self.rng.range(0.68, 1.31);
                    self.sprig(
                        point,
                        leaf_dir,
                        width * scale,
                        (width + 0.48) * scale,
                        0.65 + 0.33 * alpha,
                        !self.coarse || k % 2 == 0,
                    );
                }
            }
        }
    }

    fn leaflet(&mut self, base: Vec3, end: Vec3, normal: Vec3, width: f32, light: f32) {
        let mid = base.lerp(end, 0.45) + normal * 0.035;
        let side = normal.cross(end - base).normalize() * width;
        let n = normal.lerp(Vec3::Y, 0.26).normalize();
        let start = self.leaves.positions.len() as u32;
        for (p, uv) in [
            (base, Vec2::new(0.5, 0.0)),
            (mid + side, Vec2::new(1.0, 0.43)),
            (mid + n * width * 0.46, Vec2::new(0.5, 0.48)),
            (mid - side, Vec2::new(0.0, 0.43)),
            (end, Vec2::new(0.5, 1.0)),
        ] {
            add(&mut self.leaves, p, n, uv, light, 1.0);
        }
        self.leaves
            .indices
            .extend([0, 1, 2, 0, 2, 3, 1, 4, 2, 2, 4, 3].map(|i| i + start));
    }

    fn palm(&mut self, root: Vec3, profile: u8) {
        let rotation = self.rng.range(0.0, TAU);
        for i in 0..13 {
            let age = i as f32 / 12.0;
            let d = dir(rotation + i as f32 * 2.39996);
            let side = d.cross(Vec3::Y);
            let reach = self.rng.range(3.1, 4.6) * (0.83 + age * 0.17 + f32::from(profile) * 0.032);
            let a = root + d * 0.13;
            let b = root + d * reach * 0.45 + Vec3::Y * (2.50 - age * 1.69);
            let c = root + d * reach - Vec3::Y * (0.15 + age * 2.48);
            let path: Vec<_> = (0..=5)
                .map(|j| quad_branch(a, b, c, j as f32 / 5.0))
                .collect();
            sweep(
                &mut self.wood,
                &path,
                &[0.067, 0.054, 0.037, 0.025, 0.014, 0.006],
                4,
                age,
            );
            let segments = 19;
            for j in 0..segments {
                let t = 0.16 + 0.81 * j as f32 / segments as f32;
                for sign in [-1.0, 1.0] {
                    let at = quad_branch(a, b, c, t + sign * 0.007);
                    let span =
                        (t.powf(0.6) * PI).sin() * (0.81 + age * 0.39) * self.rng.range(0.78, 1.14);
                    let end = at + side * sign * span + d * span * (0.42 + t * 0.42)
                        - Vec3::Y * span * (0.15 + age * 0.46);
                    if !self.coarse || j % 2 == 0 {
                        self.leaflet(
                            at,
                            end,
                            Vec3::Y + side * sign * 0.17,
                            0.055 + span * 0.049,
                            0.84 + t * 0.16,
                        );
                    }
                }
            }
        }
    }
}

pub fn meshes(seed: u64, family: usize, profile: u8, coarse: bool) -> (MeshData, MeshData) {
    let mut grove = Grove {
        wood: MeshData::default(),
        leaves: MeshData::default(),
        rng: Random(hash(
            seed ^ 0x626f_7461_6e79,
            family as i64,
            i64::from(profile),
        )),
        family,
        coarse,
    };
    let h = HEIGHTS[family];
    let radius = [0.48, 0.34, 0.31, 0.24, 0.39][family] * grove.rng.range(0.81, 1.21);
    let direction = dir(grove.rng.range(0.0, TAU));
    let lean = grove.rng.range(0.23, if family == 2 { 2.4 } else { 1.27 });
    let stem = |t: f32| {
        Vec3::Y * (h * t - 0.36) + direction * (lean * t * t + (t * PI * 1.65).sin() * t * 0.18)
    };
    let path: Vec<_> = (0..=13).map(|i| stem(i as f32 / 13.0)).collect();
    let radii: Vec<_> = (0..=13)
        .map(|i| {
            let t = i as f32 / 13.0;
            if family == 2 {
                radius * (0.96 - t * 0.24 + (1.0 - t).powi(8) * 0.22)
            } else {
                (radius * ((1.0 - t).powf(0.93) + (1.0 - t).powi(12) * 0.38)).max(0.016)
            }
        })
        .collect();
    sweep(
        &mut grove.wood,
        &path,
        &radii,
        if coarse { 7 } else { 11 },
        lean,
    );
    if family != 2 {
        for i in 0..5 {
            let out = dir(i as f32 * TAU / 5.0 + grove.rng.range(-0.33, 0.33));
            let foot = out * radius * grove.rng.range(2.6, 3.9) - Vec3::Y * 0.25;
            let curve = [Vec3::Y * 0.31, out * radius * 1.37 - Vec3::Y * 0.04, foot];
            if !coarse {
                sweep(
                    &mut grove.wood,
                    &curve,
                    &[radius * 0.49, radius * 0.29, radius * 0.02],
                    5,
                    lean,
                );
            }
        }
    }
    match family {
        1 => grove.pine(stem, radius, profile),
        2 => grove.palm(stem(0.996), profile),
        _ => grove.broadleaf(stem, radius, profile),
    }
    (grove.wood, grove.leaves)
}

struct Mask<'a> {
    atlas: &'a mut [u8],
    x: usize,
    y: usize,
}

impl Mask<'_> {
    fn paint(&mut self, x: i32, y: i32, alpha: f32, pigment: [u8; 3]) {
        if !(3..125).contains(&x) || !(3..125).contains(&y) {
            return;
        }
        let i = ((self.y + y as usize) * ATLAS_SIZE[0] + self.x + x as usize) * 4;
        let alpha = (alpha * 255.0) as u8;
        if alpha > 0 {
            self.atlas[i..i + 3].copy_from_slice(&pigment);
            self.atlas[i + 3] = self.atlas[i + 3].max(alpha);
        }
    }
    fn line(&mut self, a: Vec2, b: Vec2, radius: f32, pigment: [u8; 3]) {
        let delta = b - a;
        let len = delta.length_squared().max(0.01);
        let lo = (a.min(b) - Vec2::splat(radius + 0.7)).floor();
        let hi = (a.max(b) + Vec2::splat(radius + 0.7)).ceil();
        for y in lo.y as i32..=hi.y as i32 {
            for x in lo.x as i32..=hi.x as i32 {
                let p = Vec2::new(x as f32, y as f32) - a;
                let dist = (p - delta * (p.dot(delta) / len).clamp(0.0, 1.0)).length();
                self.paint(x, y, (radius + 0.54 - dist).clamp(0.0, 1.0), pigment);
            }
        }
    }
    fn blade(&mut self, a: Vec2, end: Vec2, width: f32, tone: u8, lobes: f32) {
        let delta = end - a;
        let length = delta.length();
        let axis = delta / length.max(0.001);
        let cross = axis.perp();
        let lo = (a.min(end) - Vec2::splat(width + 1.0)).floor();
        let hi = (a.max(end) + Vec2::splat(width + 1.0)).ceil();
        for y in lo.y as i32..=hi.y as i32 {
            for x in lo.x as i32..=hi.x as i32 {
                let rel = Vec2::new(x as f32, y as f32) - a;
                let t = (rel.dot(axis) / length).clamp(0.0, 1.0);
                let lateral = rel.dot(cross).abs();
                let edge = width * (t * PI).sin().powf(0.77) * (1.0 + lobes * (t * 37.0).cos());
                let alpha = (edge + 0.5 - lateral).clamp(0.0, 1.0);
                if t == 0.0 || t == 1.0 || alpha <= 0.0 {
                    continue;
                }
                let midrib = (1.0 - lateral * 1.3).max(0.0);
                let veins = ((t * 28.0 + lateral * 0.65).sin() - 0.74).max(0.0);
                let rough = (t * 97.0 + lateral * 1.27).sin() * 9.0;
                self.paint(
                    x,
                    y,
                    alpha,
                    [
                        (f32::from(tone) + rough) as u8,
                        (midrib * 180.0 + veins * 160.0) as u8,
                        238,
                    ],
                );
            }
        }
    }

    fn needles(&mut self, rng: &mut Random, base: Vec2, tip: Vec2) {
        self.line(base, tip, 1.45, [124, 7, 39]);
        let axis = (tip - base).normalize();
        let side = axis.perp();
        for j in 0..8 {
            let t = 0.18 + j as f32 * 0.097;
            for sign in [-1.0, 1.0] {
                let root = base.lerp(tip, t + rng.range(-0.035, 0.034));
                let length = (37.5 * (1.0 - t).powf(0.64) + 5.0) * rng.range(0.73, 1.16);
                let bough = (axis * rng.range(0.53, 1.23) + side * sign).normalize();
                let end = root + bough * length;
                self.line(root, end, 0.85, [132, 9, 47]);
                for k in 0..17 {
                    let along = (k as f32 + 1.0) / 18.0 + rng.range(-0.027, 0.026);
                    let start = root.lerp(end, along);
                    let leaf = bough.perp() * if k % 2 == 0 { 1.0 } else { -1.0 };
                    let span = (12.0 - along * 7.2) * rng.range(0.71, 1.44);
                    let to =
                        start + (bough * rng.range(0.3, 0.85) + leaf * 0.76).normalize() * span;
                    self.line(
                        start,
                        to,
                        rng.range(0.66, 1.07),
                        [rng.range(93.0, 224.0) as u8, 9, 235],
                    );
                }
            }
        }
        for j in 0..10 {
            let root = base.lerp(tip, j as f32 * 0.015 + 0.832);
            for sign in [-1.0, 1.0] {
                let end = root + axis * rng.range(4.0, 10.0) + side * rng.range(4.0, 8.0) * sign;
                self.line(root, end, 0.81, [192, 8, 231]);
            }
        }
    }

    fn broad(&mut self, rng: &mut Random, base: Vec2, tip: Vec2, row: usize) {
        self.line(base, tip, 1.6, [109, 13, 38]);
        let axis = (tip - base).normalize();
        let side = axis.perp();
        for i in 0..7 {
            let t = 0.17 + i as f32 * 0.107;
            for sign in [-1.0, 1.0] {
                let root = base.lerp(tip, t + rng.range(-0.047, 0.04));
                let twig = (axis * rng.range(0.3, 0.84) + side * sign).normalize();
                let length = (35.0 - t * 23.0) * rng.range(0.73, 1.24);
                let end = root + twig * length;
                self.line(root, end, 0.67, [147, 5, 53]);
                let count = if row == 3 { 6 } else { 3 };
                for k in 0..count {
                    let a = root.lerp(end, (k as f32 + 0.5) / count as f32);
                    let b = (twig * rng.range(0.40, 0.90)
                        + twig.perp() * if k % 2 == 0 { 0.83 } else { -0.88 })
                    .normalize();
                    let length = rng.range(0.75, 1.24)
                        * match row {
                            1 => 25.0,
                            2 => 19.0,
                            _ => 13.0,
                        };
                    let width = length
                        * match row {
                            1 => 0.35,
                            2 => 0.34,
                            _ => 0.27,
                        };
                    self.blade(
                        a,
                        a + b * length,
                        width,
                        rng.range(113.0, 233.0) as u8,
                        match row {
                            1 => 0.17,
                            2 => 0.08,
                            _ => 0.02,
                        },
                    );
                }
            }
        }
        self.blade(tip - axis * 9.0, tip + axis * 11.0, 5.0, 170, 0.05);
    }
}

pub fn foliage_atlas(seed: u64) -> Vec<u8> {
    let mut atlas = vec![0; ATLAS_SIZE[0] * ATLAS_SIZE[1] * 4];
    for pixel in atlas.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[142, 9, 235, 0]);
    }
    for row in 0..4 {
        for col in 0..8 {
            let mut rng = Random(hash(seed ^ 0x6174_6c6173, col as i64, row as i64));
            let mut mask = Mask {
                atlas: &mut atlas,
                x: col * TILE,
                y: row * TILE,
            };
            let base = Vec2::new(rng.range(57.0, 70.0), 120.0);
            let tip = Vec2::new(rng.range(46.0, 82.0), rng.range(14.0, 22.0));
            if row == 0 {
                mask.needles(&mut rng, base, tip);
            } else {
                mask.broad(&mut rng, base, tip, row);
            }
        }
    }
    atlas
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn all_2048_recipes_change_tree_geometry_parameters() {
        let shapes: HashSet<_> = (0..RECIPE_COUNT)
            .map(|id| {
                let form = shape(id);
                (
                    form.prototype,
                    form.height.to_bits(),
                    form.spread.to_bits(),
                    form.clearance.to_bits(),
                )
            })
            .collect();
        assert_eq!(shapes.len(), usize::from(RECIPE_COUNT));
        let locations: HashSet<_> = (0..8192)
            .map(|i| recipe(42, 1, f64::from(i) * 8.0, -64.0))
            .collect();
        assert!(locations.len() > 1900);
        assert_eq!(recipe(42, 1, 25.0, -64.0), recipe(42, 1, 25.0, -64.0));
        assert_ne!(recipe(42, 1, 25.0, -64.0), recipe(44, 1, 25.0, -64.0));
    }

    #[test]
    fn silhouettes_are_seeded_and_lod_saves_triangles() {
        for species in 0..5 {
            for profile in 0..4 {
                let (bark, crown) = meshes(42, species, profile, false);
                let (far_bark, far_crown) = meshes(42, species, profile, true);
                for mesh in [&bark, &crown, &far_bark, &far_crown] {
                    assert!(!mesh.indices.is_empty());
                    assert!(mesh.positions.iter().flatten().all(|v| v.is_finite()));
                    assert!(mesh.normals.iter().flatten().all(|v| v.is_finite()));
                    assert!(mesh
                        .indices
                        .iter()
                        .all(|&i| (i as usize) < mesh.positions.len()));
                }
                let detail_triangles = (bark.indices.len() + crown.indices.len()) / 3;
                let far_triangles = (far_bark.indices.len() + far_crown.indices.len()) / 3;
                assert!(detail_triangles < 5200 && far_triangles < 2300);
                assert!(far_triangles * 100 < detail_triangles * 75);
                let crown_full: HashSet<_> = crown
                    .positions
                    .iter()
                    .map(|p| p.map(f32::to_bits))
                    .collect();
                assert!(far_crown
                    .positions
                    .iter()
                    .all(|p| crown_full.contains(&p.map(f32::to_bits))));
            }
        }
        let (bark, crown) = meshes(42, 1, 2, false);
        assert_eq!(bark.positions, meshes(42, 1, 2, false).0.positions);
        assert_ne!(crown.positions, meshes(42, 1, 3, false).1.positions);
        let mask = foliage_atlas(42);
        assert_eq!(mask, foliage_atlas(42));
        assert_ne!(mask, foliage_atlas(43));
        assert!(mask.chunks_exact(4).any(|pixel| pixel[3] == 0));
        assert!(mask.chunks_exact(4).any(|pixel| pixel[3] == 255));
    }
}
