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

class StreamPackCameraCaptureCoordinator(
    private val applicationContext: Context,
    private val callbacks: CameraCaptureCoordinator.Callbacks,
    private val bridge: StreamPackSenderBridge = StreamPackSenderBridge(applicationContext),
) : CameraCaptureCoordinator {

    private val mainHandler = Handler(Looper.getMainLooper())
    private var pendingConfig: CameraCaptureConfig? = null
    private var srtUrl: String = ""
    private var starting = false
    private var capturing = false
    private var sessionId = 0L

    @MainThread override fun attach() {}
    @MainThread override fun startPreview(config: CameraCaptureConfig, previewSurface: Surface) {}
    @MainThread override fun stopPreview() {}
    @MainThread override fun startCapture(config: CameraCaptureConfig) = startCapture(config, null)

    @MainThread
    override fun startCapture(config: CameraCaptureConfig, previewSurface: Surface?) {
        if (capturing) {
            Log.w(TAG, "startCapture while already capturing")
            return
        }

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

    @MainThread fun setSrtUrl(url: String) { srtUrl = url }

    @MainThread
    fun onPermissionResult(granted: Boolean) {
        val cfg = pendingConfig
        pendingConfig = null
        if (!granted) {
            callbacks.onCameraCaptureFailed("Camera/audio permission denied")
            return
        }
        if (cfg != null) startBridge(cfg)
    }

    @MainThread
    private fun startBridge(config: CameraCaptureConfig) {
        if (srtUrl.isBlank()) {
            callbacks.onCameraCaptureFailed("No SRT URL configured")
            return
        }
        if (starting || capturing) {
            Log.w(TAG, "startBridge while busy")
            return
        }
        val mySession = ++sessionId
        starting = true
        bridge.start(
            config = config,
            srtUrl = srtUrl,
            onStarted = { startedW, startedH ->
                mainHandler.post {
                    if (mySession != sessionId) return@post
                    starting = false
                    capturing = true
                    callbacks.onCameraCaptureStarted(startedW, startedH, initialRotation(config))
                }
            },
            onError = { reason ->
                mainHandler.post {
                    if (mySession != sessionId) return@post
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
        sessionId++
        starting = false
        capturing = false
        bridge.stop()
        callbacks.onCameraCaptureStopped()
    }

    @MainThread override fun shutdown() { stopCapture(); bridge.release() }
    override val isCapturing: Boolean @MainThread get() = capturing

    private fun initialRotation(config: CameraCaptureConfig): Int = when (config.orientationMode) {
        OrientationMode.PORTRAIT -> 0
        OrientationMode.LANDSCAPE -> 90
        OrientationMode.AUTO -> 0
    }

    companion object { private const val TAG = "StreamPackCameraCoord" }
}
