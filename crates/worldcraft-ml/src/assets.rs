//! Immutable, streamed model assets. Large weights are always optional.
use crate::{Progress, MODEL_BYTES, MODEL_REVISION, MODEL_SHA256};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::Ordering,
};
#[derive(Clone, Copy, Debug)]
pub struct ModelSpec {
    pub name: &'static str,
    pub bytes: u64,
    pub sha256: &'static str,
}
pub const COARSE: ModelSpec = ModelSpec {
    name: "coarse_model.onnx",
    bytes: MODEL_BYTES,
    sha256: MODEL_SHA256,
};
pub const BASE: ModelSpec = ModelSpec {
    name: "base_model.onnx",
    bytes: 2029994361,
    sha256: "543de788f73d0a4012685c908259f615601102aace4751aeccec64154ba145c0",
};
pub const DECODER: ModelSpec = ModelSpec {
    name: "decoder_model.onnx",
    bytes: 223854143,
    sha256: "6473ae47ca6ec4d743d30fe4f5d381fe4158899714eff09b762005bdbdef68c1",
};
pub const DETAILED_BYTES: u64 = COARSE.bytes + BASE.bytes + DECODER.bytes;
#[derive(Clone, Debug)]
pub struct ModelPaths {
    pub coarse: PathBuf,
    pub base: PathBuf,
    pub decoder: PathBuf,
}
impl ModelPaths {
    pub fn in_folder(root: &Path) -> Self {
        let root = root.join(MODEL_REVISION);
        Self {
            coarse: root.join(COARSE.name),
            base: root.join(BASE.name),
            decoder: root.join(DECODER.name),
        }
    }
    pub fn from_coarse(path: &Path) -> Self {
        let root = path.parent().unwrap_or_else(|| Path::new("."));
        Self {
            coarse: path.to_owned(),
            base: root.join(BASE.name),
            decoder: root.join(DECODER.name),
        }
    }
    pub fn present(&self) -> bool {
        [
            (&self.coarse, COARSE),
            (&self.base, BASE),
            (&self.decoder, DECODER),
        ]
        .iter()
        .all(|(p, s)| p.metadata().is_ok_and(|m| m.len() == s.bytes))
    }
    pub fn verify(&self, progress: &Progress) -> Result<(), String> {
        if !self.present() {
            return Err("The complete optional detailed model set is not downloaded".into());
        }
        progress.stage.store(1, Ordering::Relaxed);
        for (path, spec) in [
            (&self.coarse, COARSE),
            (&self.base, BASE),
            (&self.decoder, DECODER),
        ] {
            verify_asset(path, spec, progress)?;
        }
        Ok(())
    }
}
pub fn verify_asset(path: &Path, spec: ModelSpec, progress: &Progress) -> Result<(), String> {
    progress.check()?;
    if path.metadata().map_err(|e| e.to_string())?.len() != spec.bytes {
        return Err(format!("{} size mismatch", spec.name));
    }
    let mut hash = Sha256::new();
    let mut source = File::open(path).map_err(|e| e.to_string())?;
    let mut buffer = [0u8; 65536];
    loop {
        progress.check()?;
        let n = source.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    let actual: String = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
    if actual != spec.sha256 {
        return Err(format!(
            "{} checksum mismatch; download a clean copy",
            spec.name
        ));
    }
    Ok(())
}
fn acquire(path: &Path, spec: ModelSpec, progress: &Progress, before: u64) -> Result<(), String> {
    progress.stage.store(1, Ordering::Relaxed);
    if path.is_file() && verify_asset(path, spec, progress).is_ok() {
        progress.bytes.store(before + spec.bytes, Ordering::Relaxed);
        return Ok(());
    }
    progress.check()?;
    fs::create_dir_all(path.parent().ok_or("Invalid model destination")?)
        .map_err(|e| e.to_string())?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temporary = path.with_extension(format!("{}.{}.part", std::process::id(), stamp));
    let result = (|| {
        progress.stage.store(2, Ordering::Relaxed);
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(std::time::Duration::from_secs(1800)))
            .timeout_recv_body(Some(std::time::Duration::from_secs(30)))
            .build();
        let agent: ureq::Agent = config.into();
        let url=format!("https://huggingface.co/xandergos/terrain-diffusion-30m-onnx/resolve/{MODEL_REVISION}/{}",spec.name);
        let mut response = agent
            .get(&url)
            .call()
            .map_err(|e| format!("Model download failed: {e}"))?;
        let mut reader = response.body_mut().as_reader();
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|e| e.to_string())?;
        let mut buffer = [0u8; 65536];
        let mut count = 0_u64;
        let mut hash = Sha256::new();
        loop {
            progress.check()?;
            let n = reader.read(&mut buffer).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            count += n as u64;
            if count > spec.bytes {
                return Err("Model download exceeded the pinned size".into());
            }
            output.write_all(&buffer[..n]).map_err(|e| e.to_string())?;
            hash.update(&buffer[..n]);
            progress.bytes.store(before + count, Ordering::Relaxed);
        }
        progress.check()?;
        let actual: String = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
        if count != spec.bytes || actual != spec.sha256 {
            return Err(format!(
                "{} download did not match the immutable model",
                spec.name
            ));
        }
        output.sync_all().map_err(|e| e.to_string())?;
        drop(output);
        fs::rename(&temporary, path).map_err(|e| format!("Cannot finalize {}: {e}", spec.name))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}
pub fn download_models(
    root: &Path,
    progress: &Progress,
    detailed: bool,
) -> Result<ModelPaths, String> {
    progress.total_bytes.store(
        if detailed {
            DETAILED_BYTES
        } else {
            MODEL_BYTES
        },
        Ordering::Relaxed,
    );
    progress.bytes.store(0, Ordering::Relaxed);
    let paths = ModelPaths::in_folder(root);
    let mut before = 0;
    for (path, spec) in [
        (&paths.coarse, COARSE),
        (&paths.base, BASE),
        (&paths.decoder, DECODER),
    ] {
        if !detailed && spec.name != COARSE.name {
            break;
        }
        acquire(path, spec, progress, before)?;
        before += spec.bytes;
    }
    progress.stage.store(10, Ordering::Relaxed);
    Ok(paths)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_cancelled_asset_operation_never_touches_existing_data() {
        let p = Progress::default();
        p.cancel.store(true, Ordering::Relaxed);
        let folder = std::env::temp_dir().join(format!("worldcraft-cancel-{}", std::process::id()));
        assert!(download_models(&folder, &p, true).is_err());
        assert!(!folder.exists());
    }
}
