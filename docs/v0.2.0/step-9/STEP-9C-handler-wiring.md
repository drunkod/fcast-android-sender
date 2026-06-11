# STEP-9C — Handler wiring (`src/android_main.rs`)

## Hydrate on startup (mirrors v0.1.0 `srt_destination`)

```rust
let mut reg = SceneRegistry::default();
for s in backend_cfg.scenes.clone()  { reg.scenes.insert(s.id.clone(), s); }
for w in backend_cfg.widgets.clone() { reg.widgets.insert(w.id.clone(), w); }
reg.current_scene_id = backend_cfg.current_scene_id.clone();
if reg.scenes.is_empty() {
    let s = default_main_scene();
    reg.scenes.insert(s.id.clone(), s);
}
// store reg in shared state, then:
push_scenes(&ui.as_weak(), &reg);
```

## Representative callbacks (the rest follow the same shape)

```rust
ui.global::<Bridge>().on_create_scene({
    let ui_weak = ui.as_weak();
    move |name| {
        with_registry(|reg| {
            let id = uuid::Uuid::new_v4().to_string();
            reg.scenes.insert(id.clone(), Scene {
                id, name: name.to_string(), enabled: true,
                widgets: vec![], quick_switch_group: None,
            });
            persist_scenes(reg);
            push_scenes(&ui_weak, reg);
        });
    }
});

ui.global::<Bridge>().on_set_scene({
    let ui_weak = ui.as_weak();
    move |scene_id| {
        let id = scene_id.to_string();
        with_registry(|reg| {
            let _ = migration_runtime::runtime::handle_command(
                Command::SetScene { scene_id: id.clone() });
            reg.current_scene_id = Some(id.clone());
            persist_scenes(reg);
            push_scenes(&ui_weak, reg);
        });
    }
});

ui.global::<Bridge>().on_open_scene_edit({
    let ui_weak = ui.as_weak();
    move |scene_id| {
        let id = scene_id.to_string();
        let placements = with_registry(|reg| placement_items(reg, &id));
        let _ = ui_weak.upgrade_in_event_loop(move |u| {
            let b = u.global::<Bridge>();
            b.set_editing_scene_id(id.clone().into());
            b.set_editing_scene_widgets(std::rc::Rc::new(slint::VecModel::from(placements)).into());
            u.global::<PanelBridge>().invoke_push(Panel::SceneEdit);
        });
    }
});

ui.global::<Bridge>().on_create_widget({
    let ui_weak = ui.as_weak();
    move || {
        let Some(u) = ui_weak.upgrade() else { return };
        let b = u.global::<Bridge>();
        let wt = match b.get_draft_widget_type() {
            WidgetTypeChoice::Image => WidgetType::Image {
                asset_id: b.get_draft_widget_image_path().to_string(),
                scale_mode: Some(["fit","fill","stretch"][b.get_draft_widget_scale_idx() as usize].into()),
            },
            WidgetTypeChoice::Crop => WidgetType::Crop {
                top: b.get_draft_crop_top() as f64, bottom: b.get_draft_crop_bottom() as f64,
                left: b.get_draft_crop_left() as f64, right: b.get_draft_crop_right() as f64,
            },
            WidgetTypeChoice::Clock => WidgetType::Clock {
                format: b.get_draft_widget_clock_format().to_string(),
                font_size: Some(b.get_draft_widget_font_size() as u32), color: None,
            },
            _ => WidgetType::Text {
                format: b.get_draft_widget_text_format().to_string(),
                font_size: Some(b.get_draft_widget_font_size() as u32), color: None,
            },
        };
        let widget = Widget { id: uuid::Uuid::new_v4().to_string(),
            name: b.get_draft_widget_name().to_string(), widget_type: wt, enabled: true };
        with_registry(|reg| {
            let wid = widget.id.clone();
            reg.widgets.insert(wid.clone(), widget.clone());
            let _ = migration_runtime::runtime::handle_command(Command::CreateWidget { widget });
            let sid = b.get_editing_scene_id().to_string();
            if let Some(scene) = reg.scenes.get_mut(&sid) {
                scene.widgets.push(SceneWidgetPlacement {
                    widget_id: wid, layout: WidgetLayout::default(),
                    enabled: true, zorder: scene.widgets.len() as i32 + 1,
                });
            }
            persist_scenes(reg);
            push_scenes(&ui_weak, reg);
        });
    }
});

ui.global::<Bridge>().on_apply_widget_layout({
    move |scene_id, widget_id, x, y, w, h, opacity| {
        let _ = migration_runtime::runtime::handle_command(Command::UpdateWidgetLayout {
            scene_id: scene_id.to_string(),
            widget_id: widget_id.to_string(),
            layout: WidgetLayout {
                x: x as f64, y: y as f64, width: w as f64, height: h as f64,
                rotation: 0.0, opacity: opacity as f64,
            },
        });
    }
});
```

Remaining callbacks — `rename-scene`, `remove-scene`, `reorder-scenes`,
`set-scene-quick-group`, `remove-widget`, `add-widget-to-scene`,
`remove-widget-from-scene`, `set-placement-enabled`, `pick-widget-image` —
follow the identical shape: mutate registry → (optional `handle_command`) →
`persist_scenes` → `push_scenes`. `pick-widget-image` reuses the existing JNI
file-picker upcall (same as the cam-rtmp image path) and writes
`draft-widget-image-path`.

→ Next: [STEP-9D-verify-done.md](STEP-9D-verify-done.md)
