# STEP-3C — `UpdateWidgetLayout` + CRUD

## Live layout updates → control points

```rust
Command::UpdateWidgetLayout { scene_id, widget_id, layout } => {
    if let Some(scene) = self.scenes.get_mut(&scene_id) {
        if let Some(p) = scene.widgets.iter_mut().find(|p| p.widget_id == widget_id) {
            p.layout = layout.clone();
        }
    }
    if self.current_scene_id.as_deref() == Some(&scene_id) {
        let link_id = format!("scene-w-{widget_id}");
        self.update_slot_layout(&link_id, &layout, /*zorder*/ 0);
    }
    CommandResult::Success
}
```

`update_slot_layout` issues `AddControlPoint { mode: Set }` per field — the
control system already sets pad props each tick, so the overlay moves on the
running stream.

## Scene / Widget CRUD

`CreateScene`/`UpdateScene`/`RemoveScene`/`CreateWidget`/`UpdateWidget`/`RemoveWidget`
mutate the registry maps and (STEP-9) persist. Only `SetScene`/`UpdateWidgetLayout`
touch the live graph.

`RemoveWidget` must also: disconnect any active link for it, and drop it from
every scene's placement list:

```rust
Command::RemoveWidget { widget_id } => {
    let link_id = format!("scene-w-{widget_id}");
    if self.active_widget_links.remove(&link_id).is_some() {
        let _ = self.disconnect(&link_id);
    }
    for scene in self.scenes.values_mut() {
        scene.widgets.retain(|p| p.widget_id != widget_id);
    }
    self.widgets.remove(&widget_id);
    CommandResult::Success
}
```

## Crop application (camera source)

See STEP-4 §4e — `apply_crops(&scene)` sets `videocrop` props on the camera
source (resets to 0 when the scene has no crop widget).

→ Next: [STEP-3D-switching-and-tests.md](STEP-3D-switching-and-tests.md)
