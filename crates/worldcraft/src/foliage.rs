//! Distance-dependent nature visibility, independent of chunk topology changes.
//! Shared meshes crossfade through Bevy's forward/depth/shadow dither path.
use crate::{performance::Performance, session::Graphics};
use bevy::{camera::visibility::VisibilityRange, prelude::*};

#[derive(Component, Clone, Copy, Debug)]
pub enum Foliage {
    Solid,
    DetailedCrown,
    PairedCrown,
    SimpleCrown,
    Ground,
}

impl Foliage {
    pub fn range(self, radius: i64) -> VisibilityRange {
        // Finish fading before any resident tile can leave the streaming square.
        // Clamp the visual horizon rather than rendering tiny crowns indefinitely.
        let end = (radius.max(2) as f32 * 128.0 - 32.0).min(736.0);
        let exit = (end - 96.0)..end;
        let crown_end = exit.start.min(320.0);
        let crown = (crown_end - 64.0)..crown_end;
        let (start_margin, end_margin) = match self {
            Self::DetailedCrown => (0.0..0.0, crown),
            Self::PairedCrown => (crown, exit),
            Self::Ground => (0.0..0.0, 80.0..128.0),
            Self::Solid | Self::SimpleCrown => (0.0..0.0, exit),
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

/// A paired cheap crown is only a raster representation, not a second ray occluder.
#[derive(Component)]
pub struct NoRayProxy;

pub fn update_ranges(
    mut commands: Commands,
    graphics: Res<Graphics>,
    limits: Res<Performance>,
    mut previous: Local<Option<i64>>,
    foliage: Query<(Entity, &Foliage)>,
) {
    let radius = limits.radius(graphics.radius);
    if *previous == Some(radius) {
        return;
    }
    *previous = Some(radius);
    for (entity, kind) in &foliage {
        commands.entity(entity).insert(kind.range(radius));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crown_crossfades_have_no_distance_gaps_and_finish_before_tile_eviction() {
        for radius in 2..=12 {
            let near = Foliage::DetailedCrown.range(radius);
            let far = Foliage::PairedCrown.range(radius);
            let solid = Foliage::Solid.range(radius);
            assert_eq!(near.end_margin, far.start_margin);
            assert_eq!(solid.end_margin, far.end_margin);
            assert!(near.end_margin.end <= far.end_margin.start);
            assert!(far.end_margin.end < radius as f32 * 128.0);
            for step in 0..(far.end_margin.end * 4.0) as u32 {
                let distance = step as f32 * 0.25;
                assert!(near.is_visible_at_all(distance) || far.is_visible_at_all(distance));
            }
            // The detailed mesh is only resident in chunk bands 0..=3. On the
            // first frame a band-4 tile becomes band 3 its closest point is 384 m.
            assert!(near.end_margin.end < 384.0);
        }
        // Ground batches are prefetched in the immediate 3x3 tile neighborhood.
        assert!(Foliage::Ground.range(2).is_culled(128.0));
    }
}
