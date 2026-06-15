# STEP-2A — Scene/Widget structs

> **File:** `crates/migration-runtime/src/protocol.rs`. These are `Serialize,
> Deserialize, Clone, Debug, PartialEq` (no `Eq/Hash` — they hold `f64`).

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WidgetLayout {
    pub x: f64,        // 0.0–100.0 (percent of output width)
    pub y: f64,        // 0.0–100.0 (percent of output height)
    pub width: f64,    // 1.0–100.0
    pub height: f64,   // 1.0–100.0
    #[serde(default)]
    pub rotation: f64, // 0.0–360.0 — STORED but not honored by `compositor` in v0.2.0
    #[serde(default = "default_opacity")]
    pub opacity: f64,  // 0.0–1.0
}

fn default_opacity() -> f64 {
    1.0
}

impl Default for WidgetLayout {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0, width: 100.0, height: 100.0, rotation: 0.0, opacity: 1.0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SceneWidgetPlacement {
    pub widget_id: String,          // FK → Widget.id
    pub layout: WidgetLayout,
    #[serde(default = "default_as_true")]
    pub enabled: bool,
    /// Stacking order on the compositor; higher renders on top. Camera = 0.
    #[serde(default)]
    pub zorder: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Scene {
    pub id: String,
    pub name: String,
    #[serde(default = "default_as_true")]
    pub enabled: bool,
    #[serde(default)]
    pub widgets: Vec<SceneWidgetPlacement>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quick_switch_group: Option<u8>, // 1–4
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Widget {
    pub id: String,
    pub name: String,
    #[serde(flatten)]
    pub widget_type: WidgetType,
    #[serde(default = "default_as_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum WidgetType {
    Text {
        format: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        font_size: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        color: Option<String>,          // hex RGBA, e.g. "#ffffffff"
    },
    Image {
        asset_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        scale_mode: Option<String>,     // "fit" | "fill" | "stretch"
    },
    Crop {
        top: f64,
        bottom: f64,
        left: f64,
        right: f64,
    },
    Clock {
        format: String,                 // strftime, e.g. "%H:%M:%S"
        #[serde(default, skip_serializing_if = "Option::is_none")]
        font_size: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        color: Option<String>,
    },
}
```

> `#[serde(tag = "type")]` keeps a `Widget` flat on the wire:
> `{"id":"w1","name":"Title","type":"text","format":"…","enabled":true}`.
> `default_as_true` already exists in `protocol.rs`.

→ Next: [STEP-2B-commands.md](STEP-2B-commands.md)
