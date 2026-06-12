package org.fcast.android.sender.stream

import android.Manifest
import android.content.Context
import android.media.AudioFormat
import android.media.MediaFormat
import android.util.Log
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

import io.github.thibaultbee.streampack.core.elements.sources.audio.audiorecord.MicrophoneSourceFactory
import io.github.thibaultbee.streampack.core.elements.sources.video.camera.extensions.defaultCameraId
import io.github.thibaultbee.streampack.core.interfaces.releaseBlocking
import io.github.thibaultbee.streampack.core.interfaces.setCameraId
import io.github.thibaultbee.streampack.core.interfaces.startStream
import io.github.thibaultbee.streampack.core.streamers.single.AudioConfig
import io.github.thibaultbee.streampack.core.streamers.single.SingleStreamer
import io.github.thibaultbee.streampack.core.streamers.single.VideoConfig

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
        val w = alignDown16(config.width)
        val h = alignDown16(config.height)
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
                    streamer.setVideoConfig(
                        VideoConfig(
                            mimeType = MediaFormat.MIMETYPE_VIDEO_AVC,
                            resolution = Size(w, h),
                            fps = config.maxFps,
                        )
                    )
                    streamer.setCameraId(cameraIdFor(config.cameraIdx))
                    streamer.setTargetRotation(rotationFor(config.orientationMode))

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

    private fun alignDown16(v: Int) = v - (v % 16)

    private fun defaultBitrate(w: Int, h: Int, fps: Int): Int = when {
        w >= 1920 || h >= 1080 -> if (fps > 30) 8_000_000 else 6_000_000
        w >= 1280 || h >= 720 -> if (fps > 30) 5_000_000 else 3_500_000
        else -> 2_000_000
    }

    companion object { private const val TAG = "StreamPackSenderBridge" }
}
