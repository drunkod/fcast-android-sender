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

class RealCameraCaptureCoordinator(
    private val applicationContext: Context,
    private val callbacks: CameraCaptureCoordinator.Callbacks,
    private val engineFactory: () -> CameraCaptureEngine = { CameraCaptureEngine() },
) : CameraCaptureCoordinator {

    private val mainHandler = Handler(Looper.getMainLooper())
    private var engine: CameraCaptureEngine? = null
    private var pendingConfig: CameraCaptureConfig? = null
    private var pendingPreview: Pair<CameraCaptureConfig, Surface>? = null
    private var mode: Mode? = null

    @MainThread override fun attach() { /* nothing to subscribe to */ }

    @MainThread
    override fun startPreview(config: CameraCaptureConfig, previewSurface: Surface) {
        if (mode == Mode.CAPTURE) return
        if (mode == Mode.PREVIEW) {
            stopPreview()
        }
        if (ContextCompat.checkSelfPermission(applicationContext, Manifest.permission.CAMERA)
            != PackageManager.PERMISSION_GRANTED
        ) {
            pendingPreview = config to previewSurface
            callbacks.onCameraPermissionNeeded()
            return
        }
        startEngine(config, Mode.PREVIEW, previewSurface)
    }

    @MainThread
    override fun stopPreview() {
        if (mode != Mode.PREVIEW) return
        pendingPreview = null
        val e = engine ?: return
        engine = null
        mode = null
        e.shutdown()
    }

    @MainThread
    override fun startCapture(config: CameraCaptureConfig) = startCapture(config, null)

    @MainThread
    override fun startCapture(config: CameraCaptureConfig, previewSurface: Surface?) {
        if (mode == Mode.PREVIEW) {
            stopPreview()
        }
        if (engine != null && mode == Mode.CAPTURE) {
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
        startEngine(config, Mode.CAPTURE, previewSurface)
    }

    /** Called by Activity after the user grants/denies CAMERA. */
    @MainThread
    fun onPermissionResult(granted: Boolean) {
        val cfg = pendingConfig
        pendingConfig = null
        val preview = pendingPreview
        pendingPreview = null
        when {
            granted && cfg != null -> startEngine(cfg, Mode.CAPTURE, null)
            granted && preview != null -> startEngine(preview.first, Mode.PREVIEW, preview.second)
            !granted -> callbacks.onCameraCaptureFailed("Camera permission denied")
            // granted && cfg == null: permission granted proactively (no pending capture to resume)
        }
    }

    @MainThread
    override fun stopCapture() {
        if (mode != Mode.CAPTURE) return
        val e = engine ?: return
        engine = null
        mode = null
        e.shutdown()
        callbacks.onCameraCaptureStopped()
    }

    @MainThread override fun shutdown() {
        stopCapture()
        stopPreview()
    }
    override val isCapturing: Boolean @MainThread get() = mode == Mode.CAPTURE

    @MainThread
    private fun startEngine(cfg: CameraCaptureConfig, nextMode: Mode, previewSurface: Surface?) {
        val e = engineFactory().also {
            engine = it
            mode = nextMode
        }
        try {
            e.start(
                context = applicationContext,
                config = cfg,
                previewSurface = previewSurface,
                captureFrames = nextMode == Mode.CAPTURE,
                onStarted = { w, h, rotDeg ->
                    if (nextMode == Mode.CAPTURE) {
                        mainHandler.post { callbacks.onCameraCaptureStarted(w, h, rotDeg) }
                    } else {
                        Log.d(TAG, "camera preview started ${w}x$h rot=${rotDeg}°")
                    }
                },
                onFatalError = { reason ->
                    mainHandler.post {
                        if (nextMode == Mode.CAPTURE) {
                            stopCapture()
                            callbacks.onCameraCaptureFailed(reason)
                        } else {
                            stopPreview()
                            Log.w(TAG, "camera preview failed: $reason")
                        }
                    }
                },
            )
        } catch (t: Throwable) {
            Log.e(TAG, "engine.start failed", t)
            if (nextMode == Mode.CAPTURE) {
                stopCapture()
                callbacks.onCameraCaptureFailed(t.message ?: "engine.start failed")
            } else {
                stopPreview()
            }
        }
    }

    private enum class Mode { PREVIEW, CAPTURE }

    companion object { private const val TAG = "CameraCaptureCoordinator" }
}
