# STEP-3B — `SetScene` → existing primitives

```rust
// dispatch:
Command::SetScene { scene_id } => self.apply_scene(&scene_id),
```

```rust
impl NodeManager {
    const SCENE_MIXER_ID: &'static str = "scene-mixer";

    fn apply_scene(&mut self, scene_id: &str) -> CommandResult {
        let Some(scene) = self.scenes.get(scene_id).cloned() else {
            return CommandResult::Error(format!("no scene {scene_id}"));
        };

        // 1. Desired widgets (enabled, non-crop).
        let desired: Vec<&SceneWidgetPlacement> = scene.widgets.iter()
            .filter(|p| p.enabled)
            .filter(|p| !matches!(
                self.widgets.get(&p.widget_id).map(|w| &w.widget_type),
                Some(WidgetType::Crop { .. })))
            .collect();

        // 2. Crops drive videocrop on the camera source (STEP-4).
        self.apply_crops(&scene);

        // 3. Diff vs active links → disconnect removed.
        let desired_ids: std::collections::HashSet<&str> =
            desired.iter().map(|p| p.widget_id.as_str()).collect();
        let to_remove: Vec<String> = self.active_widget_links.iter()
            .filter(|(_, wid)| !desired_ids.contains(wid.as_str()))
            .map(|(link, _)| link.clone()).collect();
        for link_id in to_remove {
            let _ = self.disconnect(&link_id);
            self.active_widget_links.remove(&link_id);
        }

        // 4. Add / reconfigure desired.
        for placement in desired {
            let link_id = format!("scene-w-{}", placement.widget_id);
            if !self.active_widget_links.contains_key(&link_id) {
                self.ensure_widget_source(&placement.widget_id)?;        // STEP-4
                self.connect_with_config(
                    &link_id,
                    &widget_source_id(&placement.widget_id),
                    Self::SCENE_MIXER_ID,
                    slot_config_from_layout(&placement.layout, placement.zorder),
                )?;
                self.active_widget_links.insert(link_id, placement.widget_id.clone());
            } else {
                self.update_slot_layout(&link_id, &placement.layout, placement.zorder);
            }
        }

        self.current_scene_id = Some(scene_id.to_string());
        CommandResult::Success
    }
}
```

## Layout → mixer slot config

```rust
fn slot_config_from_layout(layout: &WidgetLayout, zorder: i32)
    -> HashMap<String, serde_json::Value>
{
    use serde_json::json;
    HashMap::from([
        ("video::x".into(),      json!(layout.x)),
        ("video::y".into(),      json!(layout.y)),
        ("video::width".into(),  json!(layout.width)),
        ("video::height".into(), json!(layout.height)),
        ("video::zorder".into(), json!(zorder)),
        ("video::alpha".into(),  json!(layout.opacity)),
        // rotation omitted — `compositor` pads can't rotate (INDEX §4).
    ])
}
```

> `connect_with_config`/`disconnect`/`update_slot_layout` wrap the manager's
> **existing** `Connect`/`Disconnect`/`AddControlPoint` handling — match the real
> internal method names when implementing. SetScene emits **no new primitive**.

→ Next: [STEP-3C-layout-and-crud.md](STEP-3C-layout-and-crud.md)
