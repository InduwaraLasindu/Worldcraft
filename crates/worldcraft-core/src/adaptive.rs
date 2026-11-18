//! Measured, machine-local quality policy. Never changes generation or saved edits.
#[derive(Clone, Debug)]
pub struct Controller {
    pub tier: u8,
    pub ceiling: u8,
    pub frame_ms: f64,
    pub limited: bool,
    elapsed: f64,
    good: f64,
    cooldown: f64,
    samples: Vec<f64>,
    work_samples: Vec<f64>,
}
impl Default for Controller {
    fn default() -> Self {
        Self {
            tier: 1,
            ceiling: 4,
            frame_ms: 0.0,
            limited: false,
            elapsed: 0.0,
            good: 0.0,
            cooldown: 2.0,
            samples: Vec::with_capacity(256),
            work_samples: Vec::with_capacity(256),
        }
    }
}
pub fn percentile90(samples: &mut [f64]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    samples.sort_by(f64::total_cmp);
    samples[((samples.len() - 1) * 9).div_ceil(10)]
}
/// Baseline uses an actual rendered probe, bounded CPU terrain sampling and available RAM.
pub fn initial_tier(
    frame_ms: f64,
    cpu_ms: f64,
    total_gib: f64,
    available_gib: f64,
    software: bool,
) -> u8 {
    if software || total_gib < 6.0 || available_gib < 2.0 {
        return 0;
    }
    if !frame_ms.is_finite() || frame_ms <= 0.0 || frame_ms > 35.0 || cpu_ms > 100.0 {
        return 1;
    }
    if total_gib >= 15.0 && available_gib >= 6.0 && frame_ms < 19.0 && cpu_ms < 40.0 {
        return 4;
    }
    if frame_ms < 26.0 && available_gib >= 3.0 {
        3
    } else {
        2
    }
}
impl Controller {
    pub fn reset_window(&mut self) {
        self.samples.clear();
        self.work_samples.clear();
        self.elapsed = 0.0;
        self.good = 0.0;
        self.cooldown = 2.0;
    }
    /// Wall-clock frame intervals include presentation, streaming, rendering and pacing.
    /// Slow changes and asymmetric recovery avoid a quality oscillation at the threshold.
    pub fn observe(&mut self, seconds: f64, target_fps: f64) -> bool {
        self.observe_paced(seconds, 0.0, target_fps)
    }
    /// Keep real presentation intervals for downgrade decisions. Only the
    /// application's deliberate limiter sleep can count as recoverable headroom.
    pub fn observe_paced(&mut self, seconds: f64, limiter_sleep: f64, target_fps: f64) -> bool {
        if !seconds.is_finite() || seconds <= 0.0 {
            return false;
        }
        self.cooldown = (self.cooldown - seconds).max(0.0);
        if self.cooldown > 0.0 {
            return false;
        }
        self.elapsed += seconds;
        if self.samples.len() < 512 {
            self.samples.push(seconds * 1000.0);
            self.work_samples
                .push((seconds - limiter_sleep.clamp(0.0, seconds)) * 1000.0);
        }
        if self.elapsed < 2.0 || self.samples.len() < 12 {
            return false;
        }
        self.frame_ms = percentile90(&mut self.samples);
        let work_ms = percentile90(&mut self.work_samples);
        self.work_samples.clear();
        let budget = 1000.0 / target_fps.clamp(15.0, 120.0);
        let elapsed = self.elapsed;
        self.elapsed = 0.0;
        self.samples.clear();
        self.limited = self.tier == 0 && self.frame_ms > budget * 1.02;
        if self.frame_ms > budget * 1.02 {
            self.good = 0.0;
            if self.tier > 0 {
                self.tier -= 1;
                self.cooldown = 3.0;
                return true;
            }
        } else if work_ms < budget * 0.72 {
            self.good += elapsed;
            if self.good >= 12.0 && self.tier < self.ceiling {
                self.tier += 1;
                self.good = 0.0;
                self.cooldown = 4.0;
                return true;
            }
        } else {
            self.good = 0.0;
        }
        false
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn high_refresh_targets_and_limiter_headroom_use_distinct_timings() {
        let mut controller = Controller {
            tier: 2,
            ..Default::default()
        };
        for _ in 0..700 {
            controller.observe_paced(1.0 / 70.0, 0.0, 120.0);
        }
        assert!(controller.tier < 2);
        let mut capped = Controller {
            tier: 1,
            ceiling: 2,
            ..Default::default()
        };
        for _ in 0..1600 {
            capped.observe_paced(1.0 / 60.0, 0.009, 60.0);
        }
        assert_eq!(capped.tier, 2);
        let mut busy = Controller {
            tier: 1,
            ceiling: 2,
            ..Default::default()
        };
        for _ in 0..1600 {
            busy.observe_paced(1.0 / 60.0, 0.0, 60.0);
        }
        assert_eq!(busy.tier, 1);
    }

    #[test]
    fn measured_probe_is_conservative_and_rewards_fast_hardware() {
        assert_eq!(initial_tier(12.0, 15.0, 32.0, 24.0, false), 4);
        assert_eq!(initial_tier(50.0, 15.0, 32.0, 24.0, false), 1);
        assert_eq!(initial_tier(12.0, 15.0, 8.0, 1.0, false), 0);
        assert_eq!(initial_tier(12.0, 15.0, 32.0, 24.0, true), 0);
        assert_eq!(initial_tier(f64::NAN, 1.0, 32.0, 24.0, false), 1);
    }
    #[test]
    fn sustained_slow_frames_reduce_quality_and_floor_reports_limit() {
        let mut c = Controller {
            tier: 4,
            ..Default::default()
        };
        for _ in 0..700 {
            c.observe(0.05, 30.0);
        }
        assert_eq!(c.tier, 0);
        assert!(c.limited);
        // One fast frame cannot immediately increase quality.
        c.observe(0.01, 30.0);
        assert_eq!(c.tier, 0);
    }
    #[test]
    fn upgrades_need_sustained_headroom_and_respect_ceiling() {
        let mut c = Controller {
            tier: 1,
            ceiling: 2,
            ..Default::default()
        };
        for _ in 0..600 {
            c.observe(0.016, 30.0);
        }
        assert_eq!(c.tier, 1);
        for _ in 0..2400 {
            c.observe(0.016, 30.0);
        }
        assert_eq!(c.tier, 2);
        c.reset_window();
        assert!(c.samples.is_empty());
    }
    #[test]
    fn sustained_29_fps_is_not_treated_as_meeting_the_30_fps_target() {
        let mut c = Controller {
            tier: 3,
            ..Default::default()
        };
        for _ in 0..300 {
            c.observe(1.0 / 29.0, 30.0);
        }
        assert!(c.tier < 3);
        let mut floor = Controller {
            tier: 0,
            ..Default::default()
        };
        for _ in 0..300 {
            floor.observe(1.0 / 29.0, 30.0);
        }
        assert!(floor.limited);
    }
}
