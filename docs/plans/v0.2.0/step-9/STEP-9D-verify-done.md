# STEP-9D — Cargo deps, verify & v0.2.0 summary

## Cargo deps (from STEP-4)

```toml
# crates/migration-runtime/Cargo.toml
image    = { version = "0.25", default-features = false, features = ["png", "jpeg"] }
ab_glyph = "0.2"
# uuid already present in src/ and the runtime.
```

## Verify

```bash
cargo test -p migration-runtime
cargo build --target aarch64-linux-android
```

End-to-end:
1. Settings → Scenes → create "Main" + "BRB".
2. Edit "Main" → Add Widget → Text "{time}" → appears in scene.
3. Go live (camera→mixer→destination); open layout editor; drag the clock — it
   moves on the stream.
4. Bottom scene bar shows Main/BRB; tap BRB → overlays switch.
5. Restart app → scenes/widgets/current scene restored from `backend.json`.

## Done — v0.2.0 plan complete

| Step | Layer |
|---|---|
| 1 RIST destination | Rust + build |
| 2 Scene/Widget data model | Rust |
| 3 Scene→mixer translation (reuse compositor) | Rust |
| 4 Widget renderers | Rust |
| 5 Bridge + Panels | Slint |
| 6 Scene pages | Slint |
| 7 Widget pages | Slint |
| 8 Stream buttons + nav | Slint |
| 9 Persistence + wiring | Rust |

**Performance posture:** reuses the existing `compositor` mixer (no second
compositor), appsrc-per-widget (no `pango`/`gdkpixbuf` plugin), the v0.1.0
hardware encoder, and only adds the `rist` plugin. Scene switches reconfigure a
running mixer (no pipeline rebuild).

Next milestone: **v0.3.0** — browser source (WebView texture bridge), SRTLA
bonding, video effects, scene transitions, rotation (GL).
