//! Deterministic drainage for a finite height grid with explicit boundary outlets.
use std::{
    cmp::{Ordering, Reverse},
    collections::BinaryHeap,
};

const MAX_CELLS: usize = 1_048_576;

#[derive(Debug, PartialEq)]
pub struct Drainage {
    /// Elevation in metres after filling depressions to their spill level.
    pub conditioned_height: Vec<f64>,
    /// A cardinal neighbor, or None for an explicit outlet.
    pub downstream: Vec<Option<usize>>,
    /// Number of cells draining through this cell, including the cell itself.
    pub contributing_cells: Vec<u32>,
    /// Index into `lakes` for submerged cells, with None for dry cells.
    pub lake_at: Vec<Option<usize>>,
    pub lakes: Vec<Lake>,
}

#[derive(Debug, PartialEq)]
pub struct Lake {
    pub level: f64,
    pub spill_from: usize,
    pub spill_to: usize,
}

#[derive(Debug, PartialEq)]
pub struct Channel {
    pub from: usize,
    pub to: usize,
    pub contributing_cells: u32,
}

impl Drainage {
    /// Emit qualifying edges while leaving the interior of each lake to its
    /// water surface. Inlets and outlets remain edges in the channel network.
    pub fn channels(&self, minimum_cells: u32) -> impl Iterator<Item = Channel> + '_ {
        self.downstream
            .iter()
            .enumerate()
            .filter_map(move |(from, to)| {
                let to = (*to)?;
                let area = self.contributing_cells[from];
                let inside_lake =
                    self.lake_at[from].is_some() && self.lake_at[from] == self.lake_at[to];
                (area >= minimum_cells && !inside_lake).then_some(Channel {
                    from,
                    to,
                    contributing_cells: area,
                })
            })
    }
}

#[derive(Clone, Copy)]
struct FloodCell {
    height: f64,
    index: usize,
}

impl Ord for FloodCell {
    fn cmp(&self, other: &Self) -> Ordering {
        self.height
            .total_cmp(&other.height)
            .then_with(|| self.index.cmp(&other.index))
    }
}

impl PartialOrd for FloodCell {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for FloodCell {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl Eq for FloodCell {}

fn neighbors(index: usize, width: usize, height: usize) -> impl Iterator<Item = usize> {
    let (x, y) = (index % width, index / width);
    [
        (x > 0).then(|| index - 1),
        (x + 1 < width).then(|| index + 1),
        (y > 0).then(|| index - width),
        (y + 1 < height).then(|| index + width),
    ]
    .into_iter()
    .flatten()
}

/// Priority flooding assigns every cell to the first lowest spill path that
/// reaches it. A flat cell drains to an earlier visited cell, preventing cycles
/// without numerical slopes or dependence on outlet insertion order.
pub fn drain(
    width: usize,
    height: usize,
    elevation: &[f64],
    outlets: &[usize],
) -> Result<Drainage, &'static str> {
    let count = width
        .checked_mul(height)
        .ok_or("grid dimensions overflow")?;
    if width < 2 || height < 2 || count > MAX_CELLS || count != elevation.len() {
        return Err("invalid drainage grid dimensions");
    }
    if elevation.iter().any(|value| !value.is_finite()) {
        return Err("non-finite elevation");
    }
    if outlets.is_empty() {
        return Err("at least one boundary outlet is required");
    }

    let mut visited = vec![false; count];
    let mut conditioned_height = elevation.to_vec();
    let mut downstream = vec![None; count];
    let mut frontier = BinaryHeap::new();
    let mut order = Vec::with_capacity(count);
    for &index in outlets {
        if index >= count {
            return Err("outlet index is outside the grid");
        }
        let (x, y) = (index % width, index / width);
        if x != 0 && x != width - 1 && y != 0 && y != height - 1 {
            return Err("outlet must be on the boundary");
        }
        if !visited[index] {
            visited[index] = true;
            frontier.push(Reverse(FloodCell {
                height: elevation[index],
                index,
            }));
        }
    }
    while let Some(Reverse(cell)) = frontier.pop() {
        order.push(cell.index);
        for neighbor in neighbors(cell.index, width, height) {
            if visited[neighbor] {
                continue;
            }
            visited[neighbor] = true;
            downstream[neighbor] = Some(cell.index);
            let spill = elevation[neighbor].max(cell.height);
            conditioned_height[neighbor] = spill;
            frontier.push(Reverse(FloodCell {
                height: spill,
                index: neighbor,
            }));
        }
    }

    let mut contributing_cells = vec![1; count];
    for index in order.into_iter().rev() {
        if let Some(next) = downstream[index] {
            contributing_cells[next] += contributing_cells[index];
        }
    }
    let mut lake_at = vec![None; count];
    let mut lakes = Vec::new();
    let mut pending = Vec::new();
    for first in 0..count {
        let level = conditioned_height[first];
        if lake_at[first].is_some() || elevation[first] >= level {
            continue;
        }
        let id = lakes.len();
        let submerged =
            |index: usize| conditioned_height[index] == level && elevation[index] < level;
        lake_at[first] = Some(id);
        pending.push(first);
        let mut spill = None;
        while let Some(cell) = pending.pop() {
            if let Some(to) = downstream[cell].filter(|&to| !submerged(to)) {
                let candidate = (to, cell);
                spill = Some(spill.map_or(candidate, |current| candidate.min(current)));
            }
            for neighbor in neighbors(cell, width, height) {
                if lake_at[neighbor].is_none() && submerged(neighbor) {
                    lake_at[neighbor] = Some(id);
                    pending.push(neighbor);
                }
            }
        }
        let (spill_to, spill_from) = spill.expect("filled basin must drain to its rim");
        lakes.push(Lake {
            level,
            spill_from,
            spill_to,
        });
    }
    Ok(Drainage {
        conditioned_height,
        downstream,
        contributing_cells,
        lake_at,
        lakes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depression_fills_to_its_spill_and_flat_cells_drain_without_cycles() {
        let elevation = [
            9., 9., 0., 9., 9., 9., 9., 4., 9., 9., 9., 1., 1., 1., 9., 9., 1., 1., 1., 9., 9., 9.,
            9., 9., 9.,
        ];
        let result = drain(5, 5, &elevation, &[2]).unwrap();
        assert_eq!(result.conditioned_height[12], 4.);
        assert_eq!(result.conditioned_height[17], 4.);
        assert_eq!(result.contributing_cells[2], 25);
        assert_eq!(result.lakes.len(), 1);
        assert_eq!(result.lakes[0].level, 4.);
        assert_eq!(result.lakes[0].spill_to, 7);
        assert_eq!(result.lake_at[12], result.lake_at[17]);
        assert!(result
            .channels(5)
            .any(|edge| edge.from == 7 && edge.to == 2));
        assert!(result
            .channels(1)
            .all(|edge| result.lake_at[edge.from].is_none()
                || result.lake_at[edge.from] != result.lake_at[edge.to]));
        for start in 0..25 {
            let mut cell = start;
            for _ in 0..25 {
                let Some(next) = result.downstream[cell] else {
                    break;
                };
                assert!(result.conditioned_height[next] <= result.conditioned_height[cell]);
                assert_ne!(cell, next);
                cell = next;
            }
            assert_eq!(cell, 2);
        }
    }

    #[test]
    fn outlet_order_does_not_change_flat_basins_and_counts_are_conserved() {
        let elevation = [0.; 16];
        let result = drain(4, 4, &elevation, &[0, 15]).unwrap();
        assert_eq!(result, drain(4, 4, &elevation, &[15, 0, 15]).unwrap());
        assert_eq!(
            result.contributing_cells[0] + result.contributing_cells[15],
            16
        );
        assert_eq!(
            result
                .downstream
                .iter()
                .filter(|cell| cell.is_none())
                .count(),
            2
        );
    }

    #[test]
    fn unsupported_grids_and_outlets_fail_without_a_partial_result() {
        assert!(drain(3, 3, &[1.; 9], &[4]).is_err());
        assert!(drain(3, 3, &[1.; 9], &[9]).is_err());
        assert!(drain(3, 3, &[1.; 9], &[]).is_err());
        assert!(drain(2, 2, &[0., 1., f64::NAN, 2.], &[0]).is_err());
        assert!(drain(usize::MAX, 2, &[], &[0]).is_err());
    }

    #[test]
    fn generated_terrain_at_negative_coordinates_has_conservative_drainage() {
        let generator = crate::Generator::new(crate::GeneratorSettings::default(), Vec::new());
        let elevation: Vec<_> = (0..17)
            .flat_map(|z| (0..13).map(move |x| (x, z)))
            .map(|(x, z)| {
                generator.height(-192.0 + f64::from(x) * 8.0, -384.0 + f64::from(z) * 8.0)
            })
            .collect();
        let result = drain(13, 17, &elevation, &[0, 12]).unwrap();
        assert_eq!(result.conditioned_height[0], elevation[0]);
        assert_eq!(
            result.contributing_cells[0] + result.contributing_cells[12],
            13 * 17
        );
        assert!(result.channels(4).all(|edge| {
            result.conditioned_height[edge.to] <= result.conditioned_height[edge.from]
        }));
    }
}
