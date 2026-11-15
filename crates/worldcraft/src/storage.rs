use crate::session::*;
use bevy::{
    prelude::*,
    tasks::{block_on, futures_lite::future, IoTaskPool, Task},
};
use std::{path::PathBuf, time::Instant};
use worldcraft_core::{load_world, save_world, WorldFile};

enum Outcome {
    Loaded(WorldFile, PathBuf),
    Saved {
        path: PathBuf,
        epoch: u64,
        revision: u64,
        recovery: bool,
    },
    Cancelled,
}
#[derive(Resource)]
pub struct Disk {
    task: Option<Task<Result<Outcome, String>>>,
    last_autosave: Instant,
}
impl Default for Disk {
    fn default() -> Self {
        Self {
            task: None,
            last_autosave: Instant::now(),
        }
    }
}
impl Disk {
    pub fn busy(&self) -> bool {
        self.task.is_some()
    }
}
pub fn data_dir() -> PathBuf {
    if let Some(p) = std::env::var_os("LOCALAPPDATA") {
        PathBuf::from(p).join("Worldcraft")
    } else if let Some(p) = std::env::var_os("XDG_DATA_HOME") {
        PathBuf::from(p).join("worldcraft")
    } else if let Some(p) = std::env::var_os("HOME") {
        PathBuf::from(p).join(".local/share/worldcraft")
    } else {
        std::env::temp_dir().join("worldcraft")
    }
}
pub fn open(disk: &mut Disk, path: Option<PathBuf>) {
    if disk.busy() {
        return;
    }
    disk.task = Some(IoTaskPool::get().spawn(async move {
        let path = match path.or_else(|| {
            rfd::FileDialog::new()
                .set_title("Open a Worldcraft world")
                .add_filter("Worldcraft projects", &["json"])
                .pick_file()
        }) {
            Some(p) => p,
            None => return Ok(Outcome::Cancelled),
        };
        load_world(&path).map(|world| Outcome::Loaded(world, path))
    }));
}
pub fn save(disk: &mut Disk, session: &Session, player: &Player, recovery: bool) {
    if disk.busy() {
        return;
    }
    let Some(world) = session.snapshot(player) else {
        return;
    };
    let original = session.path.clone();
    let epoch = session.epoch;
    let revision = session.revision;
    disk.task = Some(IoTaskPool::get().spawn(async move {
        let path = if recovery {
            original
                .as_ref()
                .map(|p| p.with_extension("recovery.worldcraft.json"))
                .unwrap_or_else(|| {
                    data_dir().join(format!("recovery-{}.worldcraft.json", world.settings.seed))
                })
        } else if let Some(p) = original {
            p
        } else {
            match rfd::FileDialog::new()
                .set_title("Save your world")
                .set_file_name("My-world.worldcraft.json")
                .add_filter("Worldcraft projects", &["json"])
                .save_file()
            {
                Some(p) => p,
                None => return Ok(Outcome::Cancelled),
            }
        };
        save_world(&path, &world)?;
        Ok(Outcome::Saved {
            path,
            epoch,
            revision,
            recovery,
        })
    }));
}
pub fn poll(
    mut disk: ResMut<Disk>,
    mut session: ResMut<Session>,
    mut player: ResMut<Player>,
    mut ui: ResMut<UiState>,
    controls: Res<Controls>,
    launch: Res<Launch>,
) {
    let result = disk
        .task
        .as_mut()
        .and_then(|t| block_on(future::poll_once(t)));
    if let Some(result) = result {
        disk.task = None;
        match result {
            Ok(Outcome::Loaded(world, path)) => {
                session.open(world, Some(path.clone()), &mut player);
                ui.home = false;
                ui.create = false;
                ui.message = "World opened".into();
                remember(&mut ui, path);
            }
            Ok(Outcome::Saved {
                path,
                epoch,
                revision,
                recovery,
            }) => {
                if session.epoch == epoch {
                    if !recovery {
                        session.path = Some(path.clone());
                        if session.revision == revision {
                            session.dirty = false;
                        }
                        remember(&mut ui, path);
                        ui.message = "World saved. Previous save kept as a backup.".into();
                    } else {
                        ui.message = "Recovery snapshot saved".into();
                    }
                }
            }
            Ok(Outcome::Cancelled) => {}
            Err(e) => {
                error!("{e}");
                ui.message = e;
            }
        }
    }
    if !launch.smoke && disk.last_autosave.elapsed().as_secs() >= 60 {
        disk.last_autosave = Instant::now();
        if session.dirty && controls.live.is_empty() && !disk.busy() {
            save(&mut disk, &session, &player, true);
        }
    }
}
fn remember(ui: &mut UiState, path: PathBuf) {
    ui.recents.retain(|p| p != &path);
    ui.recents.insert(0, path);
    ui.recents.truncate(8);
    let paths = ui.recents.clone();
    IoTaskPool::get()
        .spawn(async move {
            if std::fs::create_dir_all(data_dir()).is_ok() {
                if let Ok(bytes) = serde_json::to_vec(&paths) {
                    let _ = std::fs::write(data_dir().join("recent.json"), bytes);
                }
            }
        })
        .detach();
}
pub fn recent() -> Vec<PathBuf> {
    std::fs::read(data_dir().join("recent.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<Vec<PathBuf>>(&b).ok())
        .unwrap_or_default()
        .into_iter()
        .take(8)
        .collect()
}
