Below is the StreamPack migration plan rewritten for **your actual architecture**:

```text
Rust + Slint UI
  → Android NativeActivity host
  → Kotlin/Java platform bridge
  → Rust JNI runtime / GStreamer / migration-runtime services
```

The important change from the previous plan is this: **do not replace your Slint UI with StreamPack UI**. Your app already uses `MainActivity : NativeActivity()`, loads the native library via `android.app.lib_name = fcastsender`, initializes GStreamer, and routes camera frames into Rust through JNI.  

So the correct integration is:

```text
Keep:
  Slint UI in Rust
  NativeActivity
  AppGraph / JniRuntimeBridge
  MigrationRuntimeService / GStreamer runtime
  Rust-side app state

Replace:
  CameraCaptureEngine raw I420 readback path
  MainActivity.nativeProcessFrame(...) camera hot path
  GStreamer raw-video → amcvidenc camera encoding path

Add:
  StreamPack-based Android media bridge
  StreamPack Surface/MediaCodec camera encoder path
  Optional encoded-frame bridge back into Rust/GStreamer
```

StreamPack currently exposes separate artifacts such as `streampack-core`, `streampack-srt`, `streampack-rtmp`, `streampack-services`, and `streampack-ui`; for your architecture, `streampack-ui` is optional because Slint owns the UI. ([GitHub][1])

---

# 1. Target architecture

Your current camera path is effectively:

```text
Camera2
  → SurfaceTexture / GL
  → render Y plane
  → render U plane
  → render V plane
  → glReadPixels()
  → ByteBuffer Y/U/V
  → MainActivity.nativeProcessFrame(...)
  → Rust / GStreamer appsrc
  → amcvidenc
  → SRT
```

That is visible in your `CameraCaptureEngine`: it creates a `SurfaceTexture`, renders to Y/U/V framebuffers, calls `readPixels()` on each plane, and finally calls `MainActivity.nativeProcessFrame(width, height, timestampNs, y, u, v)`. 

The StreamPack-based target should be:

```text
Rust + Slint UI
  → JNI command to Kotlin bridge
  → StreamPack camera source
  → StreamPack SurfaceProcessor
  → Android MediaCodec input Surface
  → H.264/H.265 encoded frames
  → either:
       A) StreamPack SRT directly
       B) custom endpoint → Rust/GStreamer compressed pipeline
```

For an OBS-like Android app, I recommend **B** as the long-term target:

```text
Camera / screen / future scene compositor
  → StreamPack-style Surface/MediaCodec encoder
  → encoded H.264/AAC frames
  → Rust/GStreamer:
       appsrc video/x-h264
       ! h264parse
       ! mpegtsmux
       ! srtsink
```

This keeps the high-performance Android camera/encoder path while preserving GStreamer for transport, muxing, diagnostics, and future output flexibility.

---

# 2. Why this fits your codebase

Your Android side is already a platform bridge, not the real UI. `AppGraph` is your composition root and exposes `RuntimeBridge` through `JniRuntimeBridge`; services and activities read dependencies from there instead of constructing everything ad hoc. 

Your current `CameraCaptureCoordinator` is also already an abstraction boundary:

```kotlin
interface CameraCaptureCoordinator {
    @MainThread fun attach()
    @MainThread fun startPreview(config: CameraCaptureConfig, previewSurface: Surface)
    @MainThread fun stopPreview()
    @MainThread fun startCapture(config: CameraCaptureConfig)
    @MainThread fun startCapture(config: CameraCaptureConfig, previewSurface: Surface?)
    @MainThread fun stopCapture()
    @MainThread fun shutdown()
    val isCapturing: Boolean
}
```

That means the right migration is **not** to rewrite `MainActivity` around StreamPack’s `PreviewView`. Instead, create a new implementation:

```text
RealCameraCaptureCoordinator
  current: CameraCaptureEngine → raw YUV → nativeProcessFrame

StreamPackCameraCaptureCoordinator
  new: StreamPackBridge → MediaCodec Surface encoder → SRT or encoded bridge
```

Then select between them with a feature flag.

---

# 3. Dependency changes

Your `app/build.gradle` is Groovy, uses `compileSdk 36`, `minSdk 26`, `ndkVersion 28.0.13004108`, Java 17, `viewBinding`, AIDL, Kotlin stdlib, coroutines, lifecycle, and AndroidX Activity KTX. 

Add StreamPack dependencies in Groovy style.

```gradle
dependencies {
    implementation libs.material
    implementation 'androidx.security:security-crypto:1.1.0-alpha06'
    implementation libs.kotlin.stdlib
    implementation libs.kotlinx.coroutines.android
    implementation libs.androidx.activity.ktx
    implementation libs.androidx.lifecycle.runtime.ktx
    implementation libs.androidx.lifecycle.viewmodel.ktx

    // StreamPack core camera/encoder pipeline.
    implementation 'io.github.thibaultbee.streampack:streampack-core:3.1.2'

    // Use this only if StreamPack should send SRT directly.
    implementation 'io.github.thibaultbee.streampack:streampack-srt:3.1.2'

    // Optional. Do NOT use PreviewView as the main UI because Slint owns UI.
    // implementation 'io.github.thibaultbee.streampack:streampack-ui:3.1.2'

    // Optional, only for RTMP fallback/testing.
    // implementation 'io.github.thibaultbee.streampack:streampack-rtmp:3.1.2'
}
```

StreamPack’s README lists `streampack-core`, `streampack-ui`, `streampack-services`, `streampack-rtmp`, and `streampack-srt` artifacts at `3.1.2`. ([GitHub][1])

For your app, start with:

```text
Required:
  streampack-core

Temporary validation path:
  streampack-srt

Avoid initially:
  streampack-ui
  streampack-services
```

Reason: your `MainActivity` is a `NativeActivity`, and Slint/Rust is the UI. Pulling in StreamPack UI would fight your existing rendering model.

---

# 4. Manifest changes

Your manifest already has the core permissions:

```xml
<uses-permission android:name="android.permission.INTERNET" />
<uses-permission android:name="android.permission.ACCESS_NETWORK_STATE" />
<uses-permission android:name="android.permission.CAMERA" />
<uses-permission android:name="android.permission.RECORD_AUDIO" />
```

It also already has foreground service permissions and services for screen capture, GStreamer, migration runtime, and codec benchmarking. 

For the first StreamPack camera-only migration, you probably do **not** need major manifest changes. Keep your existing `MainActivity : NativeActivity` and `android.app.lib_name = fcastsender`.

Do **not** replace this:

```xml
<meta-data
    android:name="android.app.lib_name"
    android:value="fcastsender" />
```

That is part of your Rust/Slint NativeActivity hosting model. 

---

# 5. Replace the raw camera engine behind your existing coordinator

## Current implementation

Your current `RealCameraCaptureCoordinator` owns a `CameraCaptureEngine`. It handles permission checks, preview/capture mode, pending config, and callbacks into `MainActivity`. 

Keep this public interface.

Add a new implementation:

```kotlin
package org.fcast.android.sender.capture

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.os.Handler
import android.os.Looper
import android.view.Surface
import androidx.annotation.MainThread
import androidx.core.content.ContextCompat
import org.fcast.android.sender.stream.StreamPackSenderBridge

class StreamPackCameraCaptureCoordinator(
    private val applicationContext: Context,
    private val callbacks: CameraCaptureCoordinator.Callbacks,
    private val bridge: StreamPackSenderBridge = StreamPackSenderBridge(applicationContext),
) : CameraCaptureCoordinator {

    private val mainHandler = Handler(Looper.getMainLooper())
    private var pendingConfig: CameraCaptureConfig? = null
    private var capturing = false

    @MainThread
    override fun attach() {
        // No-op for now. Keep the same lifecycle shape as RealCameraCaptureCoordinator.
    }

    @MainThread
    override fun startPreview(config: CameraCaptureConfig, previewSurface: Surface) {
        // In the Slint NativeActivity architecture, preview should be handled by:
        //   1. Slint-provided Android surface, or
        //   2. a later custom SurfaceProcessor fanout.
        //
        // For phase 1, do not wire StreamPack PreviewView.
        // Keep this method as a no-op or legacy fallback.
    }

    @MainThread
    override fun stopPreview() {
        // No-op in phase 1.
    }

    @MainThread
    override fun startCapture(config: CameraCaptureConfig) {
        startCapture(config, null)
    }

    @MainThread
    override fun startCapture(config: CameraCaptureConfig, previewSurface: Surface?) {
        if (capturing) return

        val cameraGranted =
            ContextCompat.checkSelfPermission(applicationContext, Manifest.permission.CAMERA) ==
                PackageManager.PERMISSION_GRANTED

        val audioGranted =
            ContextCompat.checkSelfPermission(applicationContext, Manifest.permission.RECORD_AUDIO) ==
                PackageManager.PERMISSION_GRANTED

        if (!cameraGranted || !audioGranted) {
            pendingConfig = config
            callbacks.onCameraPermissionNeeded()
            return
        }

        startBridge(config)
    }

    @MainThread
    fun onPermissionResult(granted: Boolean) {
        val cfg = pendingConfig
        pendingConfig = null

        if (!granted) {
            callbacks.onCameraCaptureFailed("Camera/audio permission denied")
            return
        }

        if (cfg != null) {
            startBridge(cfg)
        }
    }

    @MainThread
    private fun startBridge(config: CameraCaptureConfig) {
        capturing = true

        try {
            bridge.configure(config)

            bridge.startDirectSrt(
                // Temporary validation URL. Later this comes from Rust/Slint state.
                url = StreamPackNativeConfig.currentSrtUrl()
            )

            callbacks.onCameraCaptureStarted(
                config.width,
                config.height,
                rotationDegForInitialReport(config)
            )
        } catch (t: Throwable) {
            capturing = false
            callbacks.onCameraCaptureFailed(t.message ?: "StreamPack start failed")
        }
    }

    @MainThread
    override fun stopCapture() {
        if (!capturing) return
        capturing = false

        bridge.stop()
        callbacks.onCameraCaptureStopped()
    }

    @MainThread
    override fun shutdown() {
        stopCapture()
        bridge.release()
    }

    override val isCapturing: Boolean
        @MainThread get() = capturing

    private fun rotationDegForInitialReport(config: CameraCaptureConfig): Int {
        return when (config.orientationMode) {
            OrientationMode.PORTRAIT -> 0
            OrientationMode.LANDSCAPE -> 90
            OrientationMode.AUTO -> 0
        }
    }
}
```

This keeps the Rust/Slint-facing control contract stable while replacing only the hot camera path.

---

# 6. Add the StreamPack bridge class

Create:

```text
app/src/main/java/org/fcast/android/sender/stream/StreamPackSenderBridge.kt
```

```kotlin
package org.fcast.android.sender.stream

import android.content.Context
import android.media.AudioFormat
import android.media.MediaFormat
import android.util.Size
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch
import org.fcast.android.sender.capture.CameraCaptureConfig
import org.fcast.android.sender.capture.OrientationMode

// Imports may need small adjustments based on the exact StreamPack 3.1.2 package names.
import io.github.thibaultbee.streampack.core.streamers.single.SingleStreamer
import io.github.thibaultbee.streampack.core.streamers.single.VideoConfig
import io.github.thibaultbee.streampack.core.streamers.single.AudioConfig
import io.github.thibaultbee.streampack.core.elements.sources.audio.audiorecord.MicrophoneSourceFactory
import io.github.thibaultbee.streampack.core.elements.sources.video.camera.extensions.defaultCameraId
import io.github.thibaultbee.streampack.core.interfaces.releaseBlocking
import io.github.thibaultbee.streampack.core.interfaces.setCameraId
import io.github.thibaultbee.streampack.core.interfaces.startStream

class StreamPackSenderBridge(
    context: Context,
) {
    private val appContext = context.applicationContext
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    private val streamer = SingleStreamer(
        context = appContext,
        withAudio = true,
        withVideo = true,
    )

    @Volatile
    private var configured = false

    fun configure(config: CameraCaptureConfig) {
        val safeWidth = alignDown16(config.width)
        val safeHeight = alignDown16(config.height)

        scope.launch {
            streamer.setAudioSource(MicrophoneSourceFactory())

            streamer.setAudioConfig(
                AudioConfig(
                    mimeType = MediaFormat.MIMETYPE_AUDIO_AAC,
                    sampleRate = 44100,
                    channelConfig = AudioFormat.CHANNEL_IN_STEREO,
                )
            )

            streamer.setVideoConfig(
                VideoConfig(
                    mimeType = MediaFormat.MIMETYPE_VIDEO_AVC,
                    resolution = Size(safeWidth, safeHeight),
                    fps = config.maxFps,
                    // Pick a sane default. Later wire this from Slint/Rust config.
                    bitrate = defaultBitrateFor(safeWidth, safeHeight, config.maxFps),
                )
            )

            streamer.setCameraId(appContext.defaultCameraId)

            val rotation = when (config.orientationMode) {
                OrientationMode.PORTRAIT -> 0
                OrientationMode.LANDSCAPE -> 90
                OrientationMode.AUTO -> 0
            }
            streamer.setTargetRotation(rotation)

            configured = true
        }
    }

    fun startDirectSrt(url: String) {
        scope.launch {
            check(configured) { "StreamPackSenderBridge must be configured before start" }
            streamer.startStream(url)
        }
    }

    fun stop() {
        scope.launch {
            streamer.stopStream()
        }
    }

    fun release() {
        streamer.releaseBlocking()
    }

    private fun alignDown16(value: Int): Int {
        return value - (value % 16)
    }

    private fun defaultBitrateFor(width: Int, height: Int, fps: Int): Int {
        return when {
            width >= 1920 || height >= 1080 -> if (fps > 30) 8_000_000 else 6_000_000
            width >= 1280 || height >= 720 -> if (fps > 30) 5_000_000 else 3_500_000
            else -> 2_000_000
        }
    }
}
```

The `alignDown16()` step is intentional. Your frame dumps showed clean 1920×1080 I420 before the encoder, but the encoder-facing full-frame input produced macroblock padding artifacts; the previous investigation concluded that 1080 caused a coded 1088 case and that 1920×1072 avoids the padding.  

For StreamPack’s Surface path this artifact may not reproduce, but keeping encoder dimensions aligned during the migration gives you a safer baseline.

---

# 7. Wire it through `AppGraph`

Your `AppGraph` already centralizes long-lived dependencies and exposes factory methods for coordinators. Add a second camera coordinator factory rather than changing `MainActivity` heavily. 

```kotlin
package org.fcast.android.sender

import android.content.Context
import org.fcast.android.sender.capture.CameraCaptureCoordinator
import org.fcast.android.sender.capture.StreamPackCameraCaptureCoordinator
import org.fcast.android.sender.capture.CaptureEngine
import org.fcast.android.sender.capture.ScreenCaptureCoordinator
import org.fcast.android.sender.data.AndroidSecretStore
import org.fcast.android.sender.data.SecretStore
import org.fcast.android.sender.runtime.JniRuntimeBridge
import org.fcast.android.sender.runtime.RuntimeBridge

class AppGraph(
    private val appContext: Context,
) {
    val runtime: RuntimeBridge by lazy {
        JniRuntimeBridge(appContext)
    }

    val secretStore: SecretStore by lazy {
        AndroidSecretStore(appContext)
    }

    fun newCaptureCoordinator(
        callbacks: ScreenCaptureCoordinator.CaptureCallbacks,
    ): ScreenCaptureCoordinator =
        org.fcast.android.sender.capture.RealScreenCaptureCoordinator(
            applicationContext = appContext,
            callbacks = callbacks,
            engineFactory = { CaptureEngine() },
        )

    fun newStreamPackCameraCoordinator(
        callbacks: CameraCaptureCoordinator.Callbacks,
    ): CameraCaptureCoordinator =
        StreamPackCameraCaptureCoordinator(
            applicationContext = appContext,
            callbacks = callbacks,
        )
}
```

Then in `MainActivity`, instantiate this instead of `RealCameraCaptureCoordinator` behind a feature flag.

```kotlin
private fun createCameraCoordinator(): CameraCaptureCoordinator {
    val useStreamPack = nativeUseStreamPackCameraPath()
    return if (useStreamPack) {
        (application as FcastApp).graph.newStreamPackCameraCoordinator(cameraCallbacks)
    } else {
        RealCameraCaptureCoordinator(
            applicationContext = applicationContext,
            callbacks = cameraCallbacks,
        )
    }
}
```

Add the native flag:

```kotlin
private external fun nativeUseStreamPackCameraPath(): Boolean
```

That lets your Rust/Slint config decide which path is active.

---

# 8. Keep `NativeActivity`; do not convert to AppCompat/ComponentActivity

The earlier standard StreamPack boilerplate approach uses a normal Kotlin Activity and StreamPack `PreviewView`. That is not the right primary shape here because your manifest uses `MainActivity` as a `NativeActivity` and points Android at the `fcastsender` native library. 

So keep:

```kotlin
class MainActivity : NativeActivity(), DisplayManager.DisplayListener
```

Your existing permission model is already compatible with `NativeActivity`: the current camera callbacks request permissions through legacy `requestPermissions(...)`, with a comment explaining that `NativeActivity` does not inherit from `ComponentActivity`. 

Keep that pattern.

Only route the permission result to the new coordinator:

```kotlin
override fun onRequestPermissionsResult(
    requestCode: Int,
    permissions: Array<out String>,
    grantResults: IntArray,
) {
    super.onRequestPermissionsResult(requestCode, permissions, grantResults)

    if (requestCode == REQ_CAMERA_PERM) {
        val granted = grantResults.isNotEmpty() &&
            grantResults.all { it == android.content.pm.PackageManager.PERMISSION_GRANTED }

        when (val c = cameraCoordinator) {
            is RealCameraCaptureCoordinator -> c.onPermissionResult(granted)
            is StreamPackCameraCaptureCoordinator -> c.onPermissionResult(granted)
        }
    }
}
```

---

# 9. Rust/Slint control flow

From Rust/Slint, do **not** manage StreamPack objects directly. Keep Rust as the state/control owner and call Android bridge functions through JNI.

Suggested Rust-facing commands:

```rust
pub enum CameraPipelineMode {
    LegacyRawGStreamer,
    StreamPackDirectSrt,
    StreamPackEncodedToGStreamer,
}

pub struct CameraStartConfig {
    pub camera_idx: i32,
    pub width: i32,
    pub height: i32,
    pub max_fps: i32,
    pub mirror: bool,
    pub stabilization: bool,
    pub zoom: f32,
    pub orientation_mode: OrientationMode,
    pub srt_url: String,
    pub mode: CameraPipelineMode,
}
```

Use JSON across JNI because your codebase already uses JSON for runtime/service configuration patterns such as `MigrationRuntimeServiceBridge.start(context, configJson)` and native status queries. 

Example Android static bridge:

```kotlin
package org.fcast.android.sender.stream

import android.content.Context
import org.fcast.android.sender.FcastApp
import org.fcast.android.sender.MainActivity
import org.json.JSONObject

object StreamPackNativeEntry {
    private val ctx: Context
        get() = FcastApp.appContext

    @JvmStatic
    fun startFromRust(configJson: String) {
        val json = JSONObject(configJson)

        val config = StreamPackStartConfig(
            cameraIdx = json.optInt("cameraIdx", 1),
            width = json.optInt("width", 1280),
            height = json.optInt("height", 720),
            maxFps = json.optInt("maxFps", 30),
            mirror = json.optBoolean("mirror", false),
            stabilization = json.optBoolean("stabilization", true),
            zoom = json.optDouble("zoom", 1.0).toFloat(),
            orientationMode = json.optString("orientationMode", "LANDSCAPE"),
            srtUrl = json.getString("srtUrl"),
        )

        MainActivity.runOnMainActivity { activity ->
            activity.startStreamPackCameraFromNative(config)
        }
    }

    @JvmStatic
    fun stopFromRust() {
        MainActivity.runOnMainActivity { activity ->
            activity.stopStreamPackCameraFromNative()
        }
    }
}

data class StreamPackStartConfig(
    val cameraIdx: Int,
    val width: Int,
    val height: Int,
    val maxFps: Int,
    val mirror: Boolean,
    val stabilization: Boolean,
    val zoom: Float,
    val orientationMode: String,
    val srtUrl: String,
)
```

You do not currently have `MainActivity.runOnMainActivity(...)` in the snippets, so add a weak-reference helper:

```kotlin
class MainActivity : NativeActivity(), DisplayManager.DisplayListener {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        current = java.lang.ref.WeakReference(this)

        // existing setup...
    }

    override fun onDestroy() {
        if (current?.get() === this) {
            current = null
        }
        super.onDestroy()
    }

    fun startStreamPackCameraFromNative(config: StreamPackStartConfig) {
        val cameraConfig = CameraCaptureConfig(
            cameraIdx = config.cameraIdx,
            width = config.width,
            height = config.height,
            maxFps = config.maxFps,
            mirror = config.mirror,
            stabilization = config.stabilization,
            zoom = config.zoom,
            orientationMode = when (config.orientationMode) {
                "PORTRAIT" -> OrientationMode.PORTRAIT
                "AUTO" -> OrientationMode.AUTO
                else -> OrientationMode.LANDSCAPE
            },
        )

        cameraCoordinator.startCapture(cameraConfig)
    }

    fun stopStreamPackCameraFromNative() {
        cameraCoordinator.stopCapture()
    }

    companion object {
        private var current: java.lang.ref.WeakReference<MainActivity>? = null

        @JvmStatic
        fun runOnMainActivity(block: (MainActivity) -> Unit) {
            val activity = current?.get() ?: return
            activity.runOnUiThread { block(activity) }
        }
    }
}
```

This keeps Slint/Rust in charge while letting Android own Android-only media APIs.

---

# 10. Phase 1: direct StreamPack SRT validation

Start with direct StreamPack SRT before building a custom encoded-frame endpoint.

Reason: the StreamPack boilerplate is explicitly an RTMP/SRT live streaming example and its README says to replace the default RTMP URL with an RTMP or SRT server URL in `MainViewModel.kt`. ([GitHub][2])

For this phase:

```text
Rust/Slint Start Camera
  → JNI
  → StreamPackNativeEntry.startFromRust(configJson)
  → StreamPackCameraCaptureCoordinator
  → StreamPackSenderBridge
  → StreamPack SRT
```

Do not involve GStreamer in the camera stream path yet.

Validation checklist:

```text
1. Slint UI still starts NativeActivity normally.
2. GStreamer.init(this) still succeeds for the rest of your app.
3. StreamPack camera starts at 1280x720@30.
4. Direct SRT receiver gets video and audio.
5. Start/stop works 20 times.
6. No green line at 1280x720.
7. Test 1920x1072.
8. Then test 1920x1080.
9. Compare CPU, thermals, and latency against legacy raw-I420 path.
```

---

# 11. Phase 2: bridge StreamPack encoded frames into Rust/GStreamer

Once direct StreamPack SRT is stable, stop using StreamPack transport and add a custom endpoint.

Target:

```text
StreamPack MediaCodec encoder
  → custom endpoint
  → JNI encoded frame callback
  → Rust
  → GStreamer compressed pipeline
```

Rust/GStreamer pipeline should become:

```text
appsrc name=video_h264_src is-live=true format=time do-timestamp=true
  caps=video/x-h264,stream-format=byte-stream,alignment=au
  ! queue max-size-buffers=8 max-size-time=0 max-size-bytes=0 leaky=downstream
  ! h264parse config-interval=-1
  ! mpegtsmux alignment=7
  ! srtsink uri="srt://host:port?mode=caller&latency=80"
```

That means your current `MainActivity.nativeProcessFrame(...)` raw-frame callback should no longer be used for camera streaming. Instead add:

```kotlin
external fun nativeProcessEncodedVideoFrame(
    streamId: Int,
    timestampNs: Long,
    flags: Int,
    data: ByteBuffer,
    size: Int,
)
```

Example endpoint skeleton:

```kotlin
package org.fcast.android.sender.stream

import io.github.thibaultbee.streampack.core.elements.data.FrameWithCloseable
import io.github.thibaultbee.streampack.core.elements.encoders.CodecConfig
import io.github.thibaultbee.streampack.core.elements.endpoints.IEndpoint
import io.github.thibaultbee.streampack.core.elements.endpoints.IEndpointInternal
import io.github.thibaultbee.streampack.core.configuration.mediadescriptor.MediaDescriptor
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

class RustGStreamerEndpoint : IEndpointInternal {
    private val openFlowInternal = MutableStateFlow(false)
    private val throwableFlowInternal = MutableStateFlow<Throwable?>(null)

    private var nextStreamPid = 256

    override val isOpenFlow: StateFlow<Boolean>
        get() = openFlowInternal.asStateFlow()

    override val throwableFlow: StateFlow<Throwable?>
        get() = throwableFlowInternal.asStateFlow()

    override val info: IEndpoint.IEndpointInfo
        get() = RustGStreamerEndpointInfo

    override val metrics: Any
        get() = "{}"

    override fun getInfo(type: MediaDescriptor.Type): IEndpoint.IEndpointInfo {
        return RustGStreamerEndpointInfo
    }

    override suspend fun open(descriptor: MediaDescriptor) {
        nativeOpenEncodedGStreamerPipeline()
        openFlowInternal.emit(true)
    }

    override suspend fun close() {
        nativeCloseEncodedGStreamerPipeline()
        openFlowInternal.emit(false)
    }

    override suspend fun startStream() {
        nativeStartEncodedGStreamerPipeline()
    }

    override suspend fun stopStream() {
        nativeStopEncodedGStreamerPipeline()
    }

    override suspend fun addStreams(streamConfigs: List<CodecConfig>): Map<CodecConfig, Int> {
        return streamConfigs.associateWith { addStream(it) }
    }

    override suspend fun addStream(streamConfig: CodecConfig): Int {
        val pid = nextStreamPid++
        nativeAddEncodedStream(
            pid = pid,
            mimeType = streamConfig.mimeType,
        )
        return pid
    }

    override suspend fun write(closeableFrame: FrameWithCloseable, streamPid: Int) {
        try {
            val frame = closeableFrame.frame

            nativeWriteEncodedFrame(
                streamPid = streamPid,
                timestampNs = frame.timestampInUs * 1000L,
                flags = frame.flags,
                buffer = frame.buffer,
                size = frame.buffer.remaining(),
            )
        } finally {
            closeableFrame.close()
        }
    }

    override suspend fun release() {
        stopStream()
        close()
    }

    private external fun nativeOpenEncodedGStreamerPipeline()
    private external fun nativeCloseEncodedGStreamerPipeline()
    private external fun nativeStartEncodedGStreamerPipeline()
    private external fun nativeStopEncodedGStreamerPipeline()

    private external fun nativeAddEncodedStream(
        pid: Int,
        mimeType: String,
    )

    private external fun nativeWriteEncodedFrame(
        streamPid: Int,
        timestampNs: Long,
        flags: Int,
        buffer: java.nio.ByteBuffer,
        size: Int,
    )
}
```

You may need to adjust field names like `timestampInUs`, `flags`, and `buffer` to the exact `Frame` API in the StreamPack version you pin. The endpoint shape is the important part.

---

# 12. Phase 3: replace the GL readback path

After Phase 2 works, deprecate this camera path:

```text
CameraCaptureEngine
  → Y framebuffer
  → U framebuffer
  → V framebuffer
  → glReadPixels
  → nativeProcessFrame
```

Keep `CameraCaptureEngine` only as:

```text
Legacy debug path
Regression path
Frame dump tool
Fallback for devices where StreamPack fails
```

Your code already has strong debugging infrastructure: codec benchmarking runs in a separate `:codec_bench` process and initializes GStreamer separately there, then kills the process after hardware decode to release codec/EGL state.  Keep that style for migration validation.

Suggested mode enum on the Rust side:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AndroidCameraPipeline {
    LegacyRawI420GStreamer,
    StreamPackDirectSrt,
    StreamPackEncodedToGStreamer,
}
```

---

# 13. Phase 4: Slint preview strategy

Because Slint owns your UI, avoid StreamPack `PreviewView` initially even though StreamPack supports it. The StreamPack README shows `PreviewView`, but that is for normal Android UI apps, not your `NativeActivity` + Slint stack. ([GitHub][1])

Use this progression:

```text
Phase 1:
  no embedded preview from StreamPack
  validate direct SRT output

Phase 2:
  keep existing Slint preview if it exists
  or use legacy preview path only when not streaming

Phase 3:
  add a custom SurfaceProcessor fanout:
    camera input surface
      → encoder surface
      → Slint/NativeActivity preview surface
```

StreamPack releases note that `SurfaceProcessor` became customizable and can process frames before they are sent to encoders. ([GitHub][3]) That is the right future integration point for an OBS-like scene renderer.

---

# 14. Phase 5: custom SurfaceProcessor for OBS-like scenes

Long-term, your architecture should become:

```text
Slint/Rust scene graph
  → JNI scene updates
  → Android custom SurfaceProcessor
  → GL compositor:
       camera texture
       screen texture
       image overlays
       text overlays
       crop / scale / rotate / mirror
  → MediaCodec encoder surface
  → encoded frames
  → Rust/GStreamer output
```

This replaces “convert camera frame to YUV and send to Rust” with “send scene commands to Android and let GPU render directly into the encoder surface.”

For OBS-like functionality, Rust/Slint should own:

```text
scene definitions
source positions
source visibility
transitions
stream state
record state
remote control state
```

Android should own:

```text
Camera2
MediaProjection
SurfaceTexture
EGL context
MediaCodec
StreamPack encoder lifecycle
```

GStreamer/Rust should own:

```text
SRT output
MPEG-TS muxing
recording variants
diagnostic pipelines
device capability reporting
fallback legacy path
```

---

# 15. What not to do

Do not do this:

```text
Slint UI
  → Kotlin StreamPack PreviewView
  → another Android UI tree
```

That duplicates UI responsibilities.

Do not do this either:

```text
StreamPack camera
  → raw frame extraction
  → Rust
  → GStreamer amcvidenc
```

That would lose the main performance benefit of StreamPack.

And do not immediately remove GStreamer. Your app already uses GStreamer initialization in `MainActivity`, GStreamer-native benchmarking in the codec benchmark process, and migration runtime services.   The goal is to move GStreamer **after** encoding for camera streaming, not to delete it.

---

# 16. Recommended implementation order

## Step 1 — add dependencies

Add `streampack-core` and `streampack-srt`.

## Step 2 — add `StreamPackSenderBridge`

Keep it isolated under:

```text
org.fcast.android.sender.stream
```

## Step 3 — add `StreamPackCameraCaptureCoordinator`

Implement the existing `CameraCaptureCoordinator` interface.

## Step 4 — add a Rust-controlled feature flag

Use:

```text
LegacyRawI420GStreamer
StreamPackDirectSrt
StreamPackEncodedToGStreamer
```

## Step 5 — validate direct SRT

Use `1280×720@30` first, then `1920×1072`, then `1920×1080`.

## Step 6 — create custom StreamPack endpoint

Route encoded frames to Rust/GStreamer instead of direct StreamPack SRT.

## Step 7 — convert Rust/GStreamer camera source node

Change camera input from:

```text
video/x-raw,format=I420
```

to:

```text
video/x-h264,stream-format=byte-stream,alignment=au
```

## Step 8 — retire `nativeProcessFrame` for camera streaming

Keep it for debug, dumps, and legacy fallback.

## Step 9 — build custom SurfaceProcessor

Use it for preview fanout, OBS-like composition, and encoder-size alignment.

---

# 17. Final architecture

The final architecture I recommend for your app is:

```text
Rust + Slint UI
  ↓ commands/config via JNI
MainActivity : NativeActivity
  ↓
StreamPack bridge
  ↓
Camera2 / MediaProjection
  ↓
custom SurfaceProcessor / GL compositor
  ↓
MediaCodec input Surface
  ↓
encoded H.264/AAC frames
  ↓
Rust/GStreamer compressed pipeline
  ↓
h264parse / aacparse / mpegtsmux / srtsink
```

This gives you:

```text
StreamPack-style performance:
  no raw I420 CPU readback in the camera hot path

Your existing architecture:
  NativeActivity
  Rust/Slint UI
  AppGraph/JniRuntimeBridge
  GStreamer runtime
  SRT/mux control from Rust

Migration safety:
  legacy raw path remains available
  direct StreamPack SRT can validate the encoder path
  custom endpoint can be added after validation
```

The key architectural rule is:

```text
Before encoding:
  Android Surface / GL / MediaCodec path

After encoding:
  Rust / GStreamer / SRT path
```

[1]: https://github.com/ThibaultBee/StreamPack "GitHub - ThibaultBee/StreamPack: Multiprotocol (SRT, RTMP and others) live streaming broadcaster libraries for Android · GitHub"
[2]: https://github.com/ThibaultBee/StreamPack-boilerplate "GitHub - ThibaultBee/StreamPack-boilerplate: Create your own Android RTMP/RTMPS/SRT live streaming application in less than 5 minutes! · GitHub"
[3]: https://github.com/ThibaultBee/StreamPack/releases "Releases · ThibaultBee/StreamPack · GitHub"
