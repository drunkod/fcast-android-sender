# Step 02 — `StreamPackSenderBridge`

**Master plan:** §5.2, §6 · **Phase:** 1 · **Depends on:** step-01 · **Lang:** Kotlin

## Goal

Create the **only** class in the app that imports StreamPack types. It owns a
`SingleStreamer`, configures camera+mic+encoder, and drives direct SRT. All lifecycle
mutations are serialized behind a `Mutex` over an explicit state machine so a
`stop()`/`release()` arriving mid-start can't corrupt the streamer.

## Files touched

- **New:** `app/src/main/java/org/fcast/android/sender/stream/StreamPackSenderBridge.kt`

## Full code

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
    //     sessionId (step-03) prevents a late onStarted from changing UI state after stop.
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
     *        (e.g. 1080 → 1072), so callers must report THESE, not the request.
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
     * via CameraManager).
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

## StreamPack 3.1.1 API used (verified from the boilerplate)

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
```

## How to verify

```
✅ File compiles against streampack-core 3.1.1 (no unresolved imports).
✅ No other file in the app imports io.github.thibaultbee.* (encapsulation holds).
✅ Static review: every streamer.* call is inside mutex.withLock { } except the
   release() RELEASED gate write.
```

## Risks (carried from master §15)

- **`SingleStreamer` construction:** confirm the exact constructor/factory in
  `draft/StreamPack-boilerplate/.../MainViewModelFactory.kt`; if it differs, change only
  `newStreamer`.
- **`alignDown16` changes resolution** (1080→1072) — that's why `onStarted` returns the
  actual `(w, h)`; the coordinator (step-03) forwards those, never the request.
- **`release()` is async** — `shutdown()` does not guarantee teardown before the next
  statement. Add `fun release(): Job` + `releaseBlockingForTests()` before any caller
  relies on immediate teardown.
