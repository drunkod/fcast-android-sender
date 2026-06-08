# Moblin Swift → FCast Rust Conversion Plan

> **Scope:** Map key Moblin Swift model types and view patterns to their
> FCast Rust + Slint equivalents. This document complements the general
> `draft/slint-ui/docs/swiftui-to-slint-guide.md` with **concrete struct
> and enum mappings** for the scene/widget/streaming subsystems.

---

## 1. Model Layer — Swift Classes → Rust Structs

### 1.1 Core Settings Objects

| Moblin Swift (class/struct) | FCast Rust struct | Location | Notes |
|---|---|---|---|
| `SettingsScene` | `Scene` | `protocol.rs` | `ObservableObject` → plain `#[derive(Serialize, Deserialize)]` |
| `SettingsSceneWidget` | `SceneWidgetPlacement` | `protocol.rs` | Bridge between scene and widget pool |
| `SettingsWidget` | `Widget` | `protocol.rs` | Type-erased via `WidgetType` enum |
| `SettingsWidgetType` (enum) | `WidgetType` (enum) | `protocol.rs` | Reduced set for MVP |
| `SettingsWidgetLayout` | `WidgetLayout` | `protocol.rs` | Percentage-based coordinates |
| `SettingsStream` | `StreamConfig` | `protocol.rs` (future) | Per-destination config bundle |
| `Database` | App-level JSON config file | `src/config.rs` (new) | Persisted via `serde_json` |
| `Model` | Combination of `NodeManager` + Bridge state | Split across crates | No single God object |

### 1.2 Type Conversion Rules

| Swift type | Rust type | Notes |
|---|---|---|
| `UUID` | `String` (UUID format) | Use `uuid::Uuid::new_v4().to_string()` at creation |
| `@Published var` | Slint `in-out property` + Rust `set_*()` | No reactive binding; explicit push from Rust |
| `ObservableObject` | Plain struct + manual Bridge update | Call `bridge.set_scenes(...)` after mutation |
| `@ObservedObject` | N/A in Rust | UI reads from Bridge properties |
| `@EnvironmentObject var model: Model` | Global `Bridge` singleton | All state flows through Bridge |
| `Codable` | `#[derive(Serialize, Deserialize)]` | serde JSON for persistence |
| `CaseIterable` enum | `strum::EnumIter` or manual `ALL` const array | For UI picker population |
| `String(localized:)` | `@tr("...")` in Slint | i18n translation macro |
| `Optional<T>` | `Option<T>` | Direct mapping |
| `[T]` (Array) | `Vec<T>` in Rust, `[T]` model in Slint | Use `VecModel` for dynamic lists |

### 1.3 Streaming Protocol Types

| Moblin Swift | FCast Rust | Status |
|---|---|---|
| `SettingsStreamSrt` | `DestinationFamily::Srt { uri, latency, passphrase, pbkeylen }` | `[ ]` MVP-PHASE-8 |
| `SettingsStreamRtmp` | `DestinationFamily::Rtmp { uri }` | `[x]` done |
| `SettingsStreamRist` | `DestinationFamily::Rist { uri, ... }` (future) | `[ ]` v0.2.0 |
| `SettingsStreamWhip` | `DestinationFamily::Whip { uri }` (future) | `[ ]` v0.3.0 |
| `SrtLatency` enum | `u32` field on `Srt` variant | Direct value, not enum |
| `SrtEncryption` enum | `Option<SrtEncryption>` enum in Rust | `None` / `Aes128` / `Aes256` |
| `SettingsStreamSrtAdaptiveBitrate` | Separate config struct (future) | FastIRL algorithm params |
| `SettingsStreamSrtConnectionPriority` | Vec of priority entries | Multi-path SRT |

---

## 2. View Layer — SwiftUI Views → Slint Pages

### 2.1 Scene/Widget Views

| Moblin Swift View | FCast Slint page | Pattern |
|---|---|---|
| `ScenesSettingsView` | `scene_list_page.slint` | `for scene in Bridge.scenes: SceneRow {}` |
| `SceneSettingsView` | `scene_edit_page.slint` | `VerticalLayout` with sections |
| `SceneWidgetSettingsView` | `scene_widget_edit_page.slint` | Layout editor with `TouchArea` |
| `WidgetWizardSettingsView` | `wizard_widget_create_page.slint` | Multi-step panel |
| `WidgetTextSettingsView` | `widget_text_settings_page.slint` | Form with `LineEdit` + pickers |
| `WidgetImageSettingsView` | `widget_image_settings_page.slint` | Image picker + scale mode |
| `WidgetCropSettingsView` | `widget_crop_settings_page.slint` | 4 sliders (top/bottom/left/right) |
| `WidgetBrowserSettingsView` | `widget_browser_settings_page.slint` | URL input (v0.3.0) |
| `WidgetEffectsView` | `widget_effects_settings_page.slint` | Effect chain (v0.3.0) |

### 2.2 Streaming Protocol Views

| Moblin Swift View | FCast Slint page | Phase |
|---|---|---|
| `StreamRtmpSettingsView` | `protocol_rtmp_settings_page.slint` | 31-A |
| `StreamSrtSettingsView` | `protocol_srt_settings_page.slint` | 31-B |
| `StreamSrtAdaptiveBitrateSettingsView` | `protocol_srt_adaptive_bitrate_page.slint` | Post-31 |
| `StreamSrtConnectionPriority2View` | `protocol_srt_connection_priority_page.slint` | v0.3.0 |
| `StreamRistSettingsView` | `protocol_rist_settings_page.slint` | 31 |
| `StreamWhipSettingsView` | `protocol_whip_settings_page.slint` | v0.3.0 |

### 2.3 Control Bar Views

| Moblin Swift View | FCast Slint component | Notes |
|---|---|---|
| `ControlBarPortraitView` | `ui/components/control_bar.slint` | Scene buttons + quick actions |
| `ControlBarLandscapeView` | `ui/components/control_bar.slint` (responsive) | Same component, different layout |
| `QuickButtonsView` | Quick action buttons in `stream_page.slint` | Simplified subset |

---

## 3. Logic Layer — Swift Methods → Rust Functions/Commands

### 3.1 Scene Operations

| Moblin method | FCast Rust equivalent | Trigger |
|---|---|---|
| `model.selectScene(id:)` | `Command::SetScene { scene_id }` | User taps scene button |
| `model.resetSelectedScene()` | Internal: pick first enabled scene | App startup / scene deleted |
| `model.sceneUpdated()` | `NodeManager::handle_set_scene()` | After scene switch |
| `database.scenes.append(scene)` | `Command::CreateScene { scene }` | Create button |
| `database.scenes.removeAll { $0 === scene }` | `Command::RemoveScene { scene_id }` | Delete action |
| `database.scenes.move(fromOffsets:toOffset:)` | `Bridge.reorder-scenes(from, to)` | Drag reorder |

### 3.2 Widget Operations

| Moblin method | FCast Rust equivalent | Trigger |
|---|---|---|
| `CreateWidgetWizard.create()` | `Command::CreateWidget { widget }` | Wizard complete |
| `model.toggleWidgetOnOff()` | `Command::UpdateWidget` (toggle enabled) | Toggle switch |
| `model.appendWidgetToScene()` | `Command::UpdateScene` (add placement) | Add-to-scene button |
| `model.sceneUpdated()` (widget change) | Compositor pad reconfigure | Any widget layout change |
| Widget layout position change | `Command::UpdateWidgetLayout { ... }` | Drag/numeric edit |

### 3.3 Pipeline Reconfiguration

| Moblin concept | FCast GStreamer action |
|---|---|
| Scene switch → widget effects update | Remove/add compositor sink pads |
| Widget enabled/disabled | Add/remove single compositor sink pad |
| Widget layout change | Update `xpos`/`ypos`/`width`/`height`/`alpha` on pad |
| Quick-switch group same | Skip camera reattach, only swap overlays |
| Quick-switch group different | Reattach camera source + swap overlays |

---

## 4. Persistence — Swift Codable → Rust serde

### Moblin pattern

```swift
class Database: Codable {
    var scenes: [SettingsScene]
    var widgets: [SettingsWidget]
    var streams: [SettingsStream]
    // ... hundreds of settings
}
// Saved as JSON to app documents directory
```

### FCast pattern

```rust
// src/config.rs
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub scenes: Vec<Scene>,
    pub widgets: Vec<Widget>,
    pub streams: Vec<StreamConfig>,
    pub current_scene_id: Option<String>,
}

impl AppConfig {
    pub fn load() -> Self {
        // Read from app-internal JSON file
        // Android: Context.filesDir / "fcast_config.json"
    }

    pub fn save(&self) {
        // Write JSON to app-internal storage
    }
}
```

Trigger save on every mutation (debounced). Load on app startup.

---

## 5. Key Differences: Moblin vs FCast

| Aspect | Moblin | FCast |
|---|---|---|
| **Platform** | iOS/macOS (Swift, AVFoundation) | Android (Rust, GStreamer, JNI) |
| **UI framework** | SwiftUI (reactive bindings) | Slint (property-based, Rust callbacks) |
| **State management** | `@Published` + Combine | Manual Bridge property push |
| **Media pipeline** | AVFoundation + some Metal | GStreamer (full pipeline graph) |
| **Widget rendering** | Core Animation layers + Metal | GStreamer `compositor` element |
| **Persistence** | Swift `Codable` + UserDefaults | `serde_json` + Android internal storage |
| **Widget types** | 16 types | 4 MVP types (text, image, crop, clock) |
| **Chat/Alerts** | Full Twitch/Kick/YouTube integration | Not applicable |
| **Locale** | `.xcstrings` | Slint `@tr(...)` |

---

## 6. Migration Priority Order

1. **Data model** (protocol.rs structs) — must exist before UI or pipeline
2. **Command variants** — enable scene/widget CRUD
3. **NodeManager compositor** — enables widget rendering
4. **Scene list/edit UI** — user can create/manage scenes
5. **Widget wizard UI** — user can create widgets
6. **Widget layout editor** — user can position widgets
7. **Stream page scene buttons** — user can switch scenes live
8. **Persistence** — state survives restart
9. **Protocol UI pages** — SRT/RTMP settings forms

---

## 7. Clone Reference

For full Moblin model/pipeline code (not in `draft/moblin-ui/`):

```bash
# Clone to draft/moblin/ (already in .gitignore)
git clone https://github.com/eerimoq/moblin.git draft/moblin/
```

Key files to study:
- `Moblin/Various/Settings.swift` — all `Settings*` struct/class definitions
- `Moblin/Media/` — media pipeline (AVFoundation-based, for architecture reference)
- `Moblin/Model.swift` — scene selection, widget management, state machine
- `Moblin/SceneSelector.swift` — scene switching logic
