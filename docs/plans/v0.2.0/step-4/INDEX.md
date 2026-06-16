# STEP-4 — Widget renderers (sub-steps)

> Per-widget GStreamer source feeding each mixer slot (+ crop on the camera).
> **Refs:** PHASE-41 §41-C, mapping doc §2.

| Widget | Mechanism | Plugin |
|---|---|---|
| Crop | `videocrop` on camera | ✅ present |
| Image | `image` crate → RGBA → `appsrc` | ✅ `app` (+ `image` crate) |
| Text/Clock | rasterize → RGBA → `appsrc` | ✅ `app` (+ `ab_glyph`) |

Uniform `appsrc`-per-widget gives full `WidgetLayout` geometry and adds **no
GStreamer plugin** — only Cargo crates. v0.2.0 still needs only `rist` (STEP-1).

| # | File | Scope |
|---|------|-------|
| 4A | [STEP-4A-widget-source-node.md](STEP-4A-widget-source-node.md) | `WidgetSourceNode` (appsrc, RGBA push) |
| 4B | [STEP-4B-image-renderer.md](STEP-4B-image-renderer.md) | `image` crate → RGBA |
| 4C | [STEP-4C-text-renderer.md](STEP-4C-text-renderer.md) | `ab_glyph` rasterizer |
| 4D | [STEP-4D-clock-renderer.md](STEP-4D-clock-renderer.md) | timer-driven text |
| 4E | [STEP-4E-crop-and-registration.md](STEP-4E-crop-and-registration.md) | `videocrop` + `mod.rs`/`NodeRecord` + plugins |

→ Next: [../step-5/INDEX.md](../step-5/INDEX.md)
