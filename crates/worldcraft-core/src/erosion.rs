//! Conservative four-neighbor thermal transport. All fluxes read the previous
//! iteration, so traversal order does not change the result.
pub fn thermal_relax(heights: &mut [f64], width: usize, spacing: f64, iterations: usize) {
    assert!(width >= 2 && heights.len() == width * width);
    assert!(spacing.is_finite() && spacing > 0.0 && heights.iter().all(|v| v.is_finite()));
    let mut flux = vec![[0.0; 4]; heights.len()];
    let mut next = vec![0.0; heights.len()];
    let talus = spacing * 0.60;
    for _ in 0..iterations {
        for z in 0..width {
            for x in 0..width {
                let i = z * width + x;
                let neighbors = [
                    (x + 1 < width).then_some(i + 1),
                    (x > 0).then(|| i - 1),
                    (z + 1 < width).then_some(i + width),
                    (z > 0).then(|| i - width),
                ];
                for (d, j) in neighbors.into_iter().enumerate() {
                    flux[i][d] =
                        j.map_or(0.0, |j| (heights[i] - heights[j] - talus).max(0.0) * 0.045);
                }
            }
        }
        for z in 0..width {
            for x in 0..width {
                let i = z * width + x;
                let mut h = heights[i];
                for outgoing in flux[i] {
                    h -= outgoing;
                }
                if x > 0 {
                    h += flux[i - 1][0];
                }
                if x + 1 < width {
                    h += flux[i + 1][1];
                }
                if z > 0 {
                    h += flux[i - width][2];
                }
                if z + 1 < width {
                    h += flux[i + width][3];
                }
                next[i] = h;
            }
        }
        heights.copy_from_slice(&next);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sediment_is_conserved_and_peak_relaxes() {
        let mut h = vec![0.0; 21 * 21];
        h[10 * 21 + 10] = 200.0;
        let before = h.iter().sum::<f64>();
        thermal_relax(&mut h, 21, 8.0, 12);
        assert!((h.iter().sum::<f64>() - before).abs() < 1e-9);
        assert!(h[10 * 21 + 10] < 100.0);
        assert!(h[10 * 21 + 11] > 0.0);
        assert!(h.iter().all(|v| v.is_finite() && *v >= 0.0));
    }
    #[test]
    fn stable_talus_plane_is_unchanged() {
        let mut h: Vec<_> = (0..11)
            .flat_map(|z| (0..11).map(move |x| x as f64 * 2.0 + z as f64))
            .collect();
        let original = h.clone();
        thermal_relax(&mut h, 11, 8.0, 8);
        assert_eq!(original, h);
    }
}
