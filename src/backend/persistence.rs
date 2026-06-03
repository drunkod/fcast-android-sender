use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::BackendKind;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CameraRtmpConfig {
    pub url: String,
    pub camera_idx: u32,
    pub resolution_idx: u32,
    pub framerate_idx: u32,
    pub mirror: bool,
    pub stabilization: bool,
    pub zoom: f32,
}

impl Default for CameraRtmpConfig {
    fn default() -> Self {
        Self {
            url: "rtmp://10.106.137.137/live/".to_owned(),
            camera_idx: 1,
            resolution_idx: 1,
            framerate_idx: 1,
            mirror: false,
            stabilization: true,
            zoom: 1.0,
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
