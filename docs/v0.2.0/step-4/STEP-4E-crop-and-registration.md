# STEP-4E — Crop, registration & plugins

## Crop (special — `videocrop` on the camera source)

Crop is **not** a slot. STEP-3's `apply_crops` sets `videocrop` props on the
camera source chain (insert `videocrop` after the camera `appsrc`, before the
mixer link):

```rust
fn apply_crops(&mut self, scene: &Scene) {
    let crop = scene.widgets.iter().filter(|p| p.enabled).find_map(|p| {
        match self.widgets.get(&p.widget_id).map(|w| &w.widget_type) {
            Some(WidgetType::Crop { top, bottom, left, right }) => Some((*top,*bottom,*left,*right)),
            _ => None,
        }
    });
    let (t, b, l, r) = crop.unwrap_or((0.0, 0.0, 0.0, 0.0)); // reset when none
    if let Some(vc) = self.camera_videocrop() {
        let (w, h) = self.output_size();
        vc.set_property("top",    (t * h as f64 / 100.0) as i32);
        vc.set_property("bottom", (b * h as f64 / 100.0) as i32);
        vc.set_property("left",   (l * w as f64 / 100.0) as i32);
        vc.set_property("right",  (r * w as f64 / 100.0) as i32);
    }
}
```

> One crop per scene (it reframes the single camera); take the first enabled one.

## Registration (`nodes/mod.rs` + `NodeRecord`)

```rust
// nodes/mod.rs
pub mod widget_source;
pub use widget_source::WidgetSourceNode;
```

```rust
// node_manager.rs NodeRecord enum
enum NodeRecord {
    // ... existing ...
    WidgetSource(WidgetSourceNode),
}
// can_output_video() = true, can_output_audio() = false for it.
```

## Plugin reality check

| Renderer | Plugin add? |
|---|---|
| Crop (`videocrop`) | none — already in Android.mk |
| Image/Text/Clock (`appsrc`) | none — `app` present; only Cargo crates (`image`, `ab_glyph`) |

**v0.2.0 still needs only `rist` added (STEP-1).**

### Alternative: pango overlays

Add `pango` and use `textoverlay`/`clockoverlay` — but those draw inline on the
main buffer (no per-widget scale/opacity/zorder), so the appsrc route is the
recommended primary path.

## Done — STEP-4 complete

→ Next: [../step-5/INDEX.md](../step-5/INDEX.md)
