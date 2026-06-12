# Step 11 — Retire the GL readback for streaming

**Master plan:** §10 · **Phase:** 3 · **Depends on:** step-10 stable · **Lang:** — (policy)

## Goal

Once the StreamPack encoded → GStreamer path (steps 08–10) is stable, stop using the GL
readback path for **streaming** — but **keep it** as fallback, regression baseline, and
the frame-dump diagnostic. Nothing is deleted.

## What stays

`CameraCaptureEngine` + `MainActivity.nativeProcessFrame` + `process_frame` remain as:

```
- Legacy fallback        (devices where StreamPack fails)
- Regression baseline    (compare against StreamPack output)
- Frame-dump debug tool  (FCAST_DUMP_DIR tooling in helpers.rs / camera_source.rs,
                          scripts/dump-frames.sh)
```

The raw `CameraSourceNode` builder (step-09 left it untouched) and the raw
`DestinationNode` encoder path (step-10 left it untouched) also stay for
`LegacyRawI420Gstreamer`.

## What changes

- Default/Recommended mode in the Slint selector (step-06) can move to
  `StreamPackEncodedToGstreamer` once it's proven on target devices — but keep
  `LegacyRawI420Gstreamer` selectable.
- Document that the raw path is **diagnostic/fallback only** for camera streaming so no
  one re-optimises it.

## How to verify

```
✅ Legacy mode still streams (fallback intact).
✅ Frame-dump tooling still works (create FCAST_DUMP_DIR/on marker, pull cam_*.i420).
✅ Default mode (if switched) is StreamPack; switching back to Legacy still works after
   restart (Option A).
```

## Risks

- Don't delete `nativeProcessFrame` or the GL engine — several diagnostics depend on it,
  and it's the device-failure fallback.
- Keep the green-line investigation tooling discoverable; it's the fastest way to debug
  any future encoder padding regressions.
