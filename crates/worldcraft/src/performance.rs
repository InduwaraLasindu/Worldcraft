//! Machine-local budgets. These never change a world's seed, terrain or durable edits.
use crate::{
    scene::{MainCamera, UiCamera},
    session::*,
};
use bevy::{
    prelude::*,
    window::PrimaryWindow,
    winit::{UpdateMode, WinitSettings},
};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use worldcraft_core::hardware::{self, Memory, GIB};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    #[default]
    Auto,
    Eco,
    Balanced,
    Performance,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub mode: Mode,
    pub fps: u32,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            mode: Mode::Auto,
            fps: 60,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Budget {
    pub jobs: usize,
    pub uploads: usize,
    pub radius: i64,
    pub interval: Duration,
    pub ai_threads: usize,
    pub far: bool,
}
#[derive(Resource)]
pub struct Performance {
    pub preferences: Preferences,
    pub memory: Option<Memory>,
    pub cpu_threads: usize,
    pub weak_gpu: bool,
    pub pressure: bool,
    pub suspended: bool,
    pub save_error: String,
    smoke: bool,
    audit: bool,
    sampled: Instant,
    frame: Instant,
    saved: Preferences,
    save_task: Option<bevy::tasks::Task<Result<(), String>>>,
}
impl Performance {
    pub fn load(smoke: bool) -> Self {
        let mut preferences: Preferences = if smoke {
            Preferences::default()
        } else {
            std::fs::read(crate::storage::data_dir().join("performance.json"))
                .ok()
                .and_then(|b| serde_json::from_slice(&b).ok())
                .unwrap_or_default()
        };
        preferences.fps = preferences.fps.clamp(15, 120);
        Self {
            preferences,
            saved: preferences,
            memory: hardware::memory(),
            cpu_threads: std::thread::available_parallelism().map_or(1, usize::from),
            weak_gpu: false,
            pressure: false,
            suspended: false,
            smoke,
            audit: std::env::args().any(|a| a == "--eco-smoke-test"),
            sampled: Instant::now(),
            frame: Instant::now(),
            save_task: None,
            save_error: String::new(),
        }
    }
    pub fn mode(&self) -> Mode {
        if self.smoke {
            return Mode::Performance;
        }
        if self.pressure {
            return Mode::Eco;
        }
        if self.preferences.mode == Mode::Auto {
            if self.weak_gpu || hardware::recommend_eco(self.cpu_threads, self.memory) {
                Mode::Eco
            } else {
                Mode::Balanced
            }
        } else {
            self.preferences.mode
        }
    }
    pub fn budget(&self) -> Budget {
        match self.mode() {
            Mode::Eco => Budget {
                jobs: 1,
                uploads: 1,
                radius: 2,
                interval: Duration::from_millis(150),
                ai_threads: 1,
                far: false,
            },
            Mode::Balanced | Mode::Auto => Budget {
                jobs: 2,
                uploads: 1,
                radius: 6,
                interval: Duration::from_millis(35),
                ai_threads: 2,
                far: true,
            },
            Mode::Performance => Budget {
                jobs: 4,
                uploads: 2,
                radius: 12,
                interval: Duration::ZERO,
                ai_threads: 4,
                far: true,
            },
        }
    }
    pub fn quality(&self, quality: Quality) -> Quality {
        if self.mode() == Mode::Eco {
            Quality::Low
        } else {
            quality
        }
    }
    pub fn task_pools(&self) -> bevy::app::TaskPoolOptions {
        let mut pools = bevy::app::TaskPoolOptions::default();
        let available = self.cpu_threads.saturating_sub(2).clamp(1, 6);
        pools.max_total_threads = available;
        pools.io.max_threads = 1;
        pools.async_compute.max_threads = if self.mode() == Mode::Eco { 1 } else { 2 };
        pools.compute.max_threads = available.saturating_sub(2).max(1);
        pools
    }
    fn frame_interval(&self, home: bool, focused: bool) -> Duration {
        if !focused {
            Duration::from_millis(200)
        } else if home {
            Duration::from_millis(50)
        } else {
            Duration::from_secs_f64(
                1.0 / f64::from(
                    self.preferences
                        .fps
                        .min(if self.mode() == Mode::Eco { 30 } else { 120 })
                        .clamp(15, 120),
                ),
            )
        }
    }
    pub fn ai_progress(&self) -> std::sync::Arc<worldcraft_ml::Progress> {
        let progress = std::sync::Arc::new(worldcraft_ml::Progress::default());
        progress.threads.store(
            self.budget().ai_threads,
            std::sync::atomic::Ordering::Relaxed,
        );
        progress.pause_ms.store(
            if self.mode() == Mode::Eco { 40 } else { 0 },
            std::sync::atomic::Ordering::Relaxed,
        );
        progress
    }
}
pub fn frame_start(mut limits: ResMut<Performance>) {
    limits.frame = Instant::now();
}
#[allow(clippy::too_many_arguments)]
pub fn update(
    mut limits: ResMut<Performance>,
    ui: Res<UiState>,
    session: Res<Session>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut settings: ResMut<WinitSettings>,
    mut cameras: Query<&mut Camera, With<MainCamera>>,
    mut ui_cameras: Query<&mut Camera, (With<UiCamera>, Without<MainCamera>)>,
    disk: Res<crate::storage::Disk>,
    mut idle_frames: Local<u32>,
) {
    use bevy::tasks::{block_on, futures_lite::future, IoTaskPool};
    let focused = limits.audit
        || windows
            .iter()
            .any(|w| w.focused && !w.physical_size().cmpeq(UVec2::ZERO).any());
    limits.suspended = !limits.smoke && (!focused || ui.home);
    if limits.sampled.elapsed() >= Duration::from_secs(2) {
        limits.sampled = Instant::now();
        limits.memory = hardware::memory();
        if let Some(m) = limits.memory {
            // Hysteresis avoids repeatedly changing budgets around one threshold.
            limits.pressure = m.available < if limits.pressure { GIB * 3 / 2 } else { GIB };
        }
    }
    for mut camera in &mut cameras {
        camera.is_active = session.world.is_some() && !ui.home && (focused || limits.smoke);
    }
    for mut camera in &mut ui_cameras {
        camera.clear_color = if ui.home {
            ClearColorConfig::Custom(Color::srgb(0.07, 0.10, 0.11))
        } else {
            ClearColorConfig::None
        };
    }
    if std::env::args().any(|a| a == "--idle-audit") {
        *idle_frames += 1;
        if *idle_frames == 8 {
            assert!(ui.home && session.world.is_none());
            assert!(cameras.iter().all(|camera| !camera.is_active));
            std::fs::write(
                "idle-state.json",
                "{\"home\":true,\"world_camera_active\":false}",
            )
            .expect("idle state report");
        }
    }
    if !limits.smoke {
        settings.focused_mode = if ui.home {
            UpdateMode::reactive_low_power(Duration::from_millis(if disk.busy() {
                100
            } else {
                250
            }))
        } else {
            UpdateMode::Continuous
        };
        settings.unfocused_mode = UpdateMode::reactive_low_power(Duration::from_millis(500));
    }
    if let Some(result) = limits
        .save_task
        .as_mut()
        .and_then(|t| block_on(future::poll_once(t)))
    {
        limits.save_task = None;
        limits.save_error = result.err().unwrap_or_default();
    }
    if !limits.smoke && limits.saved != limits.preferences && limits.save_task.is_none() {
        let preferences = limits.preferences;
        limits.saved = preferences;
        limits.save_task = Some(IoTaskPool::get().spawn(async move {
            let folder = crate::storage::data_dir();
            std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
            let temporary = folder.join("performance.tmp");
            std::fs::write(
                &temporary,
                serde_json::to_vec(&preferences).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            // Windows rename cannot replace an existing file; this tiny preference file is disposable.
            std::fs::copy(&temporary, folder.join("performance.json"))
                .map_err(|e| e.to_string())?;
            let _ = std::fs::remove_file(temporary);
            Ok(())
        }));
    }
}
pub fn pace(
    limits: Res<Performance>,
    ui: Res<UiState>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    if limits.smoke {
        return;
    }
    let interval = limits.frame_interval(ui.home, windows.iter().any(|w| w.focused));
    if let Some(remaining) = interval.checked_sub(limits.frame.elapsed()) {
        std::thread::sleep(remaining);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn eco_and_pressure_bound_work_without_changing_worlds() {
        let mut p = Performance::load(true);
        p.smoke = false;
        p.preferences = Preferences {
            mode: Mode::Eco,
            fps: 120,
        };
        assert_eq!(p.budget().jobs, 1);
        assert_eq!(p.budget().radius, 2);
        assert!(!p.budget().far);
        assert_eq!(p.quality(Quality::High), Quality::Low);
        assert!(p.frame_interval(false, true) >= Duration::from_secs_f64(1.0 / 30.0));
        p.preferences.mode = Mode::Performance;
        p.pressure = true;
        assert_eq!(p.budget().jobs, 1);
        assert_eq!(p.mode(), Mode::Eco);
        p.pressure = false;
        assert_eq!(p.budget().jobs, 4);
    }
    #[test]
    fn home_and_unfocused_are_always_throttled() {
        let p = Performance::load(true);
        assert_eq!(p.frame_interval(true, true), Duration::from_millis(50));
        assert_eq!(p.frame_interval(false, false), Duration::from_millis(200));
    }
}
