# Step 06 — Slint mode selector + `app.rs` wiring

**Master plan:** §9 · **Phase:** 1 · **Depends on:** step-05 · **Lang:** Slint + Rust

## Goal

Expose a single "Capture pipeline" selector in the existing camera page and persist the
chosen `AndroidCameraPipeline` via Rust. Reuse the existing start/stop callbacks; the
start handler branches on the mode to call the legacy path or the new StreamPack upcall.

## Existing Slint surface (already in the repo)

`ui/bridge.slint` already exposes:
`camera-idx`, `camera-orientation-mode-idx`, `camera-mirror-front`,
`camera-stabilization`, `camera-zoom-level`, the camera-RTMP page
(`cam-rtmp-url`, `start-camera-rtmp-stream()`, `stop-camera-rtmp-stream()`,
`start-camera-rtmp-preview()`) and `srt-destination` + `start-srt-destination()`.

## Code

### `ui/bridge.slint` — inside `global Bridge`

```slint
// 0=Legacy 1=StreamPackDirectSrt 2=StreamPackEncodedToGStreamer
in-out property <int> camera-pipeline-mode-idx: 0;
callback set-camera-pipeline-mode(int);
```

### `ui/pages/camera_page.slint` — a settings row (same shape as the Orientation row)

```slint
SettingsRow {
    title: @tr("Capture pipeline");
    value: [@tr("Legacy (GStreamer)"),
            @tr("StreamPack → SRT"),
            @tr("StreamPack → GStreamer")][Math.clamp(Bridge.camera-pipeline-mode-idx, 0, 2)];
    clicked => {
        Bridge.camera-pipeline-mode-idx = Math.mod(Bridge.camera-pipeline-mode-idx + 1, 3);
        Bridge.set-camera-pipeline-mode(Bridge.camera-pipeline-mode-idx);
    }
}
```

### `src/app.rs` — wire the callback + branch the start handler

```rust
// On set-camera-pipeline-mode(idx): map idx → AndroidCameraPipeline and persist it
// (config/mod.rs + persistence.rs from step-05). Takes effect next launch (Option A).
let mode = match idx {
    1 => AndroidCameraPipeline::StreamPackDirectSrt,
    2 => AndroidCameraPipeline::StreamPackEncodedToGstreamer,
    _ => AndroidCameraPipeline::LegacyRawI420Gstreamer,
};
persist_android_camera_pipeline(mode);
```

```rust
// In the existing start-camera-rtmp-stream / start-srt-destination handler, branch:
match current_android_camera_pipeline() {
    AndroidCameraPipeline::LegacyRawI420Gstreamer => {
        // unchanged: upcall_start_camera_capture(...) + createcamerasource graph command
    }
    AndroidCameraPipeline::StreamPackDirectSrt
    | AndroidCameraPipeline::StreamPackEncodedToGstreamer => {
        let config_json = build_streampack_config_json(/* cam idx, w, h, fps, mirror,
            stabilization, zoom, orientation, srt_url */);
        camera::upcall_start_streampack_camera(&config_json)?;
    }
}
```

> `build_streampack_config_json` must emit exactly the keys `startStreamPackCamera`
> reads (step-05): `cameraIdx,width,height,maxFps,mirror,stabilization,zoom,
> orientationMode,srtUrl`.

## How to verify

```
✅ Selector row renders and cycles Legacy → SRT → GStreamer → Legacy.
✅ set-camera-pipeline-mode persists; value survives app restart.
✅ On next launch, nativeUseStreamPackCameraPath() reflects the selected mode (step-05).
✅ Legacy mode start path unchanged (regression).
✅ StreamPackDirectSrt start emits JSON with all required keys and calls the upcall.
```

## Notes

- The selector **previews the next-launch value** in Phase 1 (Option A, step-04). Make
  the label or a sub-caption say "applies after restart" to avoid confusion.
- Keep `set-camera-pipeline-mode` idempotent; it only persists, it does not hot-swap.
