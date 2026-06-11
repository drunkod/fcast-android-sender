# STEP-4A — `WidgetSourceNode`

> An `appsrc` emitting `video/x-raw, format=RGBA` at the scene framerate.
> **Create:** `crates/migration-runtime/src/nodes/widget_source.rs` (mirrors
> `VideoGeneratorNode`).

```rust
use gst::prelude::*;
use gst_app::AppSrc;

pub struct WidgetSourceNode {
    pub id: String,
    pub widget_id: String,
    pub appsrc: Option<AppSrc>,
    pub width: u32,
    pub height: u32,
}

impl WidgetSourceNode {
    pub fn make_appsrc(id: &str, width: u32, height: u32) -> Result<AppSrc, String> {
        let el = gst::ElementFactory::make("appsrc")
            .name(format!("widget-appsrc-{id}"))
            .build()
            .map_err(|e| format!("appsrc: {}", &*e.message))?;
        let appsrc: AppSrc = el.downcast().map_err(|_| "downcast appsrc".to_string())?;
        appsrc.set_property("is-live", true);
        appsrc.set_property("do-timestamp", true);
        appsrc.set_property_from_str("format", "time");
        appsrc.set_caps(Some(
            &gst::Caps::builder("video/x-raw")
                .field("format", "RGBA")
                .field("width", width as i32)
                .field("height", height as i32)
                .field("framerate", gst::Fraction::new(30, 1))
                .build(),
        ));
        Ok(appsrc)
    }

    /// Push one fully-rendered RGBA frame (w*h*4 bytes) into the slot.
    pub fn push_rgba(&self, rgba: &[u8]) -> Result<(), String> {
        let Some(src) = self.appsrc.as_ref() else { return Ok(()); };
        let mut buffer = gst::Buffer::with_size(rgba.len()).map_err(|_| "alloc buffer")?;
        {
            let buf = buffer.get_mut().unwrap();
            let mut map = buf.map_writable().map_err(|_| "map buffer")?;
            map.copy_from_slice(rgba);
        }
        src.push_buffer(buffer).map(|_| ()).map_err(|e| format!("push: {e:?}"))
    }
}
```

Mirrors the v0.1.0 camera `appsrc` push; the mixer connects this source like any
other slot.

→ Next: [STEP-4B-image-renderer.md](STEP-4B-image-renderer.md)
