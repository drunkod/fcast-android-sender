# STEP-4B — Image renderer

```toml
# crates/migration-runtime/Cargo.toml
[dependencies]
image = { version = "0.25", default-features = false, features = ["png", "jpeg"] }
```

```rust
pub fn render_image_widget(asset_path: &str, w: u32, h: u32, scale_mode: &str) -> Result<Vec<u8>, String> {
    let img = image::open(asset_path).map_err(|e| format!("open image: {e}"))?;
    let resized = match scale_mode {
        "stretch" => img.resize_exact(w, h, image::imageops::FilterType::Triangle),
        "fill"    => img.resize_to_fill(w, h, image::imageops::FilterType::Triangle),
        _ /*fit*/ => img.resize(w, h, image::imageops::FilterType::Triangle),
    };
    let mut canvas = image::RgbaImage::from_pixel(w, h, image::Rgba([0, 0, 0, 0]));
    let (rw, rh) = (resized.width(), resized.height());
    image::imageops::overlay(
        &mut canvas, &resized.to_rgba8(),
        ((w - rw) / 2) as i64, ((h - rh) / 2) as i64,
    );
    Ok(canvas.into_raw())
}
```

Decode once on widget add; push a single frozen frame — the compositor holds the
last buffer, so static images need no re-push.

→ Next: [STEP-4C-text-renderer.md](STEP-4C-text-renderer.md)
