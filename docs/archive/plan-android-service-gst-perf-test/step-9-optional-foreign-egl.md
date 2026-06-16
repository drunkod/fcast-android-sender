# Step 9 — (Optional) `src/codec_egl.rs`: foreign-EGL guard

← [Step 8](step-8-android-main-rewire.md) · [Index](README.md) · Next → [Step 10](step-10-verification.md)

**Optional and uncertain — skip unless the separate process alone proves
insufficient.** The separate `:codec_bench` process is the real fix. This is
extra hygiene that asks GStreamer not to call `eglTerminate()` on a display it
doesn't own — but it requires **GStreamer 1.26+** and new deps.

### 9a — Preconditions (verify first)

1. The Android GStreamer binaries under `.android/gstreamer` must be **1.26+**
   (`set_foreign()` is behind the `v1_26` feature; older builds lack it).
2. New deps + a cargo feature so the rest of the plan compiles without this step:

`Cargo.toml`:
```toml
[dependencies]
gstreamer-gl     = "0.25"
gstreamer-gl-egl = { version = "0.25", features = ["v1_26"] }

[features]
foreign-egl = ["dep:gstreamer-gl", "dep:gstreamer-gl-egl"]
```
(Step 2 and Step 4a already gate the module/usage behind `#[cfg(feature = "foreign-egl")]`.)

### 9b — `src/codec_egl.rs`

```rust
//! Optional GStreamer 1.26+ foreign-EGL guard. Only meaningful inside :codec_bench.

#[cfg(target_os = "android")]
pub struct ForeignEglGuard {
    _display: Option<gstreamer_gl_egl::GLDisplayEGL>,
}
#[cfg(not(target_os = "android"))]
pub struct ForeignEglGuard;

pub fn try_install_foreign_egl_for_current_process() -> ForeignEglGuard {
    #[cfg(target_os = "android")]
    {
        match try_create_foreign_egl_display() {
            Ok(display) => ForeignEglGuard { _display: Some(display) },
            Err(e) => {
                tracing::warn!("foreign EGL guard disabled: {e}");
                ForeignEglGuard { _display: None }
            }
        }
    }
    #[cfg(not(target_os = "android"))]
    { ForeignEglGuard }
}

#[cfg(target_os = "android")]
fn try_create_foreign_egl_display() -> Result<gstreamer_gl_egl::GLDisplayEGL, String> {
    use gstreamer_gl_egl::prelude::*;
    let display = gstreamer_gl_egl::GLDisplayEGL::new()
        .map_err(|e| format!("GLDisplayEGL::new failed: {e}"))?;
    // 1.26+: mark foreign so finalization does NOT call eglTerminate().
    display.set_foreign(true);
    Ok(display)
}
```

### 9c — Reality check

The exact safe-Rust API for `gst_context_set_gl_display()` varies by
`gstreamer-rs` version; if injecting the display into a specific pipeline doesn't
compile, fall back to a tiny C helper or `gstreamer-gl-egl-sys`. Given the process
is killed right after decode anyway, the guard mostly avoids the scary
`eglTerminate()` log line — it is **not** load-bearing for the crash fix.

> Recommendation: ship steps 1–8 + 10 first. Only add Step 9 if you later want
> decode diagnostics **without** killing the process, or to silence the warning.

---

← [Step 8](step-8-android-main-rewire.md) · [Index](README.md) · Next → [Step 10](step-10-verification.md)
