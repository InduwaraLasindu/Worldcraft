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
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub mode: Mode,
    pub fps: u32,
    pub auto_target_fps: u32,
    pub render: crate::render_options::RenderOptions,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            mode: Mode::Auto,
            fps: 60,
            auto_target_fps: 30,
            render: Default::default(),
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
    pub software_gpu: bool,
    pub controller: worldcraft_core::adaptive::Controller,
    pub calibrating: bool,
    pub calibration_status: String,
    interval_start: Instant,
    pub frame_seconds: f64,
    adapting_to: Option<f64>,
    pub effective_target: f64,
    limiter_sleep: Duration,
    last_paced: Instant,
    pub pressure: bool,
    pub suspended: bool,
    pub save_error: String,
    smoke: bool,
    pub audit: bool,
    sampled: Instant,
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
        preferences.auto_target_fps = preferences.auto_target_fps.clamp(15, 120);
        preferences.render.sanitize();
        Self {
            preferences,
            saved: preferences,
            memory: hardware::memory(),
            cpu_threads: std::thread::available_parallelism().map_or(1, usize::from),
            software_gpu: false,
            controller: worldcraft_core::adaptive::Controller::default(),
            calibrating: !smoke
                && preferences.mode == Mode::Auto
                && !std::env::args().any(|a| a == "--idle-audit"),
            calibration_status: if preferences.mode == Mode::Auto {
                "Checking hardware with a short launch scene…"
            } else {
                "Manual settings selected; no launch calibration."
            }
            .into(),
            interval_start: Instant::now(),
            frame_seconds: 0.0,
            adapting_to: None,
            effective_target: f64::from(preferences.auto_target_fps.min(preferences.fps)),
            limiter_sleep: Duration::ZERO,
            last_paced: Instant::now(),
            pressure: false,
            suspended: false,
            smoke,
            audit: std::env::args().any(|a| {
                a == "--eco-smoke-test" || a == "--auto-smoke-test" || a == "--calibration-audit"
            }),
            sampled: Instant::now(),
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
            match self.controller.tier {
                0 | 1 => Mode::Eco,
                2 | 3 => Mode::Balanced,
                _ => Mode::Performance,
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
    pub fn radius(&self, requested: i64) -> i64 {
        if self.preferences.mode == Mode::Auto && !self.smoke {
            match self.controller.tier {
                0 | 1 => 2,
                2 => 4,
                3 => 6,
                _ => 9,
            }
            .min(self.budget().radius)
        } else {
            requested.clamp(2, self.budget().radius)
        }
    }
    pub fn render_scale(&self) -> f32 {
        (self.budget_render_scale() * self.preferences.render.resolution_scale).clamp(0.5, 1.0)
    }
    fn budget_render_scale(&self) -> f32 {
        if self.smoke {
            return 1.0;
        }
        if self.pressure {
            return 0.5;
        }
        if self.preferences.mode == Mode::Auto {
            match self.controller.tier {
                0 => 0.5,
                1 => 0.67,
                2 => 0.85,
                _ => 1.0,
            }
        } else if self.mode() == Mode::Eco {
            0.67
        } else {
            1.0
        }
    }
    pub fn quality(&self, quality: Quality) -> Quality {
        if !self.smoke && self.preferences.mode == Mode::Auto && !self.pressure {
            return match self.controller.tier {
                0..=2 => Quality::Low,
                3 => Quality::Balanced,
                _ => Quality::High,
            };
        }
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
        pools.async_compute.max_threads = if self.cpu_threads <= 4 { 1 } else { 2 };
        pools.compute.max_threads = available.saturating_sub(2).max(1);
        pools
    }
    fn auto_target(&self, refresh_millihz: Option<u32>) -> f64 {
        let cap = self.preferences.auto_target_fps.min(self.preferences.fps);
        let mut target = f64::from(if self.pressure { cap.min(30) } else { cap });
        if self.preferences.render.vsync {
            if let Some(hz) = refresh_millihz.filter(|hz| *hz >= 15_000) {
                target = target.min(f64::from(hz) / 1000.0);
            }
        }
        target
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
                        .min(if self.preferences.mode == Mode::Eco || self.pressure {
                            30
                        } else {
                            120
                        })
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
    let now = Instant::now();
    limits.frame_seconds = now.duration_since(limits.interval_start).as_secs_f64();
    limits.interval_start = now;
}
#[allow(clippy::too_many_arguments)]
pub fn update(
    mut limits: ResMut<Performance>,
    ui: Res<UiState>,
    session: Res<Session>,
    launch: Res<Launch>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut settings: ResMut<WinitSettings>,
    mut cameras: Query<&mut Camera, With<MainCamera>>,
    mut ui_cameras: Query<&mut Camera, (With<UiCamera>, Without<MainCamera>)>,
    disk: Res<crate::storage::Disk>,
    screen: Query<&bevy::window::OnMonitor, With<PrimaryWindow>>,
    monitors: Query<&bevy::window::Monitor>,
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
    let adapting = !limits.smoke
        && !limits.calibrating
        && !limits.suspended
        && limits.preferences.mode == Mode::Auto;
    let hz = screen
        .iter()
        .next()
        .and_then(|on| monitors.get(on.0).ok())
        .and_then(|m| m.refresh_rate_millihertz);
    let target = limits.auto_target(hz);
    limits.effective_target = target;
    let adapting_to = adapting.then_some(target);
    if adapting_to != limits.adapting_to {
        limits.controller.reset_window();
        limits.adapting_to = adapting_to;
    }
    if adapting {
        let seconds = limits.frame_seconds;
        let slept = limits.limiter_sleep.as_secs_f64();
        if limits.controller.observe_paced(seconds, slept, target) {
            info!(
                "Automatic quality adjusted to tier {}",
                limits.controller.tier
            );
        }
    }
    for mut camera in &mut cameras {
        camera.is_active =
            (session.world.is_some() || launch.ray_smoke) && !ui.home && (focused || limits.smoke);
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
        settings.focused_mode = if ui.home && !limits.calibrating {
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
    mut limits: ResMut<Performance>,
    ui: Res<UiState>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    limits.limiter_sleep = Duration::ZERO;
    let now = Instant::now();
    if limits.smoke || limits.calibrating {
        limits.last_paced = now;
        return;
    }
    let interval = limits.frame_interval(ui.home, windows.iter().any(|w| w.focused));
    // Last-to-Last includes rendering/presentation after the previous app
    // update. Sleeping against First-to-Last added that time on top of the cap.
    if let Some(remaining) = interval.checked_sub(now.duration_since(limits.last_paced)) {
        std::thread::sleep(remaining);
        let after = Instant::now();
        limits.limiter_sleep = after.duration_since(now);
        limits.last_paced = after;
    } else {
        limits.last_paced = now;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn auto_target_respects_old_settings_the_frame_cap_display_and_memory_pressure() {
        let mut p = Performance::load(true);
        p.preferences = serde_json::from_str(r#"{"fps":90}"#).unwrap();
        assert_eq!(p.auto_target(None), 30.0);
        p.preferences.auto_target_fps = 120;
        p.preferences.render.vsync = true;
        assert_eq!(p.auto_target(Some(59_940)), 59.94);
        assert_eq!(p.auto_target(Some(0)), 90.0);
        p.preferences.render.vsync = false;
        assert_eq!(p.auto_target(Some(59_940)), 90.0);
        p.pressure = true;
        assert_eq!(p.auto_target(None), 30.0);
    }

    #[test]
    fn world_and_standalone_ray_fixture_use_the_correct_camera_lifecycle() {
        let mut app = App::new();
        let mut performance = Performance::load(true);
        performance.smoke = false;
        app.add_plugins(MinimalPlugins)
            .insert_resource(performance)
            .insert_resource(Launch {
                smoke: false,
                ray_smoke: false,
            })
            .init_resource::<Session>()
            .init_resource::<UiState>()
            .init_resource::<crate::storage::Disk>()
            .init_resource::<WinitSettings>()
            .add_systems(Update, update);
        let camera = app.world_mut().spawn((MainCamera, Camera::default())).id();
        app.world_mut().spawn((UiCamera, Camera::default()));
        let window = app
            .world_mut()
            .spawn((Window::default(), PrimaryWindow))
            .id();
        app.world_mut().resource_mut::<UiState>().home = false;
        app.world_mut().get_mut::<Window>(window).unwrap().focused = true;
        app.update();
        assert!(!app.world().get::<Camera>(camera).unwrap().is_active);
        app.world_mut().resource_mut::<Launch>().ray_smoke = true;
        app.update();
        assert!(app.world().get::<Camera>(camera).unwrap().is_active);
        app.world_mut().resource_mut::<Launch>().ray_smoke = false;
        app.world_mut().resource_mut::<Session>().world = Some(worldcraft_core::WorldFile::new(
            "Camera lifecycle".into(),
            worldcraft_core::GeneratorSettings::default(),
        ));
        app.update();
        assert!(app.world().get::<Camera>(camera).unwrap().is_active);
        app.world_mut().resource_mut::<UiState>().home = true;
        app.update();
        assert!(!app.world().get::<Camera>(camera).unwrap().is_active);
        app.world_mut().resource_mut::<UiState>().home = false;
        app.world_mut().get_mut::<Window>(window).unwrap().focused = false;
        app.update();
        assert!(!app.world().get::<Camera>(camera).unwrap().is_active);
    }
    #[test]
    fn eco_and_pressure_bound_work_without_changing_worlds() {
        let mut p = Performance::load(true);
        p.smoke = false;
        p.preferences = Preferences {
            mode: Mode::Eco,
            fps: 120,
            ..Default::default()
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
    #[test]
    fn automatic_budget_scales_but_manual_choice_is_preserved() {
        let mut p = Performance::load(true);
        p.smoke = false;
        p.preferences.mode = Mode::Auto;
        p.controller.tier = 0;
        assert_eq!(p.quality(Quality::High), Quality::Low);
        assert_eq!(p.radius(12), 2);
        assert_eq!(p.render_scale(), 0.5);
        p.controller.tier = 4;
        assert_eq!(p.quality(Quality::Low), Quality::High);
        assert_eq!(p.radius(3), 9);
        assert_eq!(p.render_scale(), 1.0);
        p.preferences.mode = Mode::Balanced;
        assert_eq!(p.quality(Quality::Low), Quality::Low);
        assert_eq!(p.radius(3), 3);
        p.pressure = true;
        assert_eq!(p.radius(12), 2);
        assert_eq!(p.quality(Quality::High), Quality::Low);
        assert_eq!(p.render_scale(), 0.5);
    }
}
