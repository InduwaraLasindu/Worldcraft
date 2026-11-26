//! Distance-dependent nature visibility, independent of chunk topology changes.
//! Tree geometry reaches zero area before culling. Ground and rocks use range dithering.
use crate::{performance::Performance, session::Graphics};
use bevy::{camera::visibility::VisibilityRange, prelude::*};

#[derive(Component, Clone, Copy, Debug)]
pub enum Foliage {
    Solid,
    Wood,
    DetailedCrown,
    PairedCrown,
    SimpleCrown,
    Ground,
}

impl Foliage {
    pub fn tree_radius(radius: i64, skyline: bool) -> i64 {
        if skyline {
            radius.min(4)
        } else {
            radius
        }
    }
    pub fn outer_end(radius: i64) -> f32 {
        (radius.max(2) as f32 * 128.0 - 32.0).min(736.0)
    }
    // A full spray spans roughly one metre. Beyond this band the larger
    // parent sprays retain the outline with fewer alpha-tested triangles.
    pub fn crown_end(radius: i64) -> f32 {
        (Self::outer_end(radius) - 96.0).min(160.0)
    }
    pub fn range(self, radius: i64) -> VisibilityRange {
        // Finish fading before any resident tile can leave the streaming square.
        // Clamp the visual horizon rather than rendering tiny crowns indefinitely.
        let end = Self::outer_end(radius);
        let exit = (end - 96.0)..end;
        let crown_end = Self::crown_end(radius);
        // Geometry has reached the coarse endpoint before this topology switch.
        let crown = crown_end..crown_end;
        let (start_margin, end_margin) = match self {
            Self::DetailedCrown => (0.0..0.0, crown),
            Self::PairedCrown => (crown, end..end),
            Self::Ground => (0.0..0.0, 80.0..128.0),
            Self::Wood | Self::SimpleCrown => (0.0..0.0, end..end),
            Self::Solid => (0.0..0.0, exit),
        };
        VisibilityRange {
            start_margin,
            end_margin,
            // Paired crowns must use exactly the same origin, regardless of bounds.
            // Ground batches are explicitly recentered on their own local anchor.
            use_aabb: false,
        }
    }
}

/// Raster representations without a secondary-ray mesh: duplicate paired crowns
/// and alpha-cluster ground cover (the ray backend has no alpha-cutoff field).
#[derive(Component)]
pub struct NoRayProxy;

pub fn update_ranges(
    mut commands: Commands,
    graphics: Res<Graphics>,
    limits: Res<Performance>,
    session: Res<crate::session::Session>,
    mut previous: Local<Option<(i64, i64)>>,
    foliage: Query<(Entity, &Foliage)>,
) {
    let radius = limits.radius(graphics.radius);
    let tree_radius = Foliage::tree_radius(
        radius,
        session.generator.as_ref().is_some_and(|g| g.version() >= 3),
    );
    if *previous == Some((radius, tree_radius)) {
        return;
    }
    *previous = Some((radius, tree_radius));
    for (entity, kind) in &foliage {
        let distance = if matches!(kind, Foliage::Solid) {
            radius
        } else {
            tree_radius
        };
        commands.entity(entity).insert(kind.range(distance));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crown_switches_have_no_distance_gaps_and_finish_before_tile_eviction() {
        for radius in 2..=12 {
            let near = Foliage::DetailedCrown.range(radius);
            let far = Foliage::PairedCrown.range(radius);
            let solid = Foliage::Wood.range(radius);
            assert_eq!(near.end_margin, far.start_margin);
            assert_eq!(solid.end_margin, far.end_margin);
            assert_eq!(far.end_margin.start, far.end_margin.end);
            assert!(near.end_margin.end <= far.end_margin.start);
            assert!(far.end_margin.end < radius as f32 * 128.0);
            for step in 0..(far.end_margin.end * 4.0) as u32 {
                let distance = step as f32 * 0.25;
                assert!(near.is_visible_at_all(distance) || far.is_visible_at_all(distance));
            }
            // Detailed trees are prefetched through band two. A band-three
            // tile cannot come closer than 256 m before promotion.
            assert!(near.end_margin.end <= 160.0);
        }
        // Ground batches are prefetched in the immediate 3x3 tile neighborhood.
        assert!(Foliage::Ground.range(2).is_culled(128.0));
    }
}
