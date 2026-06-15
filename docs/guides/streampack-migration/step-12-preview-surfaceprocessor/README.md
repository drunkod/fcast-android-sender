# Step 12 — Preview via SurfaceProcessor

**Master plan:** §11 · **Phase:** 4 · **Depends on:** step-10 · **Lang:** Kotlin

## Goal

Restore an embedded camera preview in StreamPack mode without pulling `streampack-ui`'s
`PreviewView` (which assumes a normal Android view tree — Slint owns ours). Use
StreamPack's customizable `SurfaceProcessor` to fan the camera input surface out to both
the encoder surface and a NativeActivity/Slint preview surface.

## Progression

```
Phase 1 (steps 01–07): no embedded preview — validate egress only.
Phase 2 (this step, interim): keep the legacy SurfaceView preview when NOT streaming.
Phase 3 (this step, target):  custom SurfaceProcessor fanout —
        camera surface → encoder surface
                       → NativeActivity/Slint preview surface
```

## Files touched

- `app/.../stream/StreamPackSenderBridge.kt` — expose a preview surface hook
- (later) a small `SurfaceProcessor` implementation

## Approach

- Add a `setPreviewSurface(surface: Surface?)` to `StreamPackSenderBridge` that wires a
  StreamPack `SurfaceProcessor` fanout output to the provided surface.
- Reuse the existing `MainActivity` `cameraPreviewSurface` (the `SurfaceView` created by
  `ensureCameraPreviewView()`), but only attach it via the processor — **not** via the
  legacy Camera2 preview session (which is disabled in StreamPack mode, step-04 edit 4).
- Keep ownership clear: the `SurfaceHolder` owns the surface; the bridge only references
  it while streaming and drops it on stop.

## ⚠ `isCapturing` audit (carried from master §15)

`StreamPackCameraCaptureCoordinator.isCapturing` returns `capturing` only (false during
`STARTING`). The legacy preview callers were safe because preview was disabled. **This
step re-introduces preview**, so before wiring it:

```
- Audit every caller that treats isCapturing == false as "safe to create preview / start
  again". If any new one appears, expose:
      val isActive get() = starting || capturing
  on the coordinator and gate preview creation on isActive, not isCapturing.
```

## How to verify

```
✅ StreamPack mode shows a live preview that matches the encoded output (orientation,
   mirror, crop).
✅ Preview survives start/stop without leaking the SurfaceProcessor or the surface.
✅ No second Android UI tree / no streampack-ui PreviewView in the build.
✅ Preview does not stall the encoder (fanout is non-blocking).
```

## Risks

- Orientation/mirror must match the encoded stream (same `setTargetRotation` basis as
  step-02). A mismatched preview is a strong hint the encoder rotation is wrong too.
- SurfaceProcessor lifecycle vs the bridge state machine (step-02): attach/detach the
  preview output under the same serialization to avoid attaching to a released streamer.
