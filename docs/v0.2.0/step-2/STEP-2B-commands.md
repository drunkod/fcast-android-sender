# STEP-2B — Command variants

> Add to the `Command` enum (`#[serde(rename_all = "lowercase")]`), so they
> serialize as `{"setscene":{"scene_id":"…"}}` etc.

```rust
    CreateScene { scene: Scene },
    UpdateScene { scene: Scene },
    RemoveScene { scene_id: String },
    SetScene { scene_id: String },

    CreateWidget { widget: Widget },
    UpdateWidget { widget: Widget },
    RemoveWidget { widget_id: String },
    UpdateWidgetLayout {
        scene_id: String,
        widget_id: String,
        layout: WidgetLayout,
    },
```

Only `SetScene` / `UpdateWidgetLayout` touch the live graph (STEP-3); the CRUD
variants mutate the registry + persist (STEP-9).

→ Next: [STEP-2C-tests.md](STEP-2C-tests.md)
