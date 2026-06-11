# STEP-9B — Model conversion (Rust → Bridge)

```rust
fn scene_items(reg: &SceneRegistry) -> Vec<SceneItem> {
    reg.scenes.values().map(|s| SceneItem {
        id: s.id.clone().into(),
        name: s.name.clone().into(),
        enabled: s.enabled,
        active: reg.current_scene_id.as_deref() == Some(&s.id),
        widget_count: s.widgets.len() as i32,
        quick_switch_group: s.quick_switch_group.unwrap_or(0) as i32,
    }).collect()
}

fn widget_items(reg: &SceneRegistry) -> Vec<WidgetItem> {
    reg.widgets.values().map(|w| WidgetItem {
        id: w.id.clone().into(),
        name: w.name.clone().into(),
        widget_type: widget_type_str(&w.widget_type).into(), // "text"|"image"|"crop"|"clock"
        enabled: w.enabled,
    }).collect()
}

fn push_scenes(ui: &slint::Weak<MainWindow>, reg: &SceneRegistry) {
    let scenes = std::rc::Rc::new(slint::VecModel::from(scene_items(reg)));
    let widgets = std::rc::Rc::new(slint::VecModel::from(widget_items(reg)));
    let cur = reg.current_scene_id.clone().unwrap_or_default();
    let _ = ui.upgrade_in_event_loop(move |u| {
        let b = u.global::<Bridge>();
        b.set_scenes(scenes.into());
        b.set_widgets(widgets.into());
        b.set_current_scene_id(cur.into());
    });
}
```

> `SceneRegistry` (STEP-3) is the runtime source of truth; the Bridge models are
> projections pushed after every mutation. Hold the registry where the other
> runtime state lives, or keep a UI-thread copy synced from config — match the
> existing `android_main.rs` pattern.

→ Next: [STEP-9C-handler-wiring.md](STEP-9C-handler-wiring.md)
