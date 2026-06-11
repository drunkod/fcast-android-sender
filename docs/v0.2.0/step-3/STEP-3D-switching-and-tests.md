# STEP-3D — Switching cost & tests

## Scene switching cost (matches Moblin)

Same `quick_switch_group` (same camera) → only slot add/remove/reconfigure on the
**running** mixer; no camera reattach, no pipeline rebuild. Different/none group →
reattach the camera source (heavier). Mixer + encoder + destination keep running
throughout. Gate the reattach on `quick_switch_group` like PHASE-40 §40-B.

## Tests

```rust
#[test]
fn apply_scene_diffs_widget_links() {
    // register scene A {w1,w2} and B {w2,w3}; set A then B.
    // assert A→B disconnects w1, keeps w2, connects w3.
}

#[test]
fn slot_config_maps_layout_fields() {
    let cfg = slot_config_from_layout(
        &WidgetLayout { x: 10.0, y: 20.0, width: 30.0, height: 40.0, rotation: 90.0, opacity: 0.5 },
        3,
    );
    assert_eq!(cfg["video::x"], serde_json::json!(10.0));
    assert_eq!(cfg["video::alpha"], serde_json::json!(0.5));
    assert!(!cfg.contains_key("video::rotation")); // not honored in v0.2.0
}
```

## Done — STEP-3 complete

→ Next: [../step-4/INDEX.md](../step-4/INDEX.md)
