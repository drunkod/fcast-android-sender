# STEP-2C — Serde tests

```rust
#[test]
fn widget_text_flat_roundtrip() {
    let w = Widget {
        id: "w1".into(), name: "Title".into(), enabled: true,
        widget_type: WidgetType::Text { format: "Hello".into(), font_size: Some(32), color: None },
    };
    let json = serde_json::to_string(&w).unwrap();
    assert!(json.contains(r#""type":"text""#));
    assert_eq!(serde_json::from_str::<Widget>(&json).unwrap(), w);
}

#[test]
fn scene_with_placement_roundtrip() {
    let s = Scene {
        id: "s1".into(), name: "Main".into(), enabled: true,
        quick_switch_group: Some(1),
        widgets: vec![SceneWidgetPlacement {
            widget_id: "w1".into(), enabled: true, zorder: 1,
            layout: WidgetLayout { x: 10.0, y: 10.0, width: 30.0, height: 10.0, rotation: 0.0, opacity: 0.9 },
        }],
    };
    let json = serde_json::to_string(&s).unwrap();
    assert_eq!(serde_json::from_str::<Scene>(&json).unwrap(), s);
}

#[test]
fn widget_layout_defaults() {
    let l: WidgetLayout = serde_json::from_str(r#"{"x":0,"y":0,"width":50,"height":50}"#).unwrap();
    assert_eq!(l.opacity, 1.0);
    assert_eq!(l.rotation, 0.0);
}

#[test]
fn set_scene_command_wire_shape() {
    let c = Command::SetScene { scene_id: "s1".into() };
    let v = serde_json::to_value(&c).unwrap();
    assert!(v.get("setscene").is_some());
}

#[test]
fn crop_widget_roundtrip() {
    let w = Widget {
        id: "c1".into(), name: "Reframe".into(), enabled: true,
        widget_type: WidgetType::Crop { top: 5.0, bottom: 5.0, left: 0.0, right: 0.0 },
    };
    assert_eq!(serde_json::from_str::<Widget>(&serde_json::to_string(&w).unwrap()).unwrap(), w);
}
```

```bash
cargo test -p migration-runtime -- scene widget crop set_scene
```

## Done — STEP-2 complete

→ Next: [../step-3/INDEX.md](../step-3/INDEX.md)
