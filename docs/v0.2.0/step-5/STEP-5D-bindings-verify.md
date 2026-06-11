# STEP-5D — Generated Rust bindings & verify

| Slint | Rust |
|---|---|
| `Panel.scene-list` | `Panel::SceneList` |
| `SceneItem` | struct `SceneItem { id, name, enabled, active, widget_count, quick_switch_group }` |
| `scenes` property | `set_scenes(ModelRc<SceneItem>)` |
| `set-scene` callback | `on_set_scene(impl Fn(SharedString))` |
| `create-widget` | `on_create_widget(impl Fn())` |
| `apply-widget-layout` | `on_apply_widget_layout(impl Fn(SharedString,SharedString,f32,f32,f32,f32,f32))` |

> Slint `float` → Rust `f32`. The STEP-9 handlers convert to the `f64` protocol
> `WidgetLayout`.

```bash
slint-lsp ui/main.slint 2>&1 | grep -c error   # → 0
```

## Done — STEP-5 complete

→ Next: [../step-6/INDEX.md](../step-6/INDEX.md)
