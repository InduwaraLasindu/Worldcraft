use crate::{
    config::{GENERATOR_VERSION, MAX_WORLD_COORD},
    GeneratorSettings, Stroke,
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::Path,
};
pub const FILE_VERSION: u32 = 2;
const MAX_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WorldFile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub learned: Option<crate::regional::LearnedRegion>,
    pub format_version: u32,
    pub generator_version: u32,
    pub name: String,
    pub settings: GeneratorSettings,
    pub settings_hash: u64,
    pub edits: Vec<Vec<Stroke>>,
    pub camera: [f64; 3],
    pub yaw: f32,
    pub pitch: f32,
}
impl WorldFile {
    pub fn new(name: String, settings: GeneratorSettings) -> Self {
        Self {
            format_version: FILE_VERSION,
            learned: None,
            generator_version: GENERATOR_VERSION,
            settings_hash: settings.fingerprint(),
            settings,
            name,
            edits: Vec::new(),
            camera: [0.0, 100.0, 0.0],
            yaw: 0.0,
            pitch: -0.15,
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if !(1..=FILE_VERSION).contains(&self.format_version) {
            return Err(format!(
                "Unsupported project format {}; this build reads version {FILE_VERSION}",
                self.format_version
            ));
        }
        if !(1..=GENERATOR_VERSION).contains(&self.generator_version) {
            return Err("This world uses a different terrain generator version".into());
        }
        self.settings.validate()?;
        if let Some(region) = &self.learned {
            if self.format_version < 2 || self.generator_version != 2 {
                return Err("Learned regions require format 2 and generator 2".into());
            }
            region.validate()?;
        }
        if self.name.trim().is_empty() || self.name.len() > 256 {
            return Err("World name must contain 1–256 bytes".into());
        }
        if self.settings_hash != self.settings.fingerprint() {
            return Err("Generation settings checksum does not match".into());
        }
        if self.edits.len() > 25_000 || self.edits.iter().map(Vec::len).sum::<usize>() > 100_000 {
            return Err("Project exceeds the supported edit-history limit".into());
        }
        for s in self.edits.iter().flatten() {
            s.validate()?;
        }
        if !self
            .camera
            .iter()
            .all(|c| c.is_finite() && c.abs() <= MAX_WORLD_COORD)
            || !self.yaw.is_finite()
            || !self.pitch.is_finite()
            || self.pitch.abs() > 1.56
        {
            return Err("Invalid saved camera coordinates".into());
        }
        Ok(())
    }
}
pub fn load_world(path: &Path) -> Result<WorldFile, String> {
    let file = fs::File::open(path).map_err(|e| format!("Cannot open world: {e}"))?;
    if file.metadata().map_err(|e| e.to_string())?.len() > MAX_BYTES {
        return Err("World file exceeds the 32 MiB safety limit".into());
    }
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("World file is too large".into());
    }
    let world: WorldFile =
        serde_json::from_slice(&bytes).map_err(|e| format!("Invalid world file: {e}"))?;
    world.validate()?;
    Ok(world)
}
pub fn save_world(path: &Path, world: &WorldFile) -> Result<(), String> {
    world.validate()?;
    let bytes = serde_json::to_vec_pretty(world).map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("World file is too large".into());
    }
    if let Some(p) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(p).map_err(|e| format!("Cannot create world folder: {e}"))?;
    }
    let tmp = path.with_extension(format!("json.{}.tmp", std::process::id()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
            .map_err(|e| format!("Cannot create temporary world file: {e}"))?;
        file.write_all(&bytes).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        if path.exists() {
            fs::copy(path, path.with_extension("json.bak"))
                .map_err(|e| format!("Cannot back up world: {e}"))?;
        }
        fs::rename(&tmp, path).map_err(|e| format!("Cannot replace world file: {e}"))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}
