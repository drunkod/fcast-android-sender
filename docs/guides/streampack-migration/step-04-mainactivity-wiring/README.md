# Step 04 — `AppGraph` factory + `MainActivity` wiring

**Master plan:** §5.3 · **Phase:** 1 · **Depends on:** step-03 · **Lang:** Kotlin

## Goal

Let `MainActivity` choose between the legacy and StreamPack coordinators **once in
`onCreate`** behind the Rust flag, route `onPermissionResult` to whichever is active,
and disable the legacy SurfaceView preview in StreamPack mode. Mode switching is
**restart-required** in Phase 1 (Option A).

## Files touched

- `app/src/main/java/org/fcast/android/sender/AppGraph.kt` — add a factory
- `app/src/main/java/org/fcast/android/sender/MainActivity.kt` — four surgical edits + flag shim

## Option A vs B (Phase 1 chooses A)

```text
Option A (Phase 1 — chosen):
  Pipeline mode changes are persisted but only take effect after app restart.
  Simpler and race-free; the selector shows the value that will apply next launch.

Option B (Phase 2+):
  On change: stopCapture() → destroyCameraPreview() → shutdown old coordinator →
  build the other coordinator → attach() → optionally restart preview/capture.
```

Without this rule a runtime selector would *appear* to switch modes while leaving the
old coordinator instance alive.

## Code

### `AppGraph.kt` — add next to `newCaptureCoordinator`

```kotlin
import org.fcast.android.sender.capture.CameraCaptureCoordinator
import org.fcast.android.sender.capture.StreamPackCameraCaptureCoordinator

// inside class AppGraph:
fun newStreamPackCameraCoordinator(
    callbacks: CameraCaptureCoordinator.Callbacks,
): CameraCaptureCoordinator =
    StreamPackCameraCaptureCoordinator(
        applicationContext = appContext,
        callbacks = callbacks,
    )
```

### `MainActivity.kt` — edit 1: widen the field type

```kotlin
// before:  private lateinit var cameraCoordinator: RealCameraCaptureCoordinator
private lateinit var cameraCoordinator: CameraCaptureCoordinator
```

### `MainActivity.kt` — edit 2: pick the impl in `onCreate`

```kotlin
// replaces the direct `RealCameraCaptureCoordinator(applicationContext, cameraCallbacks)`:
cameraCoordinator = if (nativeUseStreamPackCameraPath()) {
    (application as FcastApp).graph.newStreamPackCameraCoordinator(cameraCallbacks)
} else {
    RealCameraCaptureCoordinator(applicationContext, cameraCallbacks)
}
cameraCoordinator.attach()
```

### `MainActivity.kt` — edit 3: route `onPermissionResult` (interface has none)

```kotlin
// in onRequestPermissionsResult(...) and the onCreate proactive-grant branch:
when (val c = cameraCoordinator) {
    is RealCameraCaptureCoordinator       -> c.onPermissionResult(cameraGranted)
    is StreamPackCameraCaptureCoordinator -> c.onPermissionResult(cameraGranted)
}
```

### `MainActivity.kt` — edit 4: gate the legacy preview on the active mode

```kotlin
// wherever startDefaultCameraPreview() is currently called
// (onCreate proactive-grant branch AND onRequestPermissionsResult):
if (!nativeUseStreamPackCameraPath()) {
    startDefaultCameraPreview()
}
```

```kotlin
// in startStreamPackCamera(...) (added in step-05), before startCapture, ensure the
// legacy SurfaceView preview is gone so camera ownership is unambiguous:
if (nativeUseStreamPackCameraPath()) {
    destroyCameraPreview()
}
```

> Net effect: in StreamPack mode the legacy `cameraPreviewSurface`/SurfaceView path is
> never created. Embedded preview returns in step-12 via a `SurfaceProcessor` fanout.

### `MainActivity.kt` — flag shim (with the other `external` declarations)

```kotlin
private external fun nativeUseStreamPackCameraPath(): Boolean
```

> The Rust side of `nativeUseStreamPackCameraPath` + the pipeline-mode flag are
> implemented in **step-05**.

## How to verify

```
✅ Flag OFF (default): identical to today — legacy coordinator + default preview.
✅ Flag ON: StreamPack coordinator constructed; startDefaultCameraPreview() NOT called;
   no SurfaceView appears.
✅ Permission grant/deny routes to the active coordinator's onPermissionResult.
✅ stopCameraCapture() calls cameraCoordinator.stopCapture() via the interface only
   (no RealCameraCaptureCoordinator-specific code) — one shared stop path (step-05).
```

## Risks (carried from master §15)

- **Default preview conflict** — handled by edit 4; if any other call site invokes
  `startDefaultCameraPreview()`, gate it too.
- **Runtime mode switching** — out of scope (Option A). The Slint selector (step-06)
  only persists the value for next launch.
