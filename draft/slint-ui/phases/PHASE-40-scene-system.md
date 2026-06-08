# Phase 40 — Scene System

> **Target version:** v0.2.0
> **Depends on:** Phase 31 (streaming protocols UI), MVP-PHASE-8 (SRT destination)
> **Status:** `[ ] Not started`
>
> **Moblin source analogues:**
> - `View/Settings/Scenes/ScenesSettingsView.swift` — scene list
> - `View/Settings/Scenes/Scene/SceneSettingsView.swift` — per-scene settings
> - `View/Settings/Scenes/Scene/SceneWidgetSettingsView.swift` — widget placement
> - `View/ControlBar/ControlBarPortraitView.swift` — scene quick-switch buttons
> - `View/ControlBar/QuickButtonsView.swift` — widget panel toggle
> - `Model.setCurrentScene()` / `SceneSelector` — scene switching logic
>
> **Architecture reference:** [deepwiki.com/eerimoq/moblin/4-scene-and-widget-system](https://deepwiki.com/eerimoq/moblin/4-scene-and-widget-system)
> **Mapping reference:** `draft/moblin-scene-widget-mapping.md`

---

## Goal

Introduce the **Scene** concept to FCast: a named, switchable collection
of widget placements that controls which overlays appear on the video
stream. After this phase:

1. Users can create/edit/delete/reorder scenes
2. Users can switch between scenes during a live stream
3. The scene switch triggers a GStreamer compositor reconfiguration
4. Scene state persists across app restarts

---

## Data Model

### Rust structs (`crates/migration-runtime/src/protocol.rs`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub widgets: Vec<SceneWidgetPlacement>,
    pub quick_switch_group: Option<u8>,  // 1–4 for instant switching
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneWidgetPlacement {
    pub widget_id: String,              // FK → Widget.id
    pub layout: WidgetLayout,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetLayout {
    pub x: f64,        // 0.0–100.0 (percentage of output)
    pub y: f64,
    pub width: f64,    // 1.0–100.0
    pub height: f64,
    pub rotation: f64, // 0.0–360.0 degrees
    pub opacity: f64,  // 0.0–1.0
}
```

### New `Command` variants

```rust
Command::SetScene { scene_id: String }
Command::CreateScene { scene: Scene }
Command::RemoveScene { scene_id: String }
Command::UpdateScene { scene: Scene }
```

### Slint Bridge additions

```slint
export struct SceneItem {
    id: string,
    name: string,
    enabled: bool,
    active: bool,       // currently selected scene
    widget-count: int,
}

export global Bridge {
    // ... existing ...
    in property <[SceneItem]> scenes;
    in-out property <string> current-scene-id;

    callback set-scene(string);           // scene_id
    callback create-scene(string);        // name
    callback remove-scene(string);        // scene_id
    callback reorder-scenes(int, int);    // from-idx, to-idx
}
```

---

## Tasks

### 40-A — Protocol extension

- [ ] Add `Scene`, `SceneWidgetPlacement`, `WidgetLayout` structs to `protocol.rs`
- [ ] Add `Command::SetScene`, `Command::CreateScene`, `Command::RemoveScene`, `Command::UpdateScene`
- [ ] Add `current_scene_id: Option<String>` to app state
- [ ] Serialize/deserialize scene list (JSON config file)
- [ ] Unit tests for scene serde round-trip

### 40-B — Scene switching in NodeManager

- [ ] On `Command::SetScene`: diff current vs new scene's widget list
- [ ] Remove compositor sink pads for deactivated widgets
- [ ] Add compositor sink pads for newly activated widgets
- [ ] Update `xpos`/`ypos`/`width`/`height`/`alpha` on retained pads
- [ ] Handle quick-switch group logic (same group → no camera reattach)
- [ ] Emit scene-changed state to Bridge

### 40-C — Panel routing + Bridge wiring

- [ ] Add `Panel::scene-list` and `Panel::scene-edit` to `bridge.slint` Panel enum
- [ ] Wire `Bridge.set-scene()` callback → Rust `Command::SetScene`
- [ ] Wire `Bridge.create-scene()` → `Command::CreateScene`
- [ ] Wire `Bridge.remove-scene()` → `Command::RemoveScene`
- [ ] Wire `Bridge.reorder-scenes()` → persistence reorder
- [ ] Update `Bridge.scenes` property from Rust on scene list change

### 40-D — Scene list page (`ui/pages/scene_list_page.slint`)

- [ ] `ListView` showing `SceneItem` rows with enable toggle
- [ ] Create button at bottom
- [ ] Swipe-to-delete (or long-press menu on Android)
- [ ] Drag-to-reorder (if Slint supports; else up/down arrows)
- [ ] Navigate to scene edit page on tap

### 40-E — Scene edit page (`ui/pages/scene_edit_page.slint`)

- [ ] Scene name editor (`LineEdit`)
- [ ] Video source section (camera picker — future, can stub)
- [ ] Mic override toggle + picker (future, can stub)
- [ ] Widget list section with per-widget enable toggle
- [ ] Add widget button → navigate to widget wizard (Phase 41)
- [ ] Quick-switch group picker (1–4 or none)
- [ ] Delete scene button

### 40-F — Scene buttons in stream page

- [ ] Add `HorizontalLayout` of `SceneButton` components to `stream_page.slint` bottom bar
- [ ] Each button shows scene name, highlights when active
- [ ] Tap → `Bridge.set-scene(id)`
- [ ] Only show scenes where `enabled == true`

### 40-G — Persistence

- [ ] Save scene list to local JSON config on every mutation
- [ ] Load scene list on app startup
- [ ] Default: one scene named "Main" with no widgets

---

## Moblin source reference

### `ScenesSettingsView.swift` — Scene list

```swift
ForEach(database.scenes) { scene in
    SceneItemView(database: database, scene: scene)
}
.onMove { froms, to in
    database.scenes.move(fromOffsets: froms, toOffset: to)
}
CreateButtonView {
    let scene = SettingsScene(name: makeUniqueName(...))
    database.scenes.append(scene)
}
```

→ Slint: `for scene in Bridge.scenes: SceneRow { ... }`

### `SceneSettingsView.swift` — Per-scene

Key sections:
1. **Video source** — camera picker with stabilization/fill-frame options
2. **Quick switch group** — `Picker` with groups 1–4
3. **Mic** — optional override with mic picker
4. **Widgets** — ordered list with toggle/reorder/delete

→ Slint: `scene_edit_page.slint` with `VerticalLayout` sections

### `ControlBarPortraitView.swift` — Scene buttons

Scene switching is done via `QuickButtons` in the control bar. In FCast
this maps to dedicated `SceneButton` components in the stream page
bottom bar.

### `Model.selectScene(id:)` / `SceneSelector`

```swift
func selectScene(id: UUID) {
    sceneSelector.selectedSceneId = id
    resetSelectedScene(changeScene: true, attachCamera: true)
}
```

→ Rust: `Command::SetScene { scene_id }` → `NodeManager` compositor reconfig

---

## Acceptance Criteria

1. Scene list page shows all scenes with create/delete/reorder
2. Scene edit page allows naming, widget assignment, quick-switch group
3. Scene buttons in stream page switch active scene
4. GStreamer compositor updates overlays on scene switch
5. Scene state persists across app restart
6. Quick-switch group scenes switch without camera reattachment delay

---

## Out of Scope (deferred to Phase 41+)

- Widget type definitions and creation wizard
- GStreamer compositor element creation (widget rendering)
- Widget layout drag-handle editor
- Scene transitions (crossfade/cut animation)
- Auto scene switcher
- Remote scene support
