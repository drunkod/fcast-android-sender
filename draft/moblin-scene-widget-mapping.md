# Moblin Scene & Widget System → FCast Rust/Slint Mapping

> **Reference:** [deepwiki.com/eerimoq/moblin/4-scene-and-widget-system](https://deepwiki.com/eerimoq/moblin/4-scene-and-widget-system)
>
> **Source:** `draft/moblin-ui/Moblin/View/Settings/Scenes/` (48 Swift files)

---

## Section 1 — Moblin Scene/Widget Architecture Summary

### Core Data Model

Moblin uses a **global widget pool** + **scene references** pattern:
widgets are defined once in `database.widgets` and referenced by
scenes via `SettingsSceneWidget` bridge objects that add per-scene
positioning.

```swift
// SettingsScene (ObservableObject / class)
class SettingsScene: ObservableObject, Identifiable {
    let id: UUID
    var name: String
    var enabled: Bool
    var widgets: [SettingsSceneWidget]       // ordered placements
    var videoSource: SettingsSceneVideoSource // camera/ingest/media-player/screen
    var micId: String                        // optional mic override
    var quickSwitchGroup: Int?               // 1–4 for instant switching
    var videoStabilizationMode: ...
    var fillFrame: Bool
}

// SettingsSceneWidget (ObservableObject / class)
class SettingsSceneWidget: ObservableObject, Identifiable {
    let id: UUID
    var widgetId: UUID                       // FK → SettingsWidget.id
    var layout: SettingsWidgetLayout         // x, y, width, height, rotation, opacity
    var enabled: Bool
}

// SettingsWidgetLayout
struct SettingsWidgetLayout {
    var x: Double       // 0–100 (percentage)
    var y: Double       // 0–100 (percentage)
    var width: Double   // 1–100
    var height: Double  // 1–100
    var rotation: Double // 0–360 degrees
    var opacity: Double  // 0.0–1.0
    var alignment: SettingsAlignment  // topLeft, center, bottomRight, etc.
}

// SettingsWidget (ObservableObject / class)
class SettingsWidget: ObservableObject, Identifiable {
    let id: UUID
    var name: String
    var type: SettingsWidgetType
    var enabled: Bool
    // type-specific sub-structs:
    var text: SettingsWidgetText
    var browser: SettingsWidgetBrowser
    var image: SettingsWidgetImage
    var crop: SettingsWidgetCrop
    var videoSource: SettingsWidgetVideoSource
    // ... etc.
}

// SettingsWidgetType (enum, CaseIterable)
enum SettingsWidgetType: CaseIterable {
    case text
    case image
    case browser
    case videoSource
    case crop
    case chat
    case alerts
    case map
    case qrCode
    case snapshot
    case scoreboard
    case vTuber
    case pngTuber
    case slideshow
    case wheelOfLuck
    case bingoCard
}
```

### Scene Lifecycle

1. **Scene selection** — `SceneSelector` tracks active scene. Switching
   triggers full pipeline reconfiguration via `model.sceneUpdated()`.
2. **Widget effect instantiation** — On scene update, each widget's
   `Effect` class is created/updated: `TextEffect`, `ImageEffect`,
   `BrowserEffect`, etc.
3. **Quick switch groups** — Scenes in the same group (1–4) can switch
   instantly without camera reattachment (same video source type).
4. **Remote scene** — A designated scene's widgets can be displayed on
   a connected remote-control assistant device.

### Key Moblin Swift Source Files

| File | Role |
|------|------|
| `View/Settings/Scenes/ScenesSettingsView.swift` | Scene list + create + reorder |
| `View/Settings/Scenes/Scene/SceneSettingsView.swift` | Per-scene: video source, mic, widgets, quick-switch |
| `View/Settings/Scenes/Scene/SceneWidgetSettingsView.swift` | Widget layout placement within a scene |
| `View/Settings/Scenes/Widgets/Widget/WidgetSettingsView.swift` | Widget layout editor (x/y/size/rotation/alignment) |
| `View/Settings/Scenes/Widgets/Widget/WidgetWizardSettingsView.swift` | Widget creation wizard (type picker → config) |
| `View/Settings/Scenes/Widgets/Widget/Text/WidgetTextSettingsView.swift` | Text widget config |
| `View/Settings/Scenes/Widgets/Widget/Image/WidgetImageSettingsView.swift` | Image widget config |
| `View/Settings/Scenes/Widgets/Widget/Crop/WidgetCropSettingsView.swift` | Crop widget config |
| `View/Settings/Scenes/Widgets/Widget/Browser/WidgetBrowserSettingsView.swift` | Browser source config |
| `View/ControlBar/ControlBarPortraitView.swift` | Scene quick-switch buttons (bottom bar) |
| `View/ControlBar/QuickButtonsView.swift` | Quick button actions incl. scene widgets panel |

---

## Section 2 — FCast Rust Equivalents

| Moblin concept | FCast Rust equivalent | Notes |
|---|---|---|
| `SettingsScene` | New `Scene` struct in `crates/migration-runtime/src/protocol.rs` | Scenes are a composition layer, not a pipeline concept; stored in app config |
| `SettingsSceneWidget` | New `SceneWidgetPlacement` struct in `protocol.rs` | Position/size of a widget within a scene (references `Widget.id`) |
| `SettingsWidget` | New `Widget` struct + `WidgetType` enum in `protocol.rs` | Only MVP-relevant types initially |
| `SettingsWidgetLayout` | New `WidgetLayout` struct | `x`, `y`, `width`, `height` as `f64` (0.0–100.0); `rotation` and `opacity` |
| `SettingsWidgetType` enum | New `WidgetType` enum | Start with `Text`, `Image`, `Crop`, `Clock`; expand post-MVP |
| `model.setCurrentScene()` / `selectScene(id:)` | New `Command::SetScene { scene_id: String }` | Switches compositor overlay set |
| `model.sceneUpdated()` | Internal `NodeManager` handler | Rebuilds GStreamer `compositor` pad config |
| `SceneSelector` | App-level state in Rust (`current_scene_id: Option<String>`) | Bridge property → Slint |
| Scene buttons (ControlBar) | Slint `HorizontalLayout` with `SceneButton` components in `ui/pages/stream_page.slint` | Bottom bar during live stream |
| Widget overlay rendering | GStreamer `compositor` or `glvideomixer` element | Composites widget textures onto the video pipeline |
| Quick-switch groups | `Scene.quick_switch_group: Option<u8>` | Enables instant scene switch without re-attaching camera |
| Widget creation wizard | Slint multi-step panel (`wizard_widget_create_page.slint`) | Type picker → config → add to scene |

### Proposed Rust Structs (v0.2.0 target)

```rust
// crates/migration-runtime/src/protocol.rs additions

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub widgets: Vec<SceneWidgetPlacement>,
    pub quick_switch_group: Option<u8>,  // 1–4
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneWidgetPlacement {
    pub widget_id: String,
    pub layout: WidgetLayout,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetLayout {
    pub x: f64,       // 0.0–100.0 (percentage)
    pub y: f64,       // 0.0–100.0
    pub width: f64,   // 1.0–100.0
    pub height: f64,  // 1.0–100.0
    pub rotation: f64, // 0.0–360.0 degrees
    pub opacity: f64,  // 0.0–1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Widget {
    pub id: String,
    pub name: String,
    pub widget_type: WidgetType,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WidgetType {
    Text { format: String },
    Image { asset_id: String },
    Crop { top: f64, bottom: f64, left: f64, right: f64 },
    Clock { format: String },
    // Post-MVP:
    // Browser { url: String, width: u32, height: u32 },
    // VideoEffect { effect_name: String },
    // Map { style: String, zoom: f64 },
}

// New command variant:
// Command::SetScene { scene_id: String }
// Command::UpdateWidgetLayout { scene_id: String, widget_id: String, layout: WidgetLayout }
```

### GStreamer Pipeline Integration

Scenes affect the **compositor** stage of the pipeline, not the
source or destination stages:

```
camera-appsrc → videoconvert ─┐
                               ├─→ compositor → encoder → mux → sink
widget-appsrc(s) ─────────────┘
```

Each active widget becomes an `appsrc` feeding a compositor sink pad
with position/size properties set from `WidgetLayout`:

| Widget type | GStreamer element(s) | Feed mechanism |
|---|---|---|
| Text / Clock | `textoverlay` or `pango` → `appsrc` | Render text to RGBA buffer, push to compositor |
| Image | `gdkpixbufoverlay` or `imagefreeze` → `appsrc` | Decode image once, freeze as static frame |
| Crop | `videocrop` on main video | Applied before compositor (modifies source geometry) |
| Browser (post-MVP) | Android WebView → texture → `appsrc` | Frame-by-frame bridge from WebView render surface |

When `Command::SetScene` is received, the `NodeManager`:
1. Removes compositor sink pads for widgets not in the new scene
2. Adds compositor sink pads for new widgets
3. Updates `xpos`/`ypos`/`width`/`height`/`alpha` properties on existing pads

---

## Section 3 — Widget Types: What to Include vs Defer

| Moblin Widget Type | MVP? | FCast implementation | GStreamer element |
|---|---|---|---|
| `.text` (label overlay) | ✅ MVP (v0.2.0) | `WidgetType::Text` — format string with variables | `textoverlay` or `pango` render → `appsrc` |
| `.image` (static image overlay) | ✅ MVP (v0.2.0) | `WidgetType::Image` — asset reference | `gdkpixbufoverlay` or `imagefreeze` → `appsrc` |
| `.crop` (video crop) | ✅ MVP (v0.2.0) | `WidgetType::Crop` — top/bottom/left/right margins | `videocrop` element on main pipeline |
| `.videoSource` (nested scene) | ⚠️ v0.2.0 stretch | Embed another scene as a widget | Recursive compositor (complex) |
| `.qrCode` (QR overlay) | ⚠️ v0.2.0 stretch | Generate QR in Rust, push as image | `qrencode` crate → RGBA → `appsrc` |
| `.browser` (web source) | ❌ Post-MVP (v0.3.0) | Requires Android WebView → texture bridge | WebView surface → `appsrc` |
| `.chat` (Twitch/YouTube chat) | ❌ Not applicable | FCast has no chat integration | — |
| `.alerts` (stream alerts) | ❌ Not applicable | FCast has no alert service | — |
| `.map` (GPS overlay) | ❌ Post-MVP (v0.3.0) | Requires location service + map renderer | Map tile render → `appsrc` |
| `.scoreboard` | ❌ Not applicable | FCast is not sports-focused | — |
| `.vTuber` / `.pngTuber` | ❌ Not applicable | Requires face-tracking hardware | — |
| `.slideshow` | ❌ Post-MVP | Rotating widget sequence | Timer-driven widget swap |
| `.wheelOfLuck` / `.bingoCard` | ❌ Not applicable | Entertainment/gaming widgets | — |
| `.snapshot` (freeze frame) | ⚠️ v0.2.0 stretch | Capture current frame as overlay | `appsrc` with last-buffer |
| time/clock overlay | ✅ MVP (v0.2.0) | `WidgetType::Clock` — clock format | `clockoverlay` or `timeoverlay` element |

### MVP Widget Subset (v0.2.0)

| Widget | Justification |
|---|---|
| **Text** | Essential for stream branding (name, title, URL) |
| **Image** | Logo overlay, watermark — most requested feature |
| **Crop** | Reframe camera output without source change |
| **Clock** | Timestamp overlay — trivial with GStreamer builtins |

### Deferred Rationale

| Category | Reason |
|---|---|
| Browser source | Android WebView → GStreamer texture bridge is non-trivial; requires SurfaceTexture interop |
| Chat/Alerts/Scoreboard | FCast is a casting tool, not a streaming platform — no chat/alert backend |
| VTuber/PngTuber | Requires ARCore face mesh or custom ML model — out of scope |
| Map/GPS | Requires `FusedLocationProvider` + map tile renderer — complex Android service |

---

## Section 4 — Scene Switching Pipeline Impact

### Does scene switching rebuild the pipeline?

In Moblin: **No full rebuild**. Scene switching only reconfigures the
compositor overlay layer. The camera source and encoder continue running.
Quick-switch groups enable truly instant switches (same camera → just
swap overlays). Cross-group switches may need camera reattachment.

### FCast equivalent behavior:

```
SetScene(scene_id) →
  1. Resolve new scene's widget list
  2. Diff against current compositor pads
  3. Remove pads for deactivated widgets
  4. Add pads for newly activated widgets
  5. Update xpos/ypos/width/height/alpha on retained pads
  6. If quick_switch_group differs: reattach camera source (heavier)
  7. Emit state change to Bridge for UI update
```

This is handled entirely within the `NodeManager` / mixer node —
no destination or source rebuild needed.
