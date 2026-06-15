# StreamPack Migration Plan — adapted to the FCast Android sender codebase

> **Status:** design doc. **Nothing in this file has been applied to the codebase.**
> It is the implementation contract; each phase below is independently shippable.
>
> Reference implementation cloned to: `draft/StreamPack-boilerplate/`
> (pins `io.github.thibaultbee.streampack:* = 3.1.1`, the version the API surface below
> was verified against). **The app now targets the latest `3.1.2`** — re-verify the
> version-sensitive bits (`SingleStreamer` construction, `VideoConfig.bitrate`, endpoint
> `Frame`) on the bump; see §15.
>
> Source research: [`draft-plan-to-surface-streampack.md`](../archive/draft-plans/draft-plan-to-surface-streampack.md).
> This document supersedes the draft where the draft drifted from the real code
> (corrections are called out explicitly in **§2 Reality check**).
>
> **Step-by-step split:** this master plan is broken into per-sub-step, independently
> reviewable work items with full code under
> [`docs/streampack-migration/`](./streampack-migration/README.md) (`step-01` … `step-13`).
> Use the master doc for architecture/context; use the step folders to implement.

---

## 0. TL;DR

Keep Rust/Slint as the UI and control owner. Keep `NativeActivity`. Keep GStreamer.
Replace **only** the camera hot path — today's `Camera2 → GL → glReadPixels → I420 →
nativeProcessFrame → GStreamer amcvidenc` — with a StreamPack `Camera2 → Surface →
MediaCodec` encoder.

```
BEFORE encoding:  Android Surface / GL / MediaCodec   (StreamPack owns this)
AFTER  encoding:  Rust / GStreamer / SRT              (we keep this)
```

Three selectable pipeline modes, chosen by Rust/Slint via a feature flag:

| Mode | Camera capture | Encode | Transport | Use |
|------|----------------|--------|-----------|-----|
| `LegacyRawI420Gstreamer` | GL readback (existing) | GStreamer `amcvidenc` | GStreamer `srtsink` | current default / fallback |
| `StreamPackDirectSrt` | StreamPack Surface | StreamPack `MediaCodec` | StreamPack SRT | **Phase 1 validation** |
| `StreamPackEncodedToGstreamer` | StreamPack Surface | StreamPack `MediaCodec` | GStreamer `srtsink`/mux | **target** |

---

## 1. Real architecture (verified against current source)

```
Rust + Slint UI  (ui/*.slint, src/app.rs, src/lib.rs)
   │  Bridge callbacks  (ui/bridge.slint)
   ▼
JNI upcalls  (src/jni_bridge/camera.rs  →  call_method on the Activity)
   ▼
MainActivity : NativeActivity     (app/.../MainActivity.kt)
   │  startCameraCapture(IIIIZZFI)V / stopCameraCapture()V / startCameraPreview / probeCameraPermission
   ▼
RealCameraCaptureCoordinator       (app/.../capture/CameraCaptureCoordinator.kt)
   ▼
CameraCaptureEngine                (app/.../capture/CameraCaptureEngine.kt)
   │  Camera2 → SurfaceTexture → GL Y/U/V FBOs → glReadPixels
   ▼
MainActivity.nativeProcessFrame(w,h,tsNs, Y,U,V)   ← @JvmStatic
   ▼
process_frame()                    (src/jni_bridge/helpers.rs)
   │  copy I420 into VideoBufferPool buffer → crate::FRAME_PAIR
   ▼
CameraSourceNode                   (crates/migration-runtime/src/nodes/camera_source.rs)
   │  appsrc video/x-raw,I420 → videoconvert → videocrop → videoflip → aligncrop → appsink
   ▼
DestinationNode                    (crates/migration-runtime/src/nodes/destination.rs)
   │  videoconvert → amcvidenc(NV12) → h264parse → mpegtsmux → queue → srtsink
   ▼
SRT / RTMP / UDP / WHEP / file
```

Key facts the plan relies on:

- `MainActivity` is `NativeActivity`; `android.app.lib_name = fcastsender`; it loads
  `gstreamer_android` + `fcastsender` and calls `GStreamer.init(this)` in `onCreate`.
- Permissions use the **legacy** `requestPermissions` / `onRequestPermissionsResult`
  path (NativeActivity is not a `ComponentActivity`). `REQ_CAMERA_PERM = 1002`.
- The Rust→Kotlin camera control surface already exists and is **JSON-free** —
  positional JNI signatures in `src/jni_bridge/camera.rs`
  (`startCameraCapture` = `(IIIIZZFI)V`, etc.).
- Graph commands (`createcamerasource`, `createdestination{Srt}`, …) are JSON and
  flow through `native_graph_command` → `migration_runtime::runtime::try_handle_command_json`.

---

## 2. Reality check — corrections to the draft

The draft is directionally correct but assumed a few things that differ from the
checked-in code. The plan below uses the **real** shapes:

| Draft assumed | Reality | Impact on plan |
|---|---|---|
| `app/build.gradle` Groovy, deps added inline | Groovy **+** version catalog `gradle/libs.versions.toml` | Add catalog entries (§3). |
| StreamPack `3.1.2` | Boilerplate pins **`3.1.1`** (API verified against it) | App targets latest **`3.1.2`**; re-verify version-sensitive bits (§3, §15). |
| `MainActivity.cameraCoordinator` is the interface, swap by feature flag | Field is the **concrete** `RealCameraCaptureCoordinator`, and `onPermissionResult()` is called on it directly + `startDefaultCameraPreview()` | Widen the field to the interface and route `onPermissionResult` via `when` (§5.3). |
| `CameraCaptureCoordinator.Callbacks` is top-level | It is a **nested** interface `CameraCaptureCoordinator.Callbacks` | Reference it correctly. |
| Pass SRT URL via `StreamPackNativeConfig.currentSrtUrl()` | No such type; control is positional JNI with **no URL arg** | Add one new JNI upcall carrying a JSON config incl. `srtUrl` (§5.4), leave the legacy positional path untouched. |
| `SingleStreamer(context, withAudio, withVideo)` | 3.1.x exposes a streamer **factory**; the boilerplate injects `SingleStreamer` via a ViewModelFactory | Construct via the documented factory and keep the constructor call behind one private helper so a version bump is a one-line change (§6). |
| Frames currently feed GStreamer `amcvidenc` | Confirmed: `DestinationNode::select_video_encoder` builds `amcvidenc-*`/`x264enc` from the **raw** `CameraSourceNode` appsink | Phase 2 swaps the camera source node to an **H.264 appsrc**, leaving DestinationNode mux/sink intact (§7). |

Everything else in the draft (keep NativeActivity, keep Slint UI, don't pull
`streampack-ui`, phase order) holds.

---

## 3. Dependency changes

### 3.1 `gradle/libs.versions.toml`

```toml
[versions]
# … existing …
streampack = "3.1.2"   # latest on Maven Central (boilerplate was 3.1.1)

[libraries]
# … existing …
streampack-core = { group = "io.github.thibaultbee.streampack", name = "streampack-core", version.ref = "streampack" }
streampack-srt  = { group = "io.github.thibaultbee.streampack", name = "streampack-srt",  version.ref = "streampack" }
# Optional, do NOT enable initially:
# streampack-ui   = { group = "io.github.thibaultbee.streampack", name = "streampack-ui",   version.ref = "streampack" }
# streampack-rtmp = { group = "io.github.thibaultbee.streampack", name = "streampack-rtmp", version.ref = "streampack" }
```

### 3.2 `app/build.gradle` (Groovy)

```groovy
dependencies {
    implementation libs.material
    implementation 'androidx.security:security-crypto:1.1.0-alpha06'
    implementation libs.kotlin.stdlib
    implementation libs.kotlinx.coroutines.android
    implementation libs.androidx.activity.ktx
    implementation libs.androidx.lifecycle.runtime.ktx
    implementation libs.androidx.lifecycle.viewmodel.ktx
    implementation "com.journeyapps:zxing-android-embedded:4.3.0"

    // StreamPack — camera/encoder Surface pipeline.
    implementation libs.streampack.core
    // Phase 1 only: direct SRT egress for end-to-end validation.
    implementation libs.streampack.srt
    // (intentionally NOT streampack-ui — Slint owns the UI)

    // … existing test deps unchanged …
}
```

> **minSdk note:** ours is `26`; the boilerplate is `24`. StreamPack 3.1.x supports
> `minSdk 24`, so no floor change is required.
>
> **ABI note:** we ship `arm64-v8a` only. StreamPack's `MediaCodec`/Camera2 usage is
> pure-Java/NDK-agnostic, so no `Android.mk`/`abiFilters` change is needed for Phase 1.

---

## 4. Manifest — no change required

All four required permissions are already present:
`INTERNET`, `ACCESS_NETWORK_STATE`, `CAMERA`, `RECORD_AUDIO`, plus
`FOREGROUND_SERVICE*`. **Do not** touch the `android.app.lib_name = fcastsender`
meta-data — it is how the Slint/Rust NativeActivity host loads.

For long-running background streaming later (Phase 1 is foreground-only) you may add a
`foregroundServiceType="camera|microphone"` service, but that is **out of scope** for
the first validation.

---

## 5. Phase 1 — StreamPack direct-SRT behind the existing coordinator

Goal: prove the StreamPack Surface→MediaCodec→SRT path works on-device without
touching GStreamer or the raw path. Selected only when the Rust flag says so.

> **Phase 1 limitations (explicit, not hidden in comments):**
>
> - **Mode switching is restart-required.** `camera-pipeline-mode-idx` is persisted but
>   the active coordinator is chosen once in `onCreate` (§5.3). Changing it takes effect
>   on next app launch. Runtime hot-swapping (tear down one coordinator, build the other)
>   is **Phase 2+** — see Option A/B in §5.3.
> - **`cameraIdx` is ignored.** StreamPack uses `defaultCameraId`; front/back/external
>   selection is Phase 2. Phase 1 validates the encoder + transport only.
> - **Requested resolution may be down-aligned.** `alignDown16` means a 1920×1080 request
>   encodes at 1920×1072. The coordinator reports the **actual** encoder size back to
>   Rust/Slint (not the request), so status stays truthful (§6).
> - **No embedded preview** in StreamPack mode (§5.3 disables the legacy preview). Preview
>   fanout is Phase 4.

### 5.1 New file — `app/.../capture/StreamPackCameraCaptureCoordinator.kt`

Implements the **existing** `CameraCaptureCoordinator` interface so nothing upstream
changes. Mirrors `RealCameraCaptureCoordinator`'s lifecycle and permission shape.

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
    // stopCapture() arriving mid-configuration cannot race the async start. See §15.
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

    /** Set by MainActivity from the JSON config before startCapture (see §5.4). */
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
            // e.g. 1080→1072) so Rust/Slint status matches what is on the wire (§6, §15).
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
        bridge.stop()                 // safe even if start() is still in flight (§5.2 mutex + state machine)
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

### 5.2 New file — `app/.../stream/StreamPackSenderBridge.kt`

Encapsulates **all** StreamPack types so the rest of the app never imports them.
API verified against `draft/StreamPack-boilerplate/.../MainViewModel.kt` (3.1.1).

```kotlin
package org.fcast.android.sender.stream

import android.Manifest
import android.content.Context
import android.media.AudioFormat
import android.media.MediaFormat
import android.util.Log
import android.util.Size
import android.view.Surface            // required for Surface.ROTATION_* in rotationFor()
import androidx.annotation.RequiresPermission
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import org.fcast.android.sender.capture.CameraCaptureConfig
import org.fcast.android.sender.capture.OrientationMode

// StreamPack 3.1.1 — package paths verified from the boilerplate MainViewModel/MainActivity.
import io.github.thibaultbee.streampack.core.elements.sources.audio.audiorecord.MicrophoneSourceFactory
import io.github.thibaultbee.streampack.core.elements.sources.video.camera.extensions.defaultCameraId
import io.github.thibaultbee.streampack.core.interfaces.releaseBlocking
import io.github.thibaultbee.streampack.core.interfaces.setCameraId
import io.github.thibaultbee.streampack.core.interfaces.startStream
import io.github.thibaultbee.streampack.core.streamers.single.AudioConfig
import io.github.thibaultbee.streampack.core.streamers.single.SingleStreamer
import io.github.thibaultbee.streampack.core.streamers.single.VideoConfig

/**
 * Thin wrapper over a StreamPack [SingleStreamer]. All StreamPack imports live here.
 *
 * IMPORTANT (version-sensitive): the [SingleStreamer] constructor/factory signature
 * changes between 3.x minors. Keep construction isolated in [newStreamer] so a bump
 * is a one-line edit. Cross-check against:
 *   draft/StreamPack-boilerplate/app/.../MainViewModelFactory.kt
 */
class StreamPackSenderBridge(context: Context) {

    private val appContext = context.applicationContext
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    private val streamer: SingleStreamer = newStreamer(appContext)

    // Internal lifecycle so a stop()/release() arriving mid-start cannot leave a
    // half-configured streamer or double-start it.
    //
    // `scope` is Dispatchers.Default (multi-threaded), so start/stop/release coroutines
    // could otherwise interleave. `mutex` SERIALIZES every streamer mutation. Precise
    // semantics:
    //   - A stop() issued during start() does NOT interrupt a suspending startStream(); it
    //     is serialized to run immediately AFTER the start critical section. The streamer
    //     may therefore briefly start then stop — that is acceptable, and the coordinator's
    //     sessionId (§5.1) prevents a late onStarted from changing UI state after the stop.
    //   - The only thing that can change `state` mid-critical-section is release(), which
    //     flips it to RELEASED outside the lock as a fast-path gate; start() re-checks
    //     `state` after configuration and after startStream() and self-cancels on RELEASED.
    //   - `state` is otherwise read/written only under `mutex`.
    private enum class State { IDLE, STARTING, STREAMING, STOPPING, RELEASED }
    @Volatile private var state = State.IDLE
    private val mutex = Mutex()

    private fun newStreamer(ctx: Context): SingleStreamer =
        // 3.1.1: audio + video enabled. Verify against the boilerplate factory.
        SingleStreamer(context = ctx, withAudio = true, withVideo = true)

    /**
     * @param onStarted invoked with the ACTUAL encoder dimensions once startStream()
     *        returns. These may differ from the requested size because of [alignDown16]
     *        (e.g. 1080 → 1072), so callers must report THESE, not the request (§6, §15).
     */
    @RequiresPermission(allOf = [Manifest.permission.CAMERA, Manifest.permission.RECORD_AUDIO])
    fun start(
        config: CameraCaptureConfig,
        srtUrl: String,
        onStarted: (width: Int, height: Int) -> Unit,
        onError: (String) -> Unit,
    ) {
        val w = alignDown16(config.width)
        val h = alignDown16(config.height)
        scope.launch {
            mutex.withLock {
                if (state != State.IDLE) { onError("StreamPack busy (state=$state)"); return@withLock }
                state = State.STARTING
                try {
                    streamer.setAudioSource(MicrophoneSourceFactory())
                    streamer.setAudioConfig(
                        AudioConfig(
                            mimeType = MediaFormat.MIMETYPE_AUDIO_AAC,
                            sampleRate = 44_100,
                            channelConfig = AudioFormat.CHANNEL_IN_STEREO,
                        )
                    )
                    streamer.setVideoConfig(
                        VideoConfig(
                            mimeType = MediaFormat.MIMETYPE_VIDEO_AVC,
                            resolution = Size(w, h),
                            fps = config.maxFps,
                            bitrate = defaultBitrate(w, h, config.maxFps),
                        )
                    )
                    streamer.setCameraId(cameraIdFor(config.cameraIdx))
                    streamer.setTargetRotation(rotationFor(config.orientationMode))

                    // RELEASED can be set outside the lock; bail without starting.
                    if (state != State.STARTING) { runCatching { streamer.stopStream() }; return@withLock }

                    // For SRT: srt://host:port?streamid=…&passphrase=… (see boilerplate comment).
                    streamer.startStream(srtUrl)

                    // Re-check after the suspending startStream(): a stop() may be queued
                    // behind us on the mutex, but a RELEASED flip can land concurrently.
                    if (state != State.STARTING) { runCatching { streamer.stopStream() }; state = State.IDLE; return@withLock }

                    state = State.STREAMING
                    onStarted(w, h)
                } catch (t: Throwable) {
                    Log.e(TAG, "StreamPack start failed", t)
                    state = State.IDLE
                    onError(t.message ?: "StreamPack start failed")
                }
            }
        }
    }

    fun stop() {
        scope.launch {
            mutex.withLock {
                if (state == State.IDLE || state == State.RELEASED) return@withLock
                state = State.STOPPING
                runCatching { streamer.stopStream() }
                state = State.IDLE
            }
        }
    }

    fun release() {
        // Fast-path gate read by the start() critical section so an in-flight start
        // self-cancels. The blocking release itself is serialized behind the mutex so it
        // can't tear the streamer down mid-startStream().
        state = State.RELEASED
        scope.launch { mutex.withLock { runCatching { streamer.releaseBlocking() } } }
    }

    /**
     * Map our 0=front/1=back/2=external index to a StreamPack camera id.
     *
     * PHASE 1 LIMITATION: cameraIdx is ignored; StreamPack uses [defaultCameraId].
     * Front/back/external selection is Phase 2 (map idx → CameraCharacteristics.LENS_FACING
     * via CameraManager). See §5 limitations.
     */
    private fun cameraIdFor(idx: Int): String = appContext.defaultCameraId

    private fun rotationFor(mode: OrientationMode): Int = when (mode) {
        OrientationMode.PORTRAIT  -> Surface.ROTATION_0
        OrientationMode.LANDSCAPE -> Surface.ROTATION_90
        OrientationMode.AUTO      -> Surface.ROTATION_0
    }

    private fun alignDown16(v: Int) = v - (v % 16)

    private fun defaultBitrate(w: Int, h: Int, fps: Int): Int = when {
        w >= 1920 || h >= 1080 -> if (fps > 30) 8_000_000 else 6_000_000
        w >= 1280 || h >= 720  -> if (fps > 30) 5_000_000 else 3_500_000
        else -> 2_000_000
    }

    companion object { private const val TAG = "StreamPackSenderBridge" }
}
```

> **`alignDown16` rationale:** our raw path proved 1080 → coded 1088 padding shows as a
> green edge at the receiver (`camera_source.rs` `align_crop` + `helpers.rs` dump
> tooling). StreamPack's Surface path likely avoids it, but keeping encoder dims
> 16-aligned during migration is the safe baseline.

### 5.3 `AppGraph` factory + `MainActivity` wiring (diff against real code)

`app/.../AppGraph.kt` — add a factory next to `newCaptureCoordinator`:

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

> **Mode selection is an `onCreate`-only decision in Phase 1 (restart-required).**
> The runtime Slint selector (§9) persists the mode but does **not** hot-swap the live
> coordinator. Two options exist; Phase 1 takes **Option A**:
>
> ```text
> Option A (Phase 1 — chosen):
>   Pipeline mode changes are persisted but only take effect after app restart.
>   Simpler and race-free; the selector shows the value that will apply next launch.
>
> Option B (Phase 2+):
>   On change: stopCapture() → destroyCameraPreview() → shutdown old coordinator →
>   build the other coordinator → attach() → optionally restart preview/capture.
> ```
>
> Without this rule, a runtime selector would *appear* to switch modes while leaving the
> old coordinator instance alive — exactly the trap to avoid.

`app/.../MainActivity.kt` — four surgical edits:

1. **Widen the field type** so either implementation fits
   (today it is the concrete `RealCameraCaptureCoordinator`):

```kotlin
// before:  private lateinit var cameraCoordinator: RealCameraCaptureCoordinator
private lateinit var cameraCoordinator: CameraCaptureCoordinator
```

2. **Pick the implementation in `onCreate`** behind the Rust flag
   (replaces the direct `RealCameraCaptureCoordinator(...)` construction):

```kotlin
cameraCoordinator = if (nativeUseStreamPackCameraPath()) {
    (application as FcastApp).graph.newStreamPackCameraCoordinator(cameraCallbacks)
} else {
    RealCameraCaptureCoordinator(applicationContext, cameraCallbacks)
}
cameraCoordinator.attach()
```

3. **Route `onPermissionResult` to whichever impl is active.** The interface has no
   `onPermissionResult`, so smart-cast (both impls expose the same method shape):

```kotlin
// in onRequestPermissionsResult(...) and the onCreate proactive-grant branch:
when (val c = cameraCoordinator) {
    is RealCameraCaptureCoordinator       -> c.onPermissionResult(cameraGranted)
    is StreamPackCameraCaptureCoordinator -> c.onPermissionResult(cameraGranted)
}
```

4. **Gate the legacy preview on the active mode.** The current `onCreate` proactive-grant
   branch and `onRequestPermissionsResult` both call `startDefaultCameraPreview()`
   unconditionally. In StreamPack mode that creates a `SurfaceView` no one renders to —
   a confusing half-state. Guard it, and tear any preview down before StreamPack capture:

```kotlin
// wherever startDefaultCameraPreview() is currently called:
if (!nativeUseStreamPackCameraPath()) {
    startDefaultCameraPreview()
}
```

```kotlin
// in startStreamPackCamera(...) (§5.4), before startCapture, ensure the legacy
// SurfaceView preview is gone so camera ownership is unambiguous:
if (nativeUseStreamPackCameraPath()) {
    destroyCameraPreview()
}
```

> Net effect: in StreamPack mode the legacy `cameraPreviewSurface`/SurfaceView path is
> never created. Embedded preview returns in Phase 4 via a `SurfaceProcessor` fanout.

Add the native flag declaration with the other `external` shims:

```kotlin
private external fun nativeUseStreamPackCameraPath(): Boolean
```

### 5.4 Carrying the SRT URL across JNI (new upcall, legacy path untouched)

The existing `startCameraCapture(IIIIZZFI)V` has **no URL argument**. Rather than
overload it, add one JSON-carrying Kotlin entry + matching Rust upcall. JSON matches
the project's existing `native_graph_command` convention.

`MainActivity.kt` — new native-callable method:

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
        (cameraCoordinator as? StreamPackCameraCaptureCoordinator)?.setSrtUrl(j.optString("srtUrl"))
        cameraCoordinator.startCapture(cfg, cameraPreviewSurface?.takeIf { it.isValid })
    }
}
```

`src/jni_bridge/camera.rs` — new upcall (mirrors `upcall_start_camera_capture`):

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

> **`stopCameraCapture()` must stay pipeline-mode-neutral.** After the field is widened
> (§5.3), it should call `cameraCoordinator.stopCapture()` through the `CameraCaptureCoordinator`
> interface only — no `RealCameraCaptureCoordinator`-specific code. Both Phase 1 (direct
> SRT) and Phase 2 (encoded→GStreamer) then share one stop path; do **not** add a separate
> StreamPack stop upcall. The existing `maybeStartCameraPreview()` follow-up in
> `stopCameraCapture()` is already a no-op in StreamPack mode because no preview was created
> (§5.3 edit 4).

> The `nativeUseStreamPackCameraPath()` JNI export pairs with a Rust function reading
> the new config flag (§8). Add its `Java_org_fcast_android_sender_MainActivity_*`
> re-export in `src/lib.rs` next to the existing native symbols.

### 5.5 Phase 1 validation checklist

```
1. App still launches; Slint NativeActivity renders; GStreamer.init(this) succeeds.
2. Flag OFF → legacy raw path identical to today (regression guard).
3. Flag ON  → StreamPack starts at 1280x720@30; SRT receiver sees A/V.
4. Start/stop 20× with no leak / no crash (watch `releaseBlocking`).
5. No green edge at 1280x720.
6. Test 1920x1072, then 1920x1080.
7. Compare CPU%, thermals, latency vs legacy path (this is the payoff).
```

Receiver for testing:

```bash
gst-launch-1.0 srtsrc uri="srt://:9000?mode=listener" ! tsdemux ! h264parse \
  ! avdec_h264 ! videoconvert ! autovideosink
```

---

## 6. Note on StreamPack 3.1.1 API (verified)

From `draft/StreamPack-boilerplate/.../MainViewModel.kt` & `MainActivity.kt`:

```kotlin
streamer.setAudioSource(MicrophoneSourceFactory())
streamer.setAudioConfig(AudioConfig(mimeType, sampleRate, channelConfig))
streamer.setVideoConfig(VideoConfig(mimeType, resolution = Size(w,h), fps, /*bitrate*/))
streamer.setCameraId(context.defaultCameraId)   // extension: core.interfaces.setCameraId
streamer.setTargetRotation(rotation)
streamer.startStream("srt://…")                  // extension: core.interfaces.startStream
streamer.stopStream()
streamer.releaseBlocking()                       // extension: core.interfaces
streamer.isStreamingFlow      // Flow<Boolean>
streamer.throwableFlow        // Flow<Throwable?>
// PreviewView (streampack-ui, NOT used here): preview.setVideoSourceProvider(streamer)
```

The only construction detail to confirm on the version bump is the `SingleStreamer`
factory/constructor — isolated in `StreamPackSenderBridge.newStreamer`.

---

## 7. Phase 2 — bridge StreamPack encoded frames into Rust/GStreamer

Once direct SRT is stable, stop using StreamPack's transport and route **encoded
H.264** into the existing GStreamer egress (mux/sink/SRT control stays in Rust).

### 7.1 Kotlin — custom StreamPack endpoint

Implement StreamPack's internal endpoint interface to receive `Frame`s and forward
them over JNI. (Field names like `timestampInUs`/`flags`/`buffer` are version-sensitive
— confirm against the 3.1.1 `Frame`/`IEndpointInternal` types before wiring.)

```kotlin
package org.fcast.android.sender.stream

import java.nio.ByteBuffer
// import io.github.thibaultbee.streampack.core.elements.endpoints.IEndpointInternal
// import io.github.thibaultbee.streampack.core.elements.encoders.CodecConfig
// … (resolve exact 3.1.1 endpoint package; the boilerplate only uses built-in SRT/RTMP)

/** Routes StreamPack-encoded frames to Rust → GStreamer instead of StreamPack SRT. */
class RustGStreamerEndpoint /* : IEndpointInternal */ {
    private var nextPid = 256

    fun openPipeline()  = nativeOpenEncodedGStreamerPipeline()
    fun closePipeline() = nativeCloseEncodedGStreamerPipeline()
    fun startPipeline() = nativeStartEncodedGStreamerPipeline()
    fun stopPipeline()  = nativeStopEncodedGStreamerPipeline()

    fun addStream(mimeType: String): Int {
        val pid = nextPid++
        nativeAddEncodedStream(pid, mimeType)
        return pid
    }

    /** Called per encoded access unit. Forward then release the StreamPack buffer. */
    fun write(streamPid: Int, timestampNs: Long, flags: Int, buffer: ByteBuffer) {
        nativeWriteEncodedFrame(streamPid, timestampNs, flags, buffer, buffer.remaining())
    }

    private external fun nativeOpenEncodedGStreamerPipeline()
    private external fun nativeCloseEncodedGStreamerPipeline()
    private external fun nativeStartEncodedGStreamerPipeline()
    private external fun nativeStopEncodedGStreamerPipeline()
    private external fun nativeAddEncodedStream(pid: Int, mimeType: String)
    private external fun nativeWriteEncodedFrame(
        streamPid: Int, timestampNs: Long, flags: Int, buffer: ByteBuffer, size: Int,
    )
}
```

### 7.2 Rust — encoded-frame ingest (new JNI symbol, parallels `nativeProcessFrame`)

Add to `src/jni_bridge/main_activity.rs` (and re-export in `src/lib.rs`):

```rust
#[cfg(target_os = "android")]
pub fn native_write_encoded_frame<'local>(
    env: JNIEnv<'local>,
    _class: JClass<'local>,
    stream_pid: jni::sys::jint,
    timestamp_ns: jni::sys::jlong,
    flags: jni::sys::jint,
    buffer: JByteBuffer<'local>,
    size: jni::sys::jint,
) {
    if let Err(err) = crate::jni_bridge::helpers::push_encoded_frame(
        env, stream_pid, timestamp_ns, flags, buffer, size,
    ) {
        error!(?err, "push_encoded_frame failed");
    }
}
```

`helpers.rs` `push_encoded_frame` mirrors `process_frame` but pushes the raw access
unit into a new H.264 `FramePair`-style channel (a `Vec<u8>` + PTS, not a
`VideoFrame`). Keep the existing `FRAME_PAIR` (raw) for the legacy path; add
`H264_FRAME_PAIR` for encoded.

### 7.3 Rust — H.264 camera source node

Add a sibling to `CameraSourceNode` (don't mutate the raw one; mode selects which is
built). New appsrc caps and a parser; **drop** `videoconvert`/`videoflip`/`videocrop`
(rotation/mirror now happen in the StreamPack encoder/SurfaceProcessor):

```rust
// crates/migration-runtime/src/nodes/camera_source.rs  (new builder variant)
let appsrc = AppSrc::builder()
    .name(format!("camera-h264-appsrc-{}", self.id))
    .format(gst::Format::Time)
    .is_live(true)
    .do_timestamp(true)
    .stream_type(gst_app::AppStreamType::Stream)
    .caps(
        &gst::Caps::builder("video/x-h264")
            .field("stream-format", "byte-stream")
            .field("alignment", "au")
            .build(),
    )
    .build();

let queue = gst::ElementFactory::make("queue")
    .name(format!("camera-h264-queue-{}", self.id))
    .property("max-size-buffers", 8u32)
    .property_from_str("leaky", "downstream")
    .build()
    .map_err(|e| format!("queue: {}", e.message))?;

let parse = gst::ElementFactory::make("h264parse")
    .name(format!("camera-h264parse-{}", self.id))
    .build()
    .map_err(|e| format!("h264parse: {}", e.message))?;
if parse.has_property("config-interval") {
    parse.set_property("config-interval", -1i32); // repeat SPS/PPS in-band
}
// appsrc → queue → h264parse → appsink   (feeds the existing DestinationNode mux/sink)
```

`wire_need_data` pulls from `H264_FRAME_PAIR` and pushes each access unit; set buffer
flags for keyframes from the Kotlin `flags` arg.

> **⚠ Confirm the H.264 bitstream format before pinning these caps.** The caps above
> (`stream-format=byte-stream, alignment=au`) are correct **only if** the StreamPack
> endpoint hands us **Annex B** access units (start-code prefixed `00 00 00 01`, SPS/PPS
> in-band). Android `MediaCodec` can instead emit **AVCC** (length-prefixed NAL units)
> with the codec config (`csd-0`/`csd-1` = SPS/PPS) delivered **separately** via
> `BUFFER_FLAG_CODEC_CONFIG`. If so, the bridge must either:
>   - configure the encoder/endpoint for byte-stream output, **or**
>   - push `stream-format=avc` + set `codec_data` on the caps from the CSD buffers, and
>     forward the config NAL on stream start (don't drop the `CODEC_CONFIG` frame).
>
> This is the same risk class as the endpoint `Frame` field names — read it from the
> 3.1.1 sources/`MediaFormat` on first encoded frame and set caps to match. The keyframe
> flag mapping (`flags` → `gst::BufferFlags::DELTA_UNIT` cleared on IDR) depends on this too.

### 7.4 Rust — DestinationNode stays mux/sink only

Because the camera source is now already H.264, the destination must **not** re-encode.
Add a "pre-encoded video" path so `DestinationFamily::Srt` does
`appsrc(h264) → h264parse → mpegtsmux → queue → srtsink` and **skips**
`select_video_encoder`. Gate it on the same Rust pipeline-mode flag so the legacy raw
destinations are untouched. Audio (AAC) still encodes in GStreamer unless StreamPack
also supplies AAC (then mux both pre-encoded streams).

---

## 8. The pipeline-mode flag (Rust = source of truth)

```rust
// e.g. src/config/mod.rs
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

- Persist it with the rest of the config (`src/backend/persistence.rs`).
- `nativeUseStreamPackCameraPath()` returns
  `!matches!(mode, AndroidCameraPipeline::LegacyRawI420Gstreamer)`.
- Phase 2 vs Phase 1 within StreamPack is chosen Rust-side (which camera source node
  + which destination video path to build).

---

## 9. Slint surface (control only)

`ui/bridge.slint` already exposes the camera controls
(`camera-idx`, `camera-orientation-mode-idx`, `camera-mirror-front`,
`camera-stabilization`, `camera-zoom-level`) and the camera-RTMP page
(`cam-rtmp-url`, `start-camera-rtmp-stream()`, `stop-camera-rtmp-stream()`,
`start-camera-rtmp-preview()`) plus `srt-destination` + `start-srt-destination()`.

Add a single mode selector; reuse the existing start/stop callbacks.

```slint
// ui/bridge.slint  (inside global Bridge)
// 0=Legacy 1=StreamPackDirectSrt 2=StreamPackEncodedToGStreamer
in-out property <int> camera-pipeline-mode-idx: 0;
callback set-camera-pipeline-mode(int);
```

```slint
// ui/pages/camera_page.slint — a settings row, same shape as the Orientation row
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

Wire `set-camera-pipeline-mode` in `src/app.rs` to persist the
`AndroidCameraPipeline` value. The existing `start-camera-rtmp-stream` /
`start-srt-destination` handlers branch on the mode: legacy modes keep calling
`upcall_start_camera_capture` + `createcamerasource`; StreamPack-direct calls
`upcall_start_streampack_camera(config_json)`.

---

## 10. Phase 3 — retire the GL readback for streaming

Keep `CameraCaptureEngine` + `nativeProcessFrame` strictly as: legacy fallback,
regression baseline, and the frame-dump debug tool (the `FCAST_DUMP_DIR` tooling in
`helpers.rs` / `camera_source.rs` and `scripts/dump-frames.sh`). Do not delete —
StreamPack can fail on some devices and the dump path is the diagnostic.

## 11. Phase 4 — preview

Phase 1–3 validate egress without embedded preview. Then progress:

```
Phase 1: no embedded preview (validate egress only)
Phase 2: keep legacy SurfaceView preview when NOT streaming
Phase 3: custom SurfaceProcessor fanout — camera surface → encoder surface
         → NativeActivity/Slint preview surface
```

Avoid `streampack-ui` `PreviewView` (it assumes a normal Android view tree; Slint owns
ours). StreamPack's customizable `SurfaceProcessor` is the right hook.

## 12. Phase 5 — OBS-style SurfaceProcessor (long-term)

```
Slint/Rust scene graph → JNI scene updates → custom SurfaceProcessor / GL compositor
  (camera + screen + image/text overlays, crop/scale/rotate/mirror)
  → MediaCodec encoder surface → encoded frames → Rust/GStreamer egress
```

Ownership split:

- **Rust/Slint:** scenes, source positions/visibility, transitions, stream/record
  state, remote control.
- **Android:** Camera2, MediaProjection, SurfaceTexture, EGL, MediaCodec, StreamPack
  encoder lifecycle.
- **GStreamer/Rust:** SRT/mux, recording variants, diagnostics, capability reporting,
  legacy fallback.

---

## 13. What NOT to do

- ❌ Don't replace the Slint UI with `streampack-ui` `PreviewView`.
- ❌ Don't extract **raw** frames from StreamPack back into `nativeProcessFrame` — that
  throws away the whole point (no CPU I420 readback).
- ❌ Don't delete GStreamer — it remains the post-encode transport/mux/diagnostics owner.
- ❌ Don't widen the legacy positional JNI signature; add the JSON upcall instead.

---

## 14. Implementation order (success criteria, not just steps)

1. **Deps** (§3) — project syncs with `streampack-core` + `streampack-srt`; legacy
   build unchanged. ✅ when Gradle sync + existing instrumentation builds pass.
2. **Bridge + Coordinator** (§5.1–5.2) — compiles; no behavior change while flag OFF.
3. **Flag + wiring** (§5.3–5.4, §8, §9) — the Slint row persists the mode; the chosen
   path is active **after restart** (Option A); legacy path byte-for-byte identical when OFF.
4. **Validate direct SRT** (§5.5) — receiver shows A/V at 720p, then 1072, then 1080;
   CPU/thermals beat the legacy path.
5. **Encoded endpoint** (§7.1–7.2) — encoded frames arrive in Rust (log AU sizes/PTS).
6. **H.264 camera source node + mux-only destination** (§7.3–7.4) — SRT out via
   GStreamer from StreamPack-encoded frames.
7. **Retire raw path for streaming** (§10) — keep as fallback/diagnostic.
8. **Preview + SurfaceProcessor** (§11–§12).

---

## 15. Open risks / things to confirm before coding

- **Version bump 3.1.1 → 3.1.2:** the API surface in this doc was read from the 3.1.1
  boilerplate; the app pins the latest `3.1.2`. Re-confirm the version-sensitive call
  sites against the 3.1.2 artifact before/at first compile: `SingleStreamer`
  constructor/factory (isolated in `newStreamer`, cross-check
  `draft/StreamPack-boilerplate/.../MainViewModelFactory.kt`), the `VideoConfig(... bitrate = ...)`
  named parameter, and (Phase 2) the endpoint `Frame`/`IEndpointInternal` shape. If any
  differ, the change is localized to `StreamPackSenderBridge` (Phase 1) or the endpoint
  (Phase 2).
- **Phase 2 endpoint API:** the boilerplate only uses built-in SRT/RTMP, so the
  `IEndpointInternal`/`Frame` field names in §7.1 must be read from the 3.1.1 sources
  (Maven artifact) before wiring.
- **Audio muxing in Phase 2:** decide whether StreamPack also emits AAC (mux both
  pre-encoded) or GStreamer keeps encoding mic audio.
- **Rotation parity:** legacy path computes `videoflip` from
  `SENSOR_ORIENTATION + deviceRotation` (`CameraCaptureEngine.calcVideoRotation`);
  StreamPack uses `setTargetRotation`. Verify AUTO-mode rotation matches the legacy
  output before retiring the raw path.
- **`release()` is asynchronous in Phase 1.** `shutdown() = stopCapture() + release()` no
  longer guarantees the StreamPack instance is fully torn down before the next statement.
  Fine for `onDestroy`; **not** fine for a caller (test, or future runtime mode switch —
  Option B §5.3) that immediately constructs another `StreamPackSenderBridge` and assumes
  the old `MediaCodec`/Camera resources are freed. Before relying on immediate teardown,
  add an awaitable variant — e.g. `fun release(): Job` (return the launched job) plus a
  `fun releaseBlockingForTests()` — and await/join it.
- **Background/foreground service:** Phase 1 is foreground-only; long-running camera
  streaming needs a `camera|microphone` foreground service later.
- **Runtime mode switching:** if `camera-pipeline-mode-idx` can change while running,
  `MainActivity` must recreate the coordinator (Option B, §5.3). **Phase 1 treats mode
  changes as restart-required (Option A)** — the selector previews the next-launch value.
- **Default preview conflict:** `startDefaultCameraPreview()` must be disabled in
  StreamPack mode and any legacy preview destroyed before StreamPack capture (§5.3 edit 4),
  or you get a SurfaceView no one renders to plus camera-ownership surprises.
- **Reported resolution:** `alignDown16` means requested ≠ actual encoder size
  (1080→1072). The bridge reports the actual size via `onStarted(w, h)` and the
  coordinator forwards THAT to `onCameraCaptureStarted` (§5.1–5.2, §6) — never the request.
- **Start/stop races:** `StreamPackSenderBridge` serializes all streamer mutations behind
  a `Mutex` over its `IDLE/STARTING/STREAMING/STOPPING/RELEASED` state machine, and the
  coordinator stamps each start with a monotonic `sessionId` so a late `onStarted`/`onError`
  is dropped if a stop/restart intervened. Together these stop a `stop()` issued during
  async configuration from half-starting, double-starting, or resurrecting the streamer
  (§5.1–5.2).
- **`isCapturing` excludes `STARTING`:** it returns `capturing` only (to match
  `RealCameraCaptureCoordinator`'s contract), so during the async start window it reads
  `false`. Verified Phase-1-safe: the real callers in `MainActivity`
  (`maybeStartCameraPreview`, `stopCameraPreview`, the `surfaceDestroyed` guard) are all on
  the **legacy preview path**, which is disabled in StreamPack mode (§5.3 edit 4). **Before
  enabling StreamPack preview (Phase 4)**, audit any new caller that treats
  `isCapturing == false` as "safe to create preview / start again" — if one appears, expose
  `val isActive get() = starting || capturing` and gate on that instead.
- **H.264 stream format (Phase 2):** confirm Annex B (byte-stream) vs AVCC
  (length-prefixed + separate CSD/SPS-PPS) from the endpoint, and set appsrc
  `stream-format`/`codec_data` + keyframe flags accordingly (§7.3).
- **Stop path neutrality:** `stopCameraCapture()` must call the interface
  `stopCapture()` only — one shared stop path across modes (§5.4).
```
