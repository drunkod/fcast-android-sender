# STEP-4C — Text renderer (`ab_glyph`)

```toml
ab_glyph = "0.2"
```

```rust
use ab_glyph::{Font, FontRef, Glyph, point, ScaleFont};

static FONT_BYTES: &[u8] = include_bytes!("../../assets/Inter.ttf"); // bundle a font

pub fn render_text_widget(text: &str, w: u32, h: u32, px: f32, rgba_color: [u8; 4]) -> Result<Vec<u8>, String> {
    let font = FontRef::try_from_slice(FONT_BYTES).map_err(|_| "load font")?;
    let scaled = font.as_scaled(px);
    let mut canvas = vec![0u8; (w * h * 4) as usize]; // transparent RGBA
    let mut caret = point(4.0, px);
    for ch in text.chars() {
        let g: Glyph = scaled.scaled_glyph(ch);
        if let Some(outline) = font.outline_glyph(g.clone()) {
            let bounds = outline.px_bounds();
            outline.draw(|gx, gy, cov| {
                let x = bounds.min.x as i32 + gx as i32;
                let y = bounds.min.y as i32 + gy as i32 + caret.y as i32 - px as i32;
                if x >= 0 && y >= 0 && (x as u32) < w && (y as u32) < h {
                    let i = ((y as u32 * w + x as u32) * 4) as usize;
                    let a = (cov * rgba_color[3] as f32) as u8;
                    canvas[i] = rgba_color[0]; canvas[i+1] = rgba_color[1];
                    canvas[i+2] = rgba_color[2]; canvas[i+3] = canvas[i+3].max(a);
                }
            });
        }
        caret.x += scaled.h_advance(g.id);
    }
    Ok(canvas)
}
```

> Variable substitution (`{time}`, `{date}`, custom) expands the `format` string
> **before** rasterizing (mirrors Moblin's `TextEffectFormatter`).

→ Next: [STEP-4D-clock-renderer.md](STEP-4D-clock-renderer.md)
