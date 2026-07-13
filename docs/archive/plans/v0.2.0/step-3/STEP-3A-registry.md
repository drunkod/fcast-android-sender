# STEP-3A — `SceneRegistry`

> Scenes/widgets are app state (config), not pipeline nodes.

**File:** `crates/migration-runtime/src/node_manager.rs`

```rust
pub struct SceneRegistry {
    pub scenes: HashMap<String, Scene>,
    pub widgets: HashMap<String, Widget>,
    pub current_scene_id: Option<String>,
    /// link_id → widget_id currently wired into the live mixer.
    pub active_widget_links: HashMap<String, String>,
}
```

Add a `SceneRegistry` field (default empty) to the manager. It is serialized by
STEP-9 and queried for the UI models (STEP-9 §9b) — it never becomes a GStreamer
node itself.

→ Next: [STEP-3B-setscene-expansion.md](STEP-3B-setscene-expansion.md)
