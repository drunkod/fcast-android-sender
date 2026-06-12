pub mod migration;

pub use crate::backend::persistence::{
    CameraRtmpConfig, SrtDestinationConfig, StoredBackendConfig,
};
use once_cell::sync::OnceCell;
use std::path::PathBuf;

static FILES_DIR: OnceCell<PathBuf> = OnceCell::new();

pub fn init(files_dir: PathBuf) {
    if FILES_DIR.set(files_dir).is_err() {
        tracing::warn!("config::init called twice; ignoring");
    }
}

pub fn get_files_dir() -> Option<&'static PathBuf> {
    FILES_DIR.get()
}

pub fn load() -> StoredBackendConfig {
    if let Some(dir) = get_files_dir() {
        StoredBackendConfig::load(dir).unwrap_or_else(|_| StoredBackendConfig::defaults())
    } else {
        StoredBackendConfig::defaults()
    }
}

pub fn update<F>(f: F) -> Result<(), String>
where
    F: FnOnce(&mut StoredBackendConfig),
{
    if let Some(dir) = get_files_dir() {
        let mut cfg =
            StoredBackendConfig::load(dir).unwrap_or_else(|_| StoredBackendConfig::defaults());
        f(&mut cfg);
        cfg.save(dir).map_err(|e| e.to_string())
    } else {
        Err("config not initialized with FILES_DIR".to_owned())
    }
}
