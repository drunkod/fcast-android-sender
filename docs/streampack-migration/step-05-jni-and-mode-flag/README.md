# Step 05 — JNI upcall (`startStreamPackCamera`) + pipeline-mode flag

**Master plan:** §5.4, §8 · **Phase:** 1 · **Depends on:** step-04 · **Lang:** Kotlin + Rust

## Goal

Carry the SRT URL (and camera config) from Rust to Kotlin **without** widening the
legacy positional JNI signature, and make Rust the source of truth for which pipeline
mode is active. The legacy `startCameraCapture(IIIIZZFI)V` path is left untouched.

## Files touched

- `app/.../MainActivity.kt` — new `startStreamPackCamera(String)` method
- `src/jni_bridge/camera.rs` — new `upcall_start_streampack_camera`
- `src/jni_bridge/main_activity.rs` + `src/lib.rs` — `nativeUseStreamPackCameraPath` export
- `src/config/mod.rs` — `AndroidCameraPipeline` enum
- `src/backend/persistence.rs` — persist the mode

## Code

### `MainActivity.kt` — new native-callable method

```kotlin
// Called from Rust via JNI. JSON: {cameraIdx,width,height,maxFps,mirror,
// stabilization,zoom,orientationMode,srtUrl}
@Suppress("unused")
private fun startStreamPackCamera(configJson: String) {
    val j = org.json.JSONObject(configJson)
    val mode = when (j.optString("orientationMode", "LANDSCAPE")) {
        "PORTRAIT" -> OrientationMode.PORTRAIT
        "AUTO"     -> OrientationMode.AUTO
        else       -> OrientationMode.LANDSCAPE
    }
    val cfg = CameraCaptureConfig(
        cameraIdx = j.optInt("cameraIdx", 1),
        width = j.optInt("width", 1280),
        height = j.optInt("height", 720),
        maxFps = j.optInt("maxFps", 30),
        mirror = j.optBoolean("mirror", false),
        stabilization = j.optBoolean("stabilization", true),
        zoom = j.optDouble("zoom", 1.0).toFloat(),
        orientationMode = mode,
    )
    runOnUiThread {
        applyOrientationLock(mode)
        if (nativeUseStreamPackCameraPath()) {
            destroyCameraPreview()    // step-04 edit 4: no legacy SurfaceView in StreamPack mode
        }
        (cameraCoordinator as? StreamPackCameraCaptureCoordinator)?.setSrtUrl(j.optString("srtUrl"))
        cameraCoordinator.startCapture(cfg, cameraPreviewSurface?.takeIf { it.isValid })
    }
}
```

### `src/jni_bridge/camera.rs` — new upcall (mirrors `upcall_start_camera_capture`)

```rust
#[cfg(target_os = "android")]
pub fn upcall_start_streampack_camera(config_json: &str) -> Result<(), String> {
    let ctx = crate::android_context().map_err(|e| e.to_string())?;
    let mut env = ctx.vm.attach_current_thread().map_err(|e| e.to_string())?;
    let j = env.new_string(config_json).map_err(|e| e.to_string())?;
    env.call_method(
        &ctx.activity,
        "startStreamPackCamera",
        "(Ljava/lang/String;)V",
        &[jni::objects::JValue::Object(&j.into())],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(not(target_os = "android"))]
pub fn upcall_start_streampack_camera(_config_json: &str) -> Result<(), String> { Ok(()) }
```

Stop reuses the existing `upcall_stop_camera_capture()` → `stopCameraCapture()`.

> **`stopCameraCapture()` must stay pipeline-mode-neutral.** After step-04 widened the
> field, it calls `cameraCoordinator.stopCapture()` through the interface only — no
> `RealCameraCaptureCoordinator`-specific code. Both Phase 1 and Phase 2 share one stop
> path; do **not** add a separate StreamPack stop upcall.

### `src/config/mod.rs` — pipeline-mode flag (Rust = source of truth)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AndroidCameraPipeline {
    LegacyRawI420Gstreamer,        // current default
    StreamPackDirectSrt,           // Phase 1
    StreamPackEncodedToGstreamer,  // Phase 2 target
}

impl Default for AndroidCameraPipeline {
    fn default() -> Self { Self::LegacyRawI420Gstreamer }
}
```

### `nativeUseStreamPackCameraPath` (Rust side)

Add the JNI symbol body (in `src/jni_bridge/main_activity.rs`) and re-export it with the
`Java_org_fcast_android_sender_MainActivity_*` name in `src/lib.rs`, next to the existing
native symbols. It returns:

```rust
// pseudocode body — reads the persisted mode (config/mod.rs + persistence.rs)
let use_streampack = !matches!(
    current_android_camera_pipeline(),
    crate::config::AndroidCameraPipeline::LegacyRawI420Gstreamer
);
jni::sys::jboolean::from(use_streampack)
```

- Persist the mode with the rest of the config (`src/backend/persistence.rs`).
- Phase 2 vs Phase 1 within StreamPack is chosen Rust-side (which source/destination
  to build — steps 09/10), keyed on the same enum.

## How to verify

```
✅ Rust builds for both target_os = android and host (cfg-gated stubs present).
✅ JNI signature "(Ljava/lang/String;)V" matches the Kotlin method exactly.
✅ With mode = StreamPackDirectSrt persisted, nativeUseStreamPackCameraPath() == true.
✅ With default mode, == false → legacy path, unchanged.
✅ Start from Rust → JSON arrives → coordinator.setSrtUrl + startCapture fire.
```

## Risks

- Keep the JSON keys in sync with `CameraCaptureConfig` field names and the Slint→Rust
  config (step-06). A typo in a key silently falls back to the `opt*` default.
- The legacy positional `startCameraCapture(IIIIZZFI)V` and `createcamerasource` graph
  command stay for `LegacyRawI420Gstreamer` — do not delete them.
