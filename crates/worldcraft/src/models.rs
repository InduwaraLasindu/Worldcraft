//! Optional, verified model sets. Downloads and checks never block the UI thread.
use bevy::{
    prelude::*,
    tasks::{block_on, futures_lite::future, IoTaskPool, Task},
};
use std::{
    path::PathBuf,
    sync::{atomic::Ordering, Arc},
};
use worldcraft_ml::{assets::ModelPaths, Progress};
struct ModelCheck {
    model: Option<PathBuf>,
    detail: Option<ModelPaths>,
    error: String,
}
#[derive(Resource)]
pub struct Models {
    pub model: Option<PathBuf>,
    pub detail: Option<ModelPaths>,
    pub runtime: Option<PathBuf>,
    pub progress: Arc<Progress>,
    pub error: String,
    task: Option<Task<ModelCheck>>,
    checked: bool,
}
pub fn runtime_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("WORLDCRAFT_ORT_DYLIB")
        .map(PathBuf::from)
        .filter(|p| p.is_file())
    {
        return Some(path);
    }
    let root = std::env::current_exe().ok()?.parent()?.to_owned();
    let name = if cfg!(windows) {
        "onnxruntime.dll"
    } else {
        "libonnxruntime.so"
    };
    [root.join(name), root.join("lib").join(name)]
        .into_iter()
        .find(|p| p.is_file())
}
impl Default for Models {
    fn default() -> Self {
        let file = worldcraft_ml::model_path(&crate::storage::data_dir().join("models"));
        let model = file
            .metadata()
            .ok()
            .filter(|m| m.len() == worldcraft_ml::MODEL_BYTES)
            .map(|_| file);
        Self {
            model,
            detail: None,
            runtime: runtime_path(),
            progress: Arc::new(Progress::default()),
            error: String::new(),
            task: None,
            checked: false,
        }
    }
}
fn inspect(paths: ModelPaths, progress: &Progress, include_detail: bool) -> ModelCheck {
    let model = match worldcraft_ml::assets::verify_asset(
        &paths.coarse,
        worldcraft_ml::assets::COARSE,
        progress,
    ) {
        Ok(()) => Some(paths.coarse.clone()),
        Err(error) => {
            return ModelCheck {
                model: None,
                detail: None,
                error,
            }
        }
    };
    if include_detail && paths.present() {
        match paths.verify(progress) {
            Ok(()) => ModelCheck {
                model,
                detail: Some(paths),
                error: String::new(),
            },
            Err(error) => ModelCheck {
                model,
                detail: None,
                error,
            },
        }
    } else {
        ModelCheck {
            model,
            detail: None,
            error: String::new(),
        }
    }
}
impl Models {
    /// Includes startup checksum verification, so incomplete files are never offered as ready.
    pub fn downloading(&self) -> bool {
        self.task.is_some()
    }
    pub fn ready(&self) -> bool {
        self.checked && self.model.is_some() && self.runtime.is_some() && !self.downloading()
    }
    pub fn detailed_ready(&self) -> bool {
        self.ready() && self.detail.is_some()
    }
    pub fn download(&mut self, detailed: bool) {
        if self.downloading() {
            return;
        }
        self.checked = true;
        self.error.clear();
        self.progress = Arc::new(Progress::default());
        let progress = Arc::clone(&self.progress);
        self.task = Some(IoTaskPool::get().spawn(async move {
            let folder = crate::storage::data_dir().join("models");
            match worldcraft_ml::assets::download_models(&folder, &progress, detailed) {
                Ok(paths) => ModelCheck {
                    model: Some(paths.coarse.clone()),
                    detail: if detailed { Some(paths) } else { None },
                    error: String::new(),
                },
                Err(error) => {
                    // A cancelled large download must keep an already verified fast model usable.
                    let mut state =
                        inspect(ModelPaths::in_folder(&folder), &Progress::default(), false);
                    state.error = error;
                    state
                }
            }
        }));
    }
    pub fn cancel(&self) {
        self.progress.cancel.store(true, Ordering::Relaxed);
    }
}
pub fn poll(mut models: ResMut<Models>) {
    if !models.checked && models.task.is_none() {
        models.checked = true;
        if let Some(path) = models.model.clone() {
            let progress = Arc::clone(&models.progress);
            progress.stage.store(1, Ordering::Relaxed);
            models.task =
                Some(IoTaskPool::get().spawn(async move {
                    inspect(ModelPaths::from_coarse(&path), &progress, true)
                }));
        }
    }
    if let Some(result) = models
        .task
        .as_mut()
        .and_then(|t| block_on(future::poll_once(t)))
    {
        models.task = None;
        models.model = result.model;
        models.detail = result.detail;
        models.error = result.error;
        models.progress.stage.store(10, Ordering::Relaxed);
    }
}
