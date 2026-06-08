use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::BackendKind;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CameraRtmpConfig {
    pub url: String,
}

impl Default for CameraRtmpConfig {
    fn default() -> Self {
        Self {
            url: "rtmp://10.106.137.137/live/".to_owned(),
        }
    }
}

fn default_srt_latency_ms() -> i32 {
    200
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SrtDestinationConfig {
    pub url: String,
    #[serde(default = "default_srt_latency_ms")]
    pub latency_ms: i32,
    /// 0 = None, 1 = AES-128, 2 = AES-192, 3 = AES-256. The passphrase itself
    /// is stored separately in the secret store, never in this file.
    #[serde(default)]
    pub pbkeylen_idx: i32,
}

impl Default for SrtDestinationConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            latency_ms: 200,
            pbkeylen_idx: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GlobalCameraConfig {
    pub camera_idx: i32,
    pub resolution_idx: i32,
    pub framerate_idx: i32,
    pub mirror_front: bool,
    pub stabilization: bool,
    pub zoom_level: f32,
}

impl Default for GlobalCameraConfig {
    fn default() -> Self {
        Self {
            camera_idx: 1,
            resolution_idx: 2,
            framerate_idx: 1,
            mirror_front: false,
            stabilization: true,
            zoom_level: 1.0,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StoredBackendConfig {
    pub kind: BackendKind,
    pub gstpop_url: String,
    #[serde(default)]
    pub gstpop_api_key_alias: Option<String>,
    pub gstpop_pipeline_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gstpop_api_key: Option<String>,
    #[serde(default)]
    pub camera_rtmp: Option<CameraRtmpConfig>,
    #[serde(default)]
    pub global_camera: Option<GlobalCameraConfig>,
    #[serde(default)]
    pub srt_destination: Option<SrtDestinationConfig>,
}

impl StoredBackendConfig {
    pub fn defaults() -> Self {
        Self {
            kind: BackendKind::Migration,
            gstpop_url: "ws://127.0.0.1:9000".into(),
            gstpop_api_key_alias: None,
            gstpop_pipeline_id: "0".into(),
            gstpop_api_key: None,
            camera_rtmp: None,
            global_camera: None,
            srt_destination: None,
        }
    }

    pub fn load(files_dir: &Path) -> Result<Self> {
        let path = files_dir.join("backend.json");
        if !path.exists() {
            return Ok(Self::defaults());
        }
        let bytes = fs::read(&path).with_context(|| format!("read {}", path.display()))?;
        serde_json::from_slice(&bytes).with_context(|| format!("parse {}", path.display()))
    }

    pub fn save(&self, files_dir: &Path) -> Result<()> {
        let path = files_dir.join("backend.json");
        let json = serde_json::to_string_pretty(self).context("serialize backend.json")?;
        fs::write(&path, json).with_context(|| format!("write {}", path.display()))?;
        Ok(())
    }
}
