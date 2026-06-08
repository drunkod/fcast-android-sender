# SwiftUI to Slint migration guide for FCast Android sender

This guide is based on:

- Moblin copied SwiftUI UI: `draft/moblin-ui/`
- Slint source/docs clone: `draft/slint/`
- Current FCast Android Slint UI: `senders/android/ui/main.slint`

## Current target constraints

FCast Android sender is already a Rust + Slint Android app:

- `senders/android/Cargo.toml` uses `slint` with `backend-android-activity-06`, `compat-1-2`, and `std`.
- `senders/android/src/lib.rs` exposes Android `android_main` and wires Rust callbacks to Slint globals.
- `senders/android/ui/main.slint` has one `MainWindow`, an exported global `Bridge`, and state-driven views:
  - `Disconnected`
  - `Connecting`
  - `SelectingSettings`
  - `WaitingForMedia`
  - `Casting`

Slint Android supports Rust as the app language. The migration should therefore reimplement Moblin SwiftUI design concepts in `.slint`, not try to compile SwiftUI.

## Key Slint patterns to use

From Slint docs:

- Use `.slint` markup for declarative UI and Rust callbacks/properties for logic.
- Keep UI, Rust code, and assets separated (`ui/`, `src/`, `ui/images`).
- Use `VerticalLayout`, `HorizontalLayout`, and `GridLayout` instead of manual coordinates for most screens.
- Use properties with `in`, `out`, and `in-out` qualifiers for component data flow.
- Use `export global Bridge`/globals for app-wide state and callbacks exposed to Rust.
- Use `for item in model` for repeated elements, but prefer `ListView` for long lists.
- Use callbacks for backend actions (`clicked => Bridge.stop-casting()` maps to Rust `on_stop_casting`).
- Use `states` and transitions for visual modes such as hidden/shown control bars, casting/disconnected, or selected tabs.
- Mark user-visible strings with `@tr("...")` if localization is required.
- Avoid binding loops; prefer direct property bindings over `changed` handlers.
- Elements outside layouts need explicit `width`/`height`; elements inside layouts are sized automatically.

## SwiftUI concept mapping

| SwiftUI / Moblin | Slint / FCast target |
|---|---|
| `@EnvironmentObject var model: Model` | `export global Bridge` plus focused Rust-owned structs/models |
| `@ObservedObject var database: Database` | `in-out property` values, Rust `VecModel`, and persistent config structs |
| `NavigationStack` / `NavigationLink` | `current-page`/`current-panel` enum, conditional components, side menu or stack-like page router |
| `Form` / `Section` | `VerticalLayout` with `SettingsSection` components and row components |
| `List` / `ForEach` | `ListView` or `for item in model` for short static lists |
| `Button` | `std-widgets.slint Button` or custom `ActionButton` wrapping `TouchArea` |
| `Toggle` | `CheckBox` / `Switch` from std widgets, or custom switch component |
| `Picker` | `ComboBox`, segmented button row, or custom `ChoiceRow` |
| `Slider` | `Slider` std widget with `in-out` value binding |
| `TextField` | `LineEdit` / `TextEdit` |
| `VStack` / `HStack` | `VerticalLayout` / `HorizontalLayout` |
| `ZStack` / overlays | nested `Rectangle`/`Image`/`Text` with explicit overlay placement |
| `GeometryReader` | parent width/height bindings and responsive layout conditions |
| `.sheet` / `.alert` | modal-like overlay component controlled by enum/bool properties |
| `AVFoundation`/camera preview | Rust/GStreamer-provided image/video surface/status; keep separate from UI shell |
| `String(localized:)` / `.xcstrings` | `@tr(...)` and Slint translation extraction |

## Migration architecture

Use a Slint-first architecture rather than copying Moblin's `Model`/`Database` directly.

```text
senders/android/
  ui/
    main.slint                 # MainWindow + page router + Bridge re-exports
    theme.slint                # colors, spacing, text sizes
    components/
      buttons.slint
      settings_rows.slint
      status_badges.slint
      control_bar.slint
      overlay.slint
    pages/
      connect_page.slint
      casting_page.slint
      settings_page.slint
      debug_page.slint
      codec_page.slint
      receivers_page.slint
  src/
    lib.rs                     # Android app entry and callback wiring
    ui_state.rs                # Rust state structs/models feeding Slint
```

### Recommended globals

Keep the current `Bridge`, but split large state into named globals once it grows:

```slint
export global Bridge {
    in-out property <AppState> app-state;
    in-out property <Panel> active-panel;
    in property <[ReceiverItem]> receivers;
    in property <[QuickAction]> quick-actions;

    callback connect-receiver(string);
    callback start-casting(scale-width: int, scale-height: int, max-framerate: int);
    callback stop-casting();
    callback open-panel(Panel);
    callback close-panel();
}
```

Use Slint structs/enums for UI data:

```slint
export enum Panel { none, settings, debug, codec, receivers }
export struct QuickAction { id: string, title: string, enabled: bool, active: bool }
export struct StatusItem { label: string, value: string, severity: string }
```

## Moblin groups and migration strategy

### 1. `View/Utils` → Slint reusable components

Priority: first.

Why: these files are the least tied to streaming internals and define reusable visual patterns.

Port into:

- `ui/components/buttons.slint`
- `ui/components/settings_rows.slint`
- `ui/components/editors.slint`
- `ui/components/status_badges.slint`

Examples:

- `ButtonView.swift`, `BorderlessButtonView.swift` → `PrimaryButton`, `IconButton`, `TextButton`
- `SliderView.swift` → `SettingsSliderRow`
- `TextEditView.swift`, `TextEditNavigationView.swift`, `ValueEditView.swift` → `SettingsTextRow` + modal/page editor
- `PositionEditView.swift`, `SizeEditView.swift` → numeric row components
- `QrCodeImageView.swift` → backend-provided image or generated asset binding

### 2. `Settings` → Slint page router + settings rows

Priority: second, selective.

Moblin has 190 settings files. Do not port every feature blindly; create FCast-specific pages first:

- Receiver/cast destination settings
- Capture/quality settings
- Codec/debug settings
- Network/connection settings
- About/logs

Use a Slint `SettingsPage` with a section model or explicit sections. Avoid deep SwiftUI-style nested navigation until the basic router exists.

### 3. `ControlBar` → FCast casting control bar

Priority: third.

Moblin control bar maps well to FCast's main actions:

- Connect/receiver picker
- Scan QR
- Start/stop casting
- Encoder/codec test shortcut
- Debug panel toggle
- Battery/status badges

Implement as:

- `ControlBarPortrait`
- `ControlBarLandscape`
- `QuickActionButton`
- `StreamButton` equivalent renamed to `CastButton`

Use parent `width > height` or Rust-provided orientation to switch layout.

### 4. `Stream` overlays → FCast status overlay

Priority: fourth.

Moblin overlays are backend-heavy. Port the concept, not the implementation:

- connection status
- encoder selected
- bitrate/framerate/resolution
- receiver name
- recording/casting duration
- debug messages

Slint pattern: full-screen root rectangle with preview/status content, then overlay child rectangles aligned at edges.

### 5. `MainView` → Slint `MainWindow`

Priority: after components/pages exist.

Current `MainWindow` already switches on `Bridge.app-state`. Extend this into a richer page/router architecture rather than recreating Moblin's SwiftUI root exactly.

## Scene & Widget System Mappings

> Added for Phase 40 (scenes) and Phase 41 (widgets).
> See also: `draft/moblin-scene-widget-mapping.md`, `draft/moblin-fcast-version-map.md`

### Scene system

| SwiftUI / Moblin | Slint / FCast target |
|---|---|
| `ScenesSettingsView` (scene list + create + reorder) | `scene_list_page.slint` — `for scene in Bridge.scenes: SceneRow { name: scene.name; active: scene.active; }` |
| `SceneSettingsView` (per-scene: source, mic, widgets) | `scene_edit_page.slint` — `VerticalLayout` with sections for video-source, mic, widget-list, quick-switch |
| `SceneWidgetSettingsView` (widget layout in scene) | `scene_widget_edit_page.slint` — layout editor with `TouchArea` drag handles for position/size |
| Scene button bar in `ControlBarPortraitView` | `HorizontalLayout` of `SceneButton { name: string; active: bool; clicked => { Bridge.set-scene(id); } }` in `stream_page.slint` bottom bar |
| `model.selectScene(id:)` / `SceneSelector` | `Bridge.set-scene(scene-id)` callback → Rust `Command::SetScene { scene_id }` |
| `model.sceneUpdated()` (pipeline reconfigure) | `NodeManager::handle_set_scene()` → diff compositor pads |
| `database.scenes.move(fromOffsets:toOffset:)` | `Bridge.reorder-scenes(from-idx, to-idx)` callback → persist reorder |
| `SettingsScene.quickSwitchGroup` (1–4) | `Scene.quick_switch_group: Option<u8>` — same group = instant switch (no camera reattach) |

### Widget system

| SwiftUI / Moblin | Slint / FCast target |
|---|---|
| `WidgetWizardSettingsView` (type picker → config wizard) | `wizard_widget_create_page.slint` — step 1: type picker, step 2: name, step 3: config, step 4: scene assignment |
| `WidgetSettingsView` / `WidgetLayoutView` (position/size/alignment) | `scene_widget_edit_page.slint` — `TouchArea` drag for x/y, resize handles for w/h, numeric inputs, 3×3 alignment grid |
| `WidgetTextSettingsView` (format string, font, color) | `widget_text_settings_page.slint` — `LineEdit` for format, preset font-size picker, color palette |
| `WidgetImageSettingsView` (image picker, scale mode) | `widget_image_settings_page.slint` — gallery picker, fit/fill/stretch selector |
| `WidgetCropSettingsView` (top/bottom/left/right) | `widget_crop_settings_page.slint` — 4 sliders (0–50%) |
| `SettingsWidgetType.allCases` (16 types) | `WidgetTypeChoice` enum: `text`, `image`, `crop`, `clock` (MVP subset) |
| Widget overlay rendering (AVFoundation layers) | GStreamer `compositor` element — one sink pad per active widget, positioned via `xpos`/`ypos`/`width`/`height`/`alpha` pad properties |
| `model.setCurrentScene()` → widget effects update | `Command::SetScene` → compositor pad add/remove/update |

### Bridge additions for scenes/widgets

```slint
// In bridge.slint — Scene/Widget globals

export struct SceneItem {
    id: string,
    name: string,
    enabled: bool,
    active: bool,
    widget-count: int,
}

export struct WidgetItem {
    id: string,
    name: string,
    widget-type: string,  // "text" | "image" | "crop" | "clock"
    enabled: bool,
}

export enum WidgetTypeChoice { text, image, crop, clock }

export global Bridge {
    // Scene properties
    in property <[SceneItem]> scenes;
    in-out property <string> current-scene-id;

    // Scene callbacks
    callback set-scene(string);
    callback create-scene(string);
    callback remove-scene(string);
    callback reorder-scenes(int, int);

    // Widget properties
    in property <[WidgetItem]> widgets;

    // Widget callbacks
    callback create-widget(string, WidgetTypeChoice);
    callback remove-widget(string);
    callback update-widget-layout(string, string, float, float, float, float);
    //                           scene_id, widget_id, x, y, w, h
}
```

### GStreamer compositor pattern (Rust side)

```rust
// When Command::SetScene is received:
fn handle_set_scene(&mut self, scene_id: &str) {
    let scene = self.config.scenes.iter().find(|s| s.id == scene_id);
    let active_widgets = scene.widgets.iter().filter(|w| w.enabled);

    // Diff current compositor pads vs new widget set
    for widget_placement in active_widgets {
        let pad = compositor.request_pad_simple("sink_%u");
        pad.set_property("xpos", (widget_placement.layout.x * width / 100.0) as i32);
        pad.set_property("ypos", (widget_placement.layout.y * height / 100.0) as i32);
        pad.set_property("width", (widget_placement.layout.width * width / 100.0) as i32);
        pad.set_property("height", (widget_placement.layout.height * height / 100.0) as i32);
        pad.set_property("alpha", widget_placement.layout.opacity);
    }
}
```

---

## Risks and blockers

- Moblin SwiftUI views depend on 233 `@EnvironmentObject` references and 749 `@ObservedObject` references in copied files.
- Settings UI is large: 190 of 279 files are settings views.
- Direct compile reuse is impossible in FCast Android because SwiftUI is iOS/macOS and FCast Android uses Rust + Slint.
- Moblin UI references Apple frameworks and app state types not available in Android.
- Some Moblin features have no FCast equivalent yet: Twitch/Kick chat, scenes/widgets, GoPro/DJI, Tesla, cat printers, watch support.
- Need Rust-side state model design before porting deep pages.

## Validation plan

1. Run Slint compile/build through the existing Android sender build path.
2. For quick `.slint` syntax checks, use the repo's normal build or Slint viewer where available.
3. For visual/runtime debugging, Slint MCP can be enabled when Slint >= 1.17 is in use:
   ```sh
   SLINT_EMIT_DEBUG_INFO=1 SLINT_MCP_PORT=9315 cargo run -p <app> --features slint/mcp
   ```
4. On Android, use the existing Gradle/CI debug APK build and device testing.

