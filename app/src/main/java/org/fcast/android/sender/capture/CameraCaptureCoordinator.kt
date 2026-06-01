package org.fcast.android.sender.capture

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.os.Handler
import android.os.Looper
import android.util.Log
import androidx.annotation.MainThread
import androidx.core.content.ContextCompat

interface CameraCaptureCoordinator {
    @MainThread fun attach()
    @MainThread fun startCapture(config: CameraCaptureConfig)
    @MainThread fun stopCapture()
    @MainThread fun shutdown()
    val isCapturing: Boolean

    interface Callbacks {
        @MainThread fun onCameraPermissionNeeded()
        @MainThread fun onCameraCaptureStarted(width: Int, height: Int)
        @MainThread fun onCameraCaptureStopped()
        @MainThread fun onCameraCaptureFailed(reason: String)
    }
}

class RealCameraCaptureCoordinator(
    private val applicationContext: Context,
    private val callbacks: CameraCaptureCoordinator.Callbacks,
    private val engineFactory: () -> CameraCaptureEngine = { CameraCaptureEngine() },
) : CameraCaptureCoordinator {

    private val mainHandler = Handler(Looper.getMainLooper())
    private var engine: CameraCaptureEngine? = null
    private var pendingConfig: CameraCaptureConfig? = null

    @MainThread override fun attach() { /* nothing to subscribe to */ }

    @MainThread
    override fun startCapture(config: CameraCaptureConfig) {
        if (engine != null) {
            Log.w(TAG, "startCapture called while already capturing")
            return
        }
        if (ContextCompat.checkSelfPermission(applicationContext, Manifest.permission.CAMERA)
            != PackageManager.PERMISSION_GRANTED
        ) {
            pendingConfig = config
            callbacks.onCameraPermissionNeeded()
            return
        }
        startEngine(config)
    }

    /** Called by Activity after the user grants/denies CAMERA. */
    @MainThread
    fun onPermissionResult(granted: Boolean) {
        val cfg = pendingConfig
        pendingConfig = null
        if (granted && cfg != null) startEngine(cfg)
        else callbacks.onCameraCaptureFailed("Camera permission denied")
    }

    @MainThread
    override fun stopCapture() {
        val e = engine ?: return
        engine = null
        e.shutdown()
        callbacks.onCameraCaptureStopped()
    }

    @MainThread override fun shutdown() { stopCapture() }
    override val isCapturing: Boolean @MainThread get() = engine != null

    @MainThread
    private fun startEngine(cfg: CameraCaptureConfig) {
        val e = engineFactory().also { engine = it }
        try {
            e.start(
                context = applicationContext,
                config = cfg,
                onStarted = { w, h -> mainHandler.post { callbacks.onCameraCaptureStarted(w, h) } },
                onFatalError = { reason ->
                    mainHandler.post {
                        stopCapture()
                        callbacks.onCameraCaptureFailed(reason)
                    }
                },
            )
        } catch (t: Throwable) {
            Log.e(TAG, "engine.start failed", t)
            stopCapture()
            callbacks.onCameraCaptureFailed(t.message ?: "engine.start failed")
        }
    }

    companion object { private const val TAG = "CameraCaptureCoordinator" }
}
