# Step 03 — `StreamPackCameraCaptureCoordinator`

**Master plan:** §5.1 · **Phase:** 1 · **Depends on:** step-02 · **Lang:** Kotlin

## Goal

Implement the **existing** `CameraCaptureCoordinator` interface backed by the
`StreamPackSenderBridge`, so nothing upstream (MainActivity contract, Rust control
surface) changes. Track `starting` vs `capturing` and stamp each start with a monotonic
`sessionId` so a late async callback can't resurrect a stopped capture.

## Files touched

- **New:** `app/src/main/java/org/fcast/android/sender/capture/StreamPackCameraCaptureCoordinator.kt`

## Interface being implemented (already in the repo, for reference)

```kotlin
// app/.../capture/CameraCaptureCoordinator.kt  (existing — do not edit)
interface CameraCaptureCoordinator {
    @MainThread fun attach()
    @MainThread fun startPreview(config: CameraCaptureConfig, previewSurface: Surface)
    @MainThread fun stopPreview()
    @MainThread fun startCapture(config: CameraCaptureConfig)
    @MainThread fun startCapture(config: CameraCaptureConfig, previewSurface: Surface?)
    @MainThread fun stopCapture()
    @MainThread fun shutdown()
    val isCapturing: Boolean

    interface Callbacks {
        @MainThread fun onCameraPermissionNeeded()
        @MainThread fun onCameraCaptureStarted(width: Int, height: Int, rotationDeg: Int)
        @MainThread fun onCameraCaptureStopped()
        @MainThread fun onCameraCaptureFailed(reason: String)
    }
}
```

## Full code

```kotlin
package org.fcast.android.sender.capture

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.os.Handler
import android.os.Looper
import android.util.Log
import android.view.Surface
import androidx.annotation.MainThread
import androidx.core.content.ContextCompat
import org.fcast.android.sender.stream.StreamPackSenderBridge

/**
 * StreamPack-backed implementation of [CameraCaptureCoordinator].
 *
 * Unlike [RealCameraCaptureCoordinator] (Camera2 → GL → I420 → nativeProcessFrame),
 * this drives a StreamPack [StreamPackSenderBridge] whose camera frames go
 * Camera2 → Surface → MediaCodec. In Phase 1 the encoded frames egress via
 * StreamPack's own SRT endpoint; GStreamer is not involved in the camera path.
 *
 * The Rust/Slint control contract (CameraCaptureCoordinator) is unchanged.
 */
class StreamPackCameraCaptureCoordinator(
    private val applicationContext: Context,
    private val callbacks: CameraCaptureCoordinator.Callbacks,
    private val bridge: StreamPackSenderBridge = StreamPackSenderBridge(applicationContext),
) : CameraCaptureCoordinator {

    private val mainHandler = Handler(Looper.getMainLooper())
    private var pendingConfig: CameraCaptureConfig? = null
    private var srtUrl: String = ""
    // Distinguish "scheduled the start coroutine" from "encoder/SRT actually up" so a
    // stopCapture() arriving mid-configuration cannot race the async start.
    private var starting = false
    private var capturing = false
    // Monotonic session token. Bumped on every start and every stop. Async bridge
    // callbacks captured `mySession` and drop themselves if the token has moved on, so a
    // late onStarted/onError can't revive a capture that was already stopped/restarted.
    private var sessionId = 0L

    @MainThread override fun attach() { /* nothing to subscribe to */ }

    // Preview is owned by the NativeActivity SurfaceView in Phase 1 — no-op here.
    @MainThread override fun startPreview(config: CameraCaptureConfig, previewSurface: Surface) {}
    @MainThread override fun stopPreview() {}

    @MainThread override fun startCapture(config: CameraCaptureConfig) = startCapture(config, null)

    @MainThread
    override fun startCapture(config: CameraCaptureConfig, previewSurface: Surface?) {
        if (capturing) { Log.w(TAG, "startCapture while already capturing"); return }

        val cameraOk = ContextCompat.checkSelfPermission(applicationContext, Manifest.permission.CAMERA) ==
            PackageManager.PERMISSION_GRANTED
        val audioOk = ContextCompat.checkSelfPermission(applicationContext, Manifest.permission.RECORD_AUDIO) ==
            PackageManager.PERMISSION_GRANTED
        if (!cameraOk || !audioOk) {
            pendingConfig = config
            callbacks.onCameraPermissionNeeded()
            return
        }
        startBridge(config)
    }

    /** Set by MainActivity from the JSON config before startCapture (see step-05). */
    @MainThread fun setSrtUrl(url: String) { srtUrl = url }

    /** Called by MainActivity from onRequestPermissionsResult. */
    @MainThread
    fun onPermissionResult(granted: Boolean) {
        val cfg = pendingConfig
        pendingConfig = null
        if (!granted) { callbacks.onCameraCaptureFailed("Camera/audio permission denied"); return }
        if (cfg != null) startBridge(cfg)
    }

    @MainThread
    private fun startBridge(config: CameraCaptureConfig) {
        if (srtUrl.isBlank()) { callbacks.onCameraCaptureFailed("No SRT URL configured"); return }
        if (starting || capturing) { Log.w(TAG, "startBridge while busy"); return }
        val mySession = ++sessionId
        starting = true
        bridge.start(
            config = config,
            srtUrl = srtUrl,
            // The bridge reports the ACTUAL encoder dimensions (possibly 16-aligned,
            // e.g. 1080→1072) so Rust/Slint status matches what is on the wire.
            onStarted = { startedW, startedH ->
                mainHandler.post {
                    if (mySession != sessionId) return@post   // stale: stopped/restarted since
                    starting = false
                    capturing = true
                    callbacks.onCameraCaptureStarted(startedW, startedH, initialRotation(config))
                }
            },
            onError = { reason ->
                mainHandler.post {
                    if (mySession != sessionId) return@post   // stale: stopped/restarted since
                    starting = false
                    capturing = false
                    callbacks.onCameraCaptureFailed(reason)
                }
            },
        )
    }

    @MainThread
    override fun stopCapture() {
        if (!starting && !capturing) return
        sessionId++                   // invalidate any in-flight start callback
        starting = false
        capturing = false
        bridge.stop()                 // safe even if start() is still in flight (step-02 mutex + state machine)
        callbacks.onCameraCaptureStopped()
    }

    @MainThread override fun shutdown() { stopCapture(); bridge.release() }

    // Reflects "user-visible capture active". `starting` is intentionally excluded so the
    // legacy/StreamPack contract matches RealCameraCaptureCoordinator.isCapturing.
    override val isCapturing: Boolean @MainThread get() = capturing

    private fun initialRotation(config: CameraCaptureConfig): Int = when (config.orientationMode) {
        OrientationMode.PORTRAIT  -> 0
        OrientationMode.LANDSCAPE -> 90
        OrientationMode.AUTO      -> 0
    }

    companion object { private const val TAG = "StreamPackCameraCoord" }
}
```

## How to verify

```
✅ Compiles; implements every CameraCaptureCoordinator member.
✅ Unit test (Robolectric) with a fake bridge:
     - stopCapture() before onStarted fires → onCameraCaptureStarted NEVER called.
     - start → onStarted → onCameraCaptureStarted(actualW, actualH).
     - double startCapture() is a no-op (logged).
✅ onCameraCaptureStarted reports the dimensions the BRIDGE returned, not the request.
```

## Risks (carried from master §15)

- **`isCapturing` excludes `STARTING`** (returns `capturing`). Phase-1-safe because the
  real callers (`maybeStartCameraPreview`, `stopCameraPreview`, `surfaceDestroyed`) are
  on the legacy preview path, disabled in StreamPack mode (step-04). Before enabling
  StreamPack preview (step-12), audit any new caller and consider exposing
  `val isActive get() = starting || capturing`.
- **Stop/start race** is closed by the `sessionId` token here + the `Mutex` in step-02.
