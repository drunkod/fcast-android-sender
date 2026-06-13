pub mod migration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum AndroidCameraPipeline {
    #[default]
    LegacyRawI420Gstreamer,
    StreamPackDirectSrt,
    StreamPackEncodedToGstreamer,
}

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

static SESSION_PIPELINE: OnceCell<AndroidCameraPipeline> = OnceCell::new();

/// The camera pipeline mode for *this* process session: read from the persisted
/// config once and cached. Mode switching is restart-required (Phase 1 / Option A),
/// so this is the single source of truth the Kotlin coordinator
/// (`nativeUseStreamPackCameraPath`) and the Rust start/stop paths must agree on.
/// Reading the live Slint selector instead lets the two diverge mid-session (the
/// Kotlin coordinator is launch-fixed), which mismatches the JNI upcall and can leak
/// SRT graph nodes on stop.
pub fn session_android_camera_pipeline() -> AndroidCameraPipeline {
    *SESSION_PIPELINE.get_or_init(|| load().android_camera_pipeline)
}

/// Lock in the session pipeline mode at startup from the already-loaded config,
/// before the live selector can rewrite the persisted value. Must be called once
/// early in `android_main`; later calls are ignored. Without this, the first
/// `session_android_camera_pipeline()` call could read a selector-changed value and
/// diverge from the launch-fixed Kotlin coordinator.
pub fn prime_session_android_camera_pipeline(mode: AndroidCameraPipeline) {
    let _ = SESSION_PIPELINE.set(mode);
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
