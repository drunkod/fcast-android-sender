# Phase 41 — Widget System

> **Target version:** v0.2.0
> **Depends on:** Phase 40 (scene system — scene data model must exist)
> **Status:** `[ ] Not started`
>
> **Moblin source analogues:**
> - `View/Settings/Scenes/Widgets/WidgetsSettingsView.swift` — global widget list
> - `View/Settings/Scenes/Widgets/Widget/WidgetWizardSettingsView.swift` — creation wizard
> - `View/Settings/Scenes/Widgets/Widget/WidgetSettingsView.swift` — layout editor
> - `View/Settings/Scenes/Widgets/Widget/Text/WidgetTextSettingsView.swift` — text config
> - `View/Settings/Scenes/Widgets/Widget/Image/WidgetImageSettingsView.swift` — image config
> - `View/Settings/Scenes/Widgets/Widget/Crop/WidgetCropSettingsView.swift` — crop config
> - Model: `SettingsWidget`, `SettingsWidgetType`, `SettingsWidgetLayout`
>
> **Architecture reference:** [deepwiki.com/eerimoq/moblin/4-scene-and-widget-system](https://deepwiki.com/eerimoq/moblin/4-scene-and-widget-system)
> **Mapping reference:** `draft/moblin-scene-widget-mapping.md`

---

## Goal

Introduce the **Widget** concept to FCast: overlay elements rendered on
top of the video pipeline via GStreamer's `compositor` element. After
this phase:

1. Users can create widgets (text, image, crop, clock) via a wizard
2. Widgets are placed within scenes with configurable layout
3. GStreamer compositor renders active widgets onto the output stream
4. Widget state persists across app restarts

---

## MVP Widget Types (v0.2.0)

| Type | Purpose | GStreamer element | Config fields |
|------|---------|-------------------|---------------|
| **Text** | Stream branding, labels, titles | `textoverlay` or `pango` render → `appsrc` | `format: String` (supports `{time}`, `{date}`, custom text) |
| **Image** | Logo, watermark, static overlay | `gdkpixbufoverlay` or `imagefreeze` → `appsrc` | `asset_id: String` (reference to stored image) |
| **Crop** | Reframe video without source change | `videocrop` on main pipeline | `top, bottom, left, right: f64` (pixels or %) |
| **Clock** | Timestamp / elapsed time overlay | `clockoverlay` / `timeoverlay` | `format: String` (strftime-style) |

---

## Data Model

### Rust structs (`crates/migration-runtime/src/protocol.rs`)

```rust
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
    Text {
        format: String,
        font_size: Option<u32>,
        color: Option<String>,      // hex RGBA
    },
    Image {
        asset_id: String,
        scale_mode: Option<String>, // "fit" | "fill" | "stretch"
    },
    Crop {
        top: f64,
        bottom: f64,
        left: f64,
        right: f64,
    },
    Clock {
        format: String,             // e.g. "%H:%M:%S"
        font_size: Option<u32>,
        color: Option<String>,
    },
}
```

### New `Command` variants

```rust
Command::CreateWidget { widget: Widget }
Command::RemoveWidget { widget_id: String }
Command::UpdateWidget { widget: Widget }
Command::UpdateWidgetLayout {
    scene_id: String,
    widget_id: String,
    layout: WidgetLayout,
}
```

### Slint Bridge additions

```slint
export struct WidgetItem {
    id: string,
    name: string,
    widget-type: string,   // "text" | "image" | "crop" | "clock"
    enabled: bool,
}

export enum WidgetTypeChoice {
    text,
    image,
    crop,
    clock,
}

export global Bridge {
    // ... existing ...
    in property <[WidgetItem]> widgets;

    callback create-widget(string, WidgetTypeChoice);  // name, type
    callback remove-widget(string);                     // widget_id
    callback update-widget-layout(string, string, float, float, float, float);
    //                           scene_id, widget_id, x, y, w, h
}
```

---

## Tasks

### 41-A — Protocol extension

- [ ] Add `Widget` struct to `protocol.rs`
- [ ] Add `WidgetType` enum with `Text`, `Image`, `Crop`, `Clock` variants
- [ ] Add `Command::CreateWidget`, `Command::RemoveWidget`, `Command::UpdateWidget`
- [ ] Add `Command::UpdateWidgetLayout`
- [ ] Unit tests for widget/type serde round-trip
- [ ] Validate widget type config (e.g. crop values ≥ 0, format non-empty)

### 41-B — GStreamer compositor integration

- [ ] Create `CompositorNode` in `crates/migration-runtime/src/nodes/compositor.rs`
- [ ] Insert `compositor` element between source and encoder in pipeline
- [ ] For each active widget in current scene, create a compositor sink pad
- [ ] Set pad properties from `WidgetLayout`: `xpos`, `ypos`, `width`, `height`, `alpha`
- [ ] Handle dynamic pad add/remove on scene switch (from Phase 40)

### 41-C — Widget renderers

#### Text renderer
- [ ] Create `TextWidgetRenderer` that renders format string to RGBA buffer
- [ ] Support variable substitution: `{time}`, `{date}`, custom static text
- [ ] Push rendered buffer to `appsrc` connected to compositor sink pad
- [ ] Alternative: use `textoverlay` element directly (simpler, less flexible)

#### Image renderer
- [ ] Load image from asset storage (app internal storage or bundled)
- [ ] Decode to RGBA via `image` crate
- [ ] Create `imagefreeze` → `appsrc` chain for static image
- [ ] Scale image to match `WidgetLayout` dimensions

#### Crop handler
- [ ] Apply `videocrop` element properties from `WidgetType::Crop` config
- [ ] Crop is applied to main video source, not as compositor overlay
- [ ] Update crop values dynamically via `Command::UpdateWidget`

#### Clock renderer
- [ ] Use GStreamer `clockoverlay` or `timeoverlay` element
- [ ] Configure `time-format` property from `WidgetType::Clock.format`
- [ ] Position via `text-x`, `text-y` relative properties

### 41-D — Widget creation wizard (`ui/pages/wizard_widget_create_page.slint`)

- [ ] Step 1: Type picker (radio buttons / segmented control for text/image/crop/clock)
- [ ] Step 2: Name input (`LineEdit` with uniqueness validation)
- [ ] Step 3: Type-specific config (conditional section based on chosen type)
- [ ] Step 4: Scene assignment (pick which scenes include this widget)
- [ ] "Create" button → `Bridge.create-widget()` callback

### 41-E — Widget settings pages

#### Text widget (`ui/pages/widget_text_settings_page.slint`)
- [ ] Format string input (`LineEdit`)
- [ ] Font size picker (preset values: 12, 16, 24, 32, 48)
- [ ] Color picker (preset palette matching theme)
- [ ] Live preview (if feasible — show formatted text)

#### Image widget (`ui/pages/widget_image_settings_page.slint`)
- [ ] Image picker (from device gallery or bundled assets)
- [ ] Scale mode selector (fit / fill / stretch)
- [ ] Image preview thumbnail

#### Crop widget (`ui/pages/widget_crop_settings_page.slint`)
- [ ] Top/Bottom/Left/Right sliders (0–50% each)
- [ ] Live preview showing crop region overlay

#### Clock widget (inline in wizard — minimal config)
- [ ] Format string preset picker (`HH:MM:SS`, `HH:MM`, custom)
- [ ] Font size + color (same as text widget)

### 41-F — Widget layout editor (`ui/pages/scene_widget_edit_page.slint`)

- [ ] Visual canvas showing widget position (rectangle on preview area)
- [ ] Drag via `TouchArea` to reposition (updates `x`, `y`)
- [ ] Resize handles (corner drag → updates `width`, `height`)
- [ ] Numeric inputs for precise values (x, y, w, h, rotation, opacity)
- [ ] Alignment grid (3×3: top-left through bottom-right)
- [ ] Save/Load layout buttons (quick copy between widgets)
- [ ] Opacity slider (0.0–1.0)

### 41-G — Persistence

- [ ] Save widget pool to JSON config alongside scene list
- [ ] Load widgets on app startup
- [ ] Handle orphaned widgets (widget exists but not in any scene)
- [ ] Image asset management (copy to app-internal storage on import)

---

## Moblin Source Reference

### `WidgetWizardSettingsView.swift` — Creation wizard

```swift
Picker("Type", selection: $createWidgetWizard.type) {
    ForEach(SettingsWidgetType.allCases, id: \.self) { type in
        Text(type.toString())
    }
}
// Then switch on type for type-specific config view:
switch createWidgetWizard.type {
    case .text: WidgetWizardTextSettingsView(...)
    case .browser: WidgetWizardBrowserSettingsView(...)
    case .image: WidgetWizardImageSettingsView(...)
    // ...
}
```

→ Slint: Conditional `if widget-type == WidgetTypeChoice.text:` blocks

### `WidgetSettingsView.swift` — Layout editor

Key components:
- `AlignmentOptionView` — 3×3 alignment grid
- `PositionEditView` — directional arrows for x/y adjustment
- `SizeEditView` — width/height controls
- `SaveLoadLayoutView` — clipboard for layout presets

```swift
struct WidgetLayoutView {
    @Binding var layout: SettingsWidgetLayout
    // x, y as percentage (0–100)
    // width, height as percentage
    // rotation (0–360)
    // opacity (0–1)
    // alignment (9 positions)
}
```

→ Slint: `scene_widget_edit_page.slint` with `TouchArea` drag + numeric inputs

### `WidgetTextSettingsView.swift`

```swift
Form {
    Section { TextField("Format", text: $text.formatString) }
    Section { FontSizePicker(...) }
    Section { ColorPicker(...) }
}
```

→ Slint: `widget_text_settings_page.slint`

---

## GStreamer Pipeline Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                     compositor (or glvideomixer)              │
│                                                              │
│  sink_0 ← camera source (main video, z-order=0)             │
│  sink_1 ← text widget appsrc (z-order=1, positioned)        │
│  sink_2 ← image widget appsrc (z-order=2, positioned)       │
│  sink_3 ← clock overlay (z-order=3, positioned)             │
│  ...                                                         │
│                                                              │
│  src → videoconvert → encoder → mux → sink (RTMP/SRT/etc.)  │
└──────────────────────────────────────────────────────────────┘
```

### Compositor pad properties (per widget)

| Property | From WidgetLayout | Notes |
|----------|-------------------|-------|
| `xpos` | `layout.x * output_width / 100.0` | Absolute pixel position |
| `ypos` | `layout.y * output_height / 100.0` | Absolute pixel position |
| `width` | `layout.width * output_width / 100.0` | Scale widget to this width |
| `height` | `layout.height * output_height / 100.0` | Scale widget to this height |
| `alpha` | `layout.opacity` | 0.0–1.0 transparency |
| `zorder` | Widget index in scene's widget list | Higher = renders on top |

### Crop widget (special case)

Crop does NOT use the compositor — it applies `videocrop` properties
directly to the main source pipeline:

```rust
// In compositor node setup:
if widget_type == WidgetType::Crop { top, bottom, left, right } {
    videocrop.set_property("top", (top * height / 100.0) as i32);
    videocrop.set_property("bottom", (bottom * height / 100.0) as i32);
    videocrop.set_property("left", (left * width / 100.0) as i32);
    videocrop.set_property("right", (right * width / 100.0) as i32);
}
```

---

## Acceptance Criteria

1. Widget creation wizard allows creating text, image, crop, and clock widgets
2. Widgets appear in scene's widget list and can be toggled on/off
3. Widget layout editor allows positioning (x, y, width, height)
4. GStreamer compositor renders active widgets on stream output
5. Text widget displays formatted text at specified position
6. Image widget displays static image at specified position
7. Crop widget reframes the video source
8. Clock widget shows current time with configurable format
9. Widget pool persists across app restart
10. Removing a widget from all scenes doesn't crash; orphans are cleaned up

---

## Out of Scope (deferred to v0.3.0)

- Browser source widget (requires WebView texture bridge)
- Video effect widgets (filter chains)
- Map/GPS widget
- Scene transitions (crossfade between scene compositor states)
- Widget animation (moving widgets, fade in/out)
- Slideshow widget (auto-rotating set)
- Auto scene switcher
