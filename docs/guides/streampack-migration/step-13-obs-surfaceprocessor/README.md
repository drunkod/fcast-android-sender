# Step 13 — OBS-style SurfaceProcessor compositor (long-term)

**Master plan:** §12 · **Phase:** 5 · **Depends on:** step-12 · **Lang:** Kotlin + Rust

## Goal

Grow the `SurfaceProcessor` from a preview fanout (step-12) into a full GL compositor so
Rust/Slint can drive OBS-like scenes: multiple sources, overlays, transitions — all
rendered directly into the `MediaCodec` encoder surface (no raw frame round-trip).

## Target data flow

```
Slint/Rust scene graph → JNI scene updates → custom SurfaceProcessor / GL compositor
  (camera + screen + image/text overlays, crop/scale/rotate/mirror)
  → MediaCodec encoder surface → encoded frames → Rust/GStreamer egress
```

This replaces "convert camera frame to YUV and send to Rust" with "send scene commands
to Android and let the GPU render directly into the encoder surface".

## Ownership split

```
Rust/Slint:   scenes, source positions/visibility, transitions, stream/record state,
              remote control state.
Android:      Camera2, MediaProjection, SurfaceTexture, EGL, MediaCodec, StreamPack
              encoder lifecycle.
GStreamer/Rust: SRT/mux, recording variants, diagnostic pipelines, device capability
              reporting, legacy fallback path.
```

## Suggested build-up

1. Single camera source → compositor → encoder surface (parity with step-12 preview).
2. Add a screen (MediaProjection) source as a second texture.
3. Add image + text overlays (scene widgets from the Slint scene graph).
4. Add per-source crop/scale/rotate/mirror (maps to the existing Slint widget model:
   `camera-*`, crop widgets, scene/widget bridge models).
5. Add transitions (cut → crossfade) between scenes.

JNI scene-update channel mirrors the existing `native_graph_command` JSON convention so
Rust stays the scene source of truth.

## How to verify (per increment)

```
✅ Each added source/overlay renders into the encoded output without a separate Android
   UI tree.
✅ Scene updates from Rust/Slint are reflected within one frame or a defined transition.
✅ Compositor runs on the GL/encoder thread without stalling MediaCodec.
✅ Falls back cleanly (or is disabled) on devices where StreamPack/GL compositing fails.
```

## Risks / open questions

- This is the largest step; treat each increment above as its own PR.
- GL context sharing between the camera SurfaceTexture, overlays, and the MediaCodec
  input surface needs careful EGL management (reuse lessons from `CaptureEngine`'s EGL
  setup in the legacy path).
- Performance budget: compositing + encoding must stay within thermal headroom — measure
  against the step-07 baseline at every increment.
- Keep GStreamer/SRT strictly post-encode; do not move compositing into GStreamer.
