package org.fcast.android.sender.stream

import android.Manifest
import android.content.Context
import android.media.AudioFormat
import android.media.MediaFormat
import android.util.Log
import android.util.Range
import android.util.Size
import android.view.Surface
import androidx.annotation.RequiresPermission
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import org.fcast.android.sender.capture.CameraCaptureConfig
import org.fcast.android.sender.capture.OrientationMode

import io.github.thibaultbee.streampack.core.configuration.BitrateRegulatorConfig
import io.github.thibaultbee.streampack.core.elements.sources.audio.audiorecord.MicrophoneSourceFactory
import io.github.thibaultbee.streampack.core.elements.sources.video.camera.extensions.defaultCameraId
import io.github.thibaultbee.streampack.core.interfaces.releaseBlocking
import io.github.thibaultbee.streampack.core.interfaces.setCameraId
import io.github.thibaultbee.streampack.core.interfaces.startStream
import io.github.thibaultbee.streampack.core.streamers.single.AudioConfig
import io.github.thibaultbee.streampack.core.streamers.single.SingleStreamer
import io.github.thibaultbee.streampack.core.streamers.single.VideoConfig
// Adaptive bitrate (SRT-only). Packages/signatures verified against the 3.1.2 artifact:
//   ext.srt.regulator.DefaultSrtBitrateRegulator.Factory()
//   ext.srt.regulator.controllers.DefaultSrtBitrateRegulatorController.Factory(
//       SrtBitrateRegulator.Factory, BitrateRegulatorConfig, delayMs: Long)
//   SingleStreamer.addBitrateRegulatorController(IBitrateRegulatorController.Factory)
import io.github.thibaultbee.streampack.ext.srt.regulator.DefaultSrtBitrateRegulator
import io.github.thibaultbee.streampack.ext.srt.regulator.controllers.DefaultSrtBitrateRegulatorController

class StreamPackSenderBridge(context: Context) {
    private val appContext = context.applicationContext
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    private val streamer: SingleStreamer = newStreamer(appContext)

    private enum class State { IDLE, STARTING, STREAMING, STOPPING, RELEASED }
    @Volatile private var state = State.IDLE
    private val mutex = Mutex()

    private fun newStreamer(ctx: Context): SingleStreamer =
        SingleStreamer(context = ctx, withAudio = true, withVideo = true)

    @RequiresPermission(allOf = [Manifest.permission.CAMERA, Manifest.permission.RECORD_AUDIO])
    fun start(
        config: CameraCaptureConfig,
        srtUrl: String,
        onStarted: (width: Int, height: Int) -> Unit,
        onError: (String) -> Unit,
    ) {
        val w = config.width
        val h = config.height
        scope.launch {
            mutex.withLock {
                if (state != State.IDLE) {
                    onError("StreamPack busy (state=$state)")
                    return@withLock
                }
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
                    val maxVideoBitrate = defaultBitrate(w, h, config.maxFps)
                    val minVideoBitrate = (maxVideoBitrate / 4).coerceAtLeast(MIN_VIDEO_BITRATE_FLOOR)
                    streamer.setVideoConfig(
                        VideoConfig(
                            mimeType = MediaFormat.MIMETYPE_VIDEO_AVC,
                            resolution = Size(w, h),
                            fps = config.maxFps,
                            // Initial/ceiling bitrate; the SRT adaptive-bitrate regulator
                            // (attached below) moves the live video bitrate within
                            // [minVideoBitrate, maxVideoBitrate] based on network conditions.
                            bitrate = maxVideoBitrate,
                        )
                    )
                    streamer.setCameraId(cameraIdFor(config.cameraIdx))
                    streamer.setTargetRotation(rotationFor(config.orientationMode))

                    // Network-adaptive bitrate (SRT-only). Must be attached BEFORE
                    // startStream so the regulator hooks the encoder + SRT endpoint.
                    // Clear any controller left from a previous start/stop cycle first
                    // (only one regulator may be active at a time).
                    runCatching { streamer.removeBitrateRegulatorController() }
                    streamer.addBitrateRegulatorController(
                        DefaultSrtBitrateRegulatorController.Factory(
                            DefaultSrtBitrateRegulator.Factory(),
                            BitrateRegulatorConfig(
                                Range(minVideoBitrate, maxVideoBitrate),         // video ABR range
                                Range(AUDIO_BITRATE, AUDIO_BITRATE),             // audio held constant
                            ),
                            ABR_UPDATE_INTERVAL_MS,                              // regulator tick (ms)
                        )
                    )

                    if (state != State.STARTING) {
                        runCatching { streamer.stopStream() }
                        return@withLock
                    }

                    streamer.startStream(srtUrl)

                    if (state != State.STARTING) {
                        runCatching { streamer.stopStream() }
                        state = State.IDLE
                        return@withLock
                    }

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
                // Detach the adaptive-bitrate regulator so its coroutine stops and the
                // next start() attaches a fresh one (only one may be active at a time).
                runCatching { streamer.removeBitrateRegulatorController() }
                state = State.IDLE
            }
        }
    }

    fun release() {
        state = State.RELEASED
        scope.launch { mutex.withLock { runCatching { streamer.releaseBlocking() } } }
    }

    private fun cameraIdFor(idx: Int): String = appContext.defaultCameraId

    private fun rotationFor(mode: OrientationMode): Int = when (mode) {
        OrientationMode.PORTRAIT -> Surface.ROTATION_0
        OrientationMode.LANDSCAPE -> Surface.ROTATION_90
        OrientationMode.AUTO -> Surface.ROTATION_0
    }

    // Currently bypassed: start() uses the raw requested width/height to validate true
    // 1920x1080 on-device. The 1088-padding green-edge artifact was specific to the legacy
    // amcvidenc-via-appsrc path; StreamPack's Surface→MediaCodec path appears unaffected on
    // the test device. Re-enable (and verify across devices) before shipping non-test builds.
    @Suppress("unused")
    private fun alignDown16(v: Int) = v - (v % 16)

    // Per-resolution ceiling for the adaptive bitrate range (also the encoder's initial
    // bitrate). The regulator floors at [maxVideoBitrate / 4, MIN_VIDEO_BITRATE_FLOOR].
    private fun defaultBitrate(w: Int, h: Int, fps: Int): Int = when {
        w >= 1920 || h >= 1080 -> if (fps > 30) 8_000_000 else 6_000_000
        w >= 1280 || h >= 720 -> if (fps > 30) 5_000_000 else 3_500_000
        else -> 2_000_000
    }

    companion object {
        private const val TAG = "StreamPackSenderBridge"
        private const val AUDIO_BITRATE = 128_000        // AAC, held constant by the regulator
        private const val MIN_VIDEO_BITRATE_FLOOR = 500_000
        private const val ABR_UPDATE_INTERVAL_MS = 500L  // regulator update cadence (StreamPack default)
    }
}
