//! Deterministic drainage for a finite height grid with explicit boundary outlets.
use std::{
    cmp::{Ordering, Reverse},
    collections::BinaryHeap,
};

const MAX_CELLS: usize = 1_048_576;
// 1024 active cells, two 256-cell margins and the shared endpoint sample.
const MAX_REGIONAL_CELLS: usize = 1537 * 1537;
const NO_CELL: u32 = u32::MAX;
const UNVISITED: u32 = u32::MAX - 1;

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

#[derive(Debug, PartialEq)]
pub struct RegionalDrainage {
    /// Conditioned routing height. This is a water surface where `lake_index` is set.
    pub water_surface_m: Vec<f32>,
    /// Sum of annual precipitation weighted by contributing cell area and routing fraction.
    pub runoff_m3_per_year: Vec<f64>,
    /// Weighted horizontal outflow direction. Opposed flows can partially cancel.
    pub flow_direction: Vec<[f32; 2]>,
    pub lakes: Vec<LakeBasin>,
    outlets_or_receivers: Vec<u32>,
    lake_at: Vec<u32>,
}

#[derive(Debug, PartialEq)]
pub struct LakeBasin {
    pub level_m: f32,
    pub spill_from: usize,
    pub spill_to: usize,
    pub area_m2: f64,
    pub max_depth_m: f64,
    pub volume_m3: f64,
    /// Based on mean temperature across submerged cells, when temperatures are available.
    pub frozen: bool,
}

impl RegionalDrainage {
    /// The receiving neighbor with greatest routing fraction, or None at an outlet.
    pub fn downstream(&self, index: usize) -> Option<usize> {
        cell_index(*self.outlets_or_receivers.get(index)?)
    }

    pub fn lake_index(&self, index: usize) -> Option<usize> {
        cell_index(*self.lake_at.get(index)?)
    }
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

fn cell_index(index: u32) -> Option<usize> {
    (index != NO_CELL).then_some(index as usize)
}

struct Flood {
    levels: Vec<f64>,
    parents: Vec<u32>,
    order: Vec<u32>,
}

fn priority_flood(
    width: usize,
    height: usize,
    mut levels: Vec<f64>,
    outlets: impl IntoIterator<Item = usize>,
) -> Flood {
    let mut parents = vec![UNVISITED; levels.len()];
    let mut frontier = BinaryHeap::new();
    let mut order = Vec::with_capacity(levels.len());
    for index in outlets {
        if parents[index] == UNVISITED {
            parents[index] = NO_CELL;
            frontier.push(Reverse(FloodCell {
                height: levels[index],
                index,
            }));
        }
    }
    while let Some(Reverse(cell)) = frontier.pop() {
        order.push(cell.index as u32);
        for neighbor in neighbors(cell.index, width, height) {
            if parents[neighbor] != UNVISITED {
                continue;
            }
            parents[neighbor] = cell.index as u32;
            let spill = levels[neighbor].max(cell.height);
            levels[neighbor] = spill;
            frontier.push(Reverse(FloodCell {
                height: spill,
                index: neighbor,
            }));
        }
    }
    Flood {
        levels,
        parents,
        order,
    }
}

fn boundary(index: usize, width: usize, height: usize) -> bool {
    let (x, z) = (index % width, index / width);
    x == 0 || x + 1 == width || z == 0 || z + 1 == height
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

    for &index in outlets {
        if index >= count {
            return Err("outlet index is outside the grid");
        }
        if !boundary(index, width, height) {
            return Err("outlet must be on the boundary");
        }
    }
    let Flood {
        levels,
        parents,
        order,
    } = priority_flood(width, height, elevation.to_vec(), outlets.iter().copied());
    let downstream: Vec<_> = parents.iter().copied().map(cell_index).collect();
    let mut contributing_cells = vec![1; count];
    for index in order.into_iter().rev().map(|index| index as usize) {
        if let Some(next) = downstream[index] {
            contributing_cells[next] += contributing_cells[index];
        }
    }
    let (lake_at, basins) = filled_basins(width, height, &levels, &parents, |i| elevation[i], None);
    Ok(Drainage {
        conditioned_height: levels,
        downstream,
        contributing_cells,
        lake_at: lake_at.into_iter().map(cell_index).collect(),
        lakes: basins.into_iter().map(|basin| basin.lake).collect(),
    })
}

struct FilledBasin {
    lake: Lake,
    cells: u32,
    max_depth: f64,
    depth_sum: f64,
    temperature_sum: f64,
}

fn filled_basins(
    width: usize,
    height: usize,
    levels: &[f64],
    parents: &[u32],
    elevation: impl Fn(usize) -> f64,
    temperature_c: Option<&[f32]>,
) -> (Vec<u32>, Vec<FilledBasin>) {
    let mut lake_at = vec![NO_CELL; levels.len()];
    let mut basins = Vec::new();
    let mut pending = Vec::new();
    for first in 0..levels.len() {
        let level = levels[first];
        if lake_at[first] != NO_CELL || elevation(first) >= level {
            continue;
        }
        let id = basins.len() as u32;
        let submerged = |index: usize| levels[index] == level && elevation(index) < level;
        lake_at[first] = id;
        pending.push(first);
        let mut spill = None;
        let mut cells = 0;
        let mut max_depth = 0.0_f64;
        let mut depth_sum = 0.0;
        let mut temperature_sum = 0.0;
        while let Some(cell) = pending.pop() {
            let depth = level - elevation(cell);
            cells += 1;
            max_depth = max_depth.max(depth);
            depth_sum += depth;
            if let Some(temperature) = temperature_c {
                temperature_sum += f64::from(temperature[cell]);
            }
            if let Some(to) = cell_index(parents[cell]).filter(|&to| !submerged(to)) {
                let candidate = (to, cell);
                spill = Some(spill.map_or(candidate, |current| candidate.min(current)));
            }
            for neighbor in neighbors(cell, width, height) {
                if lake_at[neighbor] == NO_CELL && submerged(neighbor) {
                    lake_at[neighbor] = id;
                    pending.push(neighbor);
                }
            }
        }
        let (spill_to, spill_from) = spill.expect("filled basin must drain to its rim");
        basins.push(FilledBasin {
            lake: Lake {
                level,
                spill_from,
                spill_to,
            },
            cells,
            max_depth,
            depth_sum,
            temperature_sum,
        });
    }
    (lake_at, basins)
}

/// Analyze a padded regional grid. Perimeter cells and elevations at or below
/// sea level are outlets. Runoff splits between strictly lower cardinal neighbors
/// in proportion to their slope, with one Priority-Flood parent on an exact flat.
/// The reverse flood order is topological for both kinds of edge.
///
/// The caller owns the padding and crop. Independent region boundaries do not
/// share downstream outlets or prove continuity across watersheds.
pub fn drain_region(
    width: usize,
    height: usize,
    spacing_m: f64,
    sea_level: f32,
    elevation_m: &[f32],
    precipitation_mm: &[f32],
    temperature_c: Option<&[f32]>,
) -> Result<RegionalDrainage, &'static str> {
    let count = width
        .checked_mul(height)
        .ok_or("grid dimensions overflow")?;
    if width < 2
        || height < 2
        || count > MAX_REGIONAL_CELLS
        || count != elevation_m.len()
        || count != precipitation_mm.len()
        || temperature_c.is_some_and(|temperature| temperature.len() != count)
    {
        return Err("invalid regional drainage grid dimensions");
    }
    let area_m2 = spacing_m * spacing_m;
    if spacing_m <= 0.0
        || area_m2 == 0.0
        || !(area_m2 * count as f64 * (2.0 * f64::from(f32::MAX))).is_finite()
        || !sea_level.is_finite()
        || elevation_m.iter().any(|height| !height.is_finite())
    {
        return Err("invalid regional drainage elevation or spacing");
    }
    if precipitation_mm
        .iter()
        .any(|rain| !rain.is_finite() || *rain < 0.0)
        || temperature_c.is_some_and(|samples| samples.iter().any(|temp| !temp.is_finite()))
    {
        return Err("invalid regional drainage climate");
    }
    let outlets = (0..count).filter(|&i| boundary(i, width, height) || elevation_m[i] <= sea_level);
    let Flood {
        levels,
        mut parents,
        order,
    } = priority_flood(
        width,
        height,
        elevation_m.iter().copied().map(f64::from).collect(),
        outlets,
    );
    let (lake_at, basins) = filled_basins(
        width,
        height,
        &levels,
        &parents,
        |i| f64::from(elevation_m[i]),
        temperature_c,
    );
    let mut runoff_m3_per_year: Vec<_> = precipitation_mm
        .iter()
        .map(|&mm| f64::from(mm) * area_m2 / 1000.0)
        .collect();
    let mut flow_direction = vec![[0.0; 2]; count];
    let direction = |from: usize, to: usize| {
        [
            (to % width) as f64 - (from % width) as f64,
            (to / width) as f64 - (from / width) as f64,
        ]
    };
    for index in order.into_iter().rev().map(|index| index as usize) {
        let Some(parent) = cell_index(parents[index]) else {
            continue;
        };
        let mut lower = [(0, 0.0); 4];
        let mut used = 0;
        let mut total = 0.0;
        for neighbor in neighbors(index, width, height) {
            let drop = levels[index] - levels[neighbor];
            if drop > 0.0 {
                lower[used] = (neighbor, drop);
                used += 1;
                total += drop;
            }
        }
        if used == 0 {
            runoff_m3_per_year[parent] += runoff_m3_per_year[index];
            flow_direction[index] = direction(index, parent).map(|component| component as f32);
            continue;
        }
        let mut strongest = 0.0;
        let mut vector = [0.0; 2];
        for &(neighbor, drop) in &lower[..used] {
            let weight = drop / total;
            runoff_m3_per_year[neighbor] += runoff_m3_per_year[index] * weight;
            let [dx, dz] = direction(index, neighbor);
            vector[0] += dx * weight;
            vector[1] += dz * weight;
            if drop > strongest {
                parents[index] = neighbor as u32;
                strongest = drop;
            }
        }
        flow_direction[index] = vector.map(|component| component as f32);
    }
    let lakes = basins
        .into_iter()
        .map(|basin| LakeBasin {
            level_m: basin.lake.level as f32,
            spill_from: basin.lake.spill_from,
            spill_to: basin.lake.spill_to,
            area_m2: f64::from(basin.cells) * area_m2,
            max_depth_m: basin.max_depth,
            volume_m3: basin.depth_sum * area_m2,
            frozen: basin.temperature_sum < 0.0,
        })
        .collect();
    Ok(RegionalDrainage {
        water_surface_m: levels.into_iter().map(|height| height as f32).collect(),
        runoff_m3_per_year,
        flow_direction,
        lakes,
        outlets_or_receivers: parents,
        lake_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regional_rainfall_splits_by_slope_with_conservative_volume() {
        let elevation = [10., 10., 10., 4., 10., 6., 10., 10., 10.];
        let rainfall = [0., 0., 0., 0., 1000., 0., 0., 0., 0.];
        let result = drain_region(3, 3, 2.0, 0.0, &elevation, &rainfall, None).unwrap();
        assert_eq!(result.downstream(4), Some(3));
        assert!((result.runoff_m3_per_year[3] - 2.4).abs() < 1.0e-12);
        assert!((result.runoff_m3_per_year[5] - 1.6).abs() < 1.0e-12);
        assert_eq!(result.flow_direction[4], [-0.2, 0.0]);
    }

    #[test]
    fn regional_lake_levels_and_frozen_area_follow_the_filled_basin() {
        let elevation = [
            9., 9., 0., 9., 9., 9., 9., 4., 9., 9., 9., 1., 1., 1., 9., 9., 1., 1., 1., 9., 9., 9.,
            9., 9., 9.,
        ];
        let rain = [750.; 25];
        let solve = || drain_region(5, 5, 2.0, 0.0, &elevation, &rain, Some(&[-3.; 25])).unwrap();
        let result = solve();
        assert_eq!(result, solve());
        assert_eq!(result.lakes.len(), 1);
        let lake = &result.lakes[0];
        assert_eq!(lake.level_m, 4.0);
        assert_eq!(lake.area_m2, 24.0);
        assert_eq!(lake.volume_m3, 72.0);
        assert_eq!(lake.max_depth_m, 3.0);
        assert_eq!(lake.spill_to, 7);
        assert!(lake.frozen);
        assert_eq!(result.lake_index(lake.spill_from), Some(0));
        assert_eq!(result.lake_index(lake.spill_to), None);
        for start in 0..25 {
            let mut cell = start;
            let mut steps = 0;
            while let Some(next) = result.downstream(cell) {
                assert!(result.water_surface_m[next] <= result.water_surface_m[cell]);
                assert!(neighbors(cell, 5, 5).any(|neighbor| neighbor == next));
                steps += 1;
                assert!(steps < 25);
                cell = next;
            }
            assert!(boundary(cell, 5, 5));
        }
        let outlet_volume: f64 = (0..25)
            .filter(|&i| result.downstream(i).is_none())
            .map(|i| result.runoff_m3_per_year[i])
            .sum();
        assert!((outlet_volume - 75.0).abs() < 1.0e-12);
    }

    #[test]
    fn regional_ocean_cells_are_outlets_and_invalid_climate_fails() {
        let elevation = [8., 8., 8., 8., -1., 8., 8., 8., 8.];
        let result = drain_region(3, 3, 4.0, 0.0, &elevation, &[300.; 9], None).unwrap();
        assert_eq!(result.downstream(4), None);
        assert!(result.lakes.is_empty());
        assert_eq!(result.water_surface_m[4], -1.0);
        assert!(drain_region(3, 3, 4.0, 0.0, &elevation, &[-1.; 9], None).is_err());
        assert!(drain_region(3, 3, 4.0, 0.0, &elevation, &[1.; 9], Some(&[f32::NAN; 9])).is_err());
        assert!(drain_region(3, 3, 0.0, 0.0, &elevation, &[1.; 9], None).is_err());
    }

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
