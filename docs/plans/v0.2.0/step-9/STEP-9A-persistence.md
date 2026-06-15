# STEP-9A — Persistence

> Scenes/widgets are app config (mapping doc §2). Reuse the v0.1.0
> `StoredBackendConfig` + `crate::config::update`/`load` machinery.

**File:** `src/backend/persistence.rs`

```rust
use migration_runtime::protocol::{Scene, Widget};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StoredBackendConfig {
    // ... existing fields (kind, gstpop_*, camera_rtmp, global_camera, srt_destination) ...
    #[serde(default)]
    pub scenes: Vec<Scene>,
    #[serde(default)]
    pub widgets: Vec<Widget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_scene_id: Option<String>,
}
```

Also:
- Add `scenes`/`widgets`/`current_scene_id` to `defaults()` (empty/None).
- **Preserve them** in `lifecycle.rs::read_config_from_bridge`'s spread (like
  v0.1.0 did for `srt_destination`) so media-backend saves don't wipe them.
- Default seed: if `scenes` is empty on first load, create one
  `Scene { id, name: "Main", enabled: true, widgets: [], quick_switch_group: None }`
  (PHASE-40 §40-G).

`Scene`/`Widget` already derive `Serialize, Deserialize` (STEP-2);
`#[serde(default)]` keeps existing `backend.json` files loading cleanly.

→ Next: [STEP-9B-model-conversion.md](STEP-9B-model-conversion.md)
