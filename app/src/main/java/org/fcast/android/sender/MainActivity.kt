package org.fcast.android.sender

import android.app.NativeActivity
import android.content.Context
import android.content.Intent
import android.content.res.Configuration
import android.hardware.display.DisplayManager
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.util.Log
import android.view.Gravity
import android.view.KeyEvent
import android.view.Surface
import android.view.SurfaceHolder
import android.view.SurfaceView
import android.view.View
import android.widget.FrameLayout
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch
import org.fcast.android.sender.capture.CaptureConfig
import org.fcast.android.sender.capture.ScreenCaptureCoordinator
import org.fcast.android.sender.capture.CameraCaptureConfig
import org.fcast.android.sender.capture.CameraCaptureCoordinator
import org.fcast.android.sender.capture.RealCameraCaptureCoordinator
import org.fcast.android.sender.discovery.Discoverer
import org.fcast.android.sender.qr.QrScannerLauncher
import org.fcast.android.sender.runtime.BackendKind
import org.fcast.android.sender.shell.SenderController
import org.fcast.android.sender.shell.UiState
import org.freedesktop.gstreamer.GStreamer
import java.nio.ByteBuffer

@Suppress("GestureBackNavigation", "DEPRECATION")
class MainActivity : NativeActivity(), DisplayManager.DisplayListener {

    private lateinit var displayManager: DisplayManager
    private lateinit var coordinator: ScreenCaptureCoordinator
    private lateinit var cameraCoordinator: RealCameraCaptureCoordinator
    private lateinit var qr: QrScannerLauncher
    private lateinit var controller: SenderController

    private val activityScope = MainScope()
    private var cameraPreviewView: SurfaceView? = null
    private var cameraPreviewSurface: Surface? = null
    private var cameraPreviewConfig: CameraCaptureConfig? = null
    private var cameraPreviewRequested = false

    private val cameraCallbacks = object : CameraCaptureCoordinator.Callbacks {
        override fun onCameraPermissionNeeded() {
            // NativeActivity does not inherit from ComponentActivity, so we use
            // the legacy requestPermissions + onRequestPermissionsResult path
            // (same approach the screen-capture flow uses for its consent flow).
            requestPermissions(
                arrayOf(android.Manifest.permission.CAMERA, android.Manifest.permission.RECORD_AUDIO),
                REQ_CAMERA_PERM
            )
        }
        override fun onCameraCaptureStarted(width: Int, height: Int) {
            nativeCameraCaptureStarted(width, height)
        }
        override fun onCameraCaptureStopped() {
            nativeCameraCaptureStopped()
        }
        override fun onCameraCaptureFailed(reason: String) {
            nativeCameraCaptureFailed(reason)
        }
    }

    private val captureCallbacks = object : ScreenCaptureCoordinator.CaptureCallbacks {
        @Suppress("DEPRECATION")
        override fun onPermissionRequested(intent: Intent) {
            startActivityForResult(intent, REQ_PROJECTION)
        }
        override fun onCaptureStarted(width: Int, height: Int) {
            // TODO(step-10): once Slint→Kotlin wires backend start through the controller,
            // uiState will be Connected here and the MIGRATION fallback will stop firing.
            val kind = (controller.uiState.value as? UiState.Connected)?.kind ?: BackendKind.MIGRATION
            controller.onCaptureStartedFromCoordinator(kind, width, height)
            nativeCaptureStarted()
        }
        override fun onCaptureStopped() {
            controller.onCaptureStoppedFromCoordinator()
            nativeCaptureStopped()
        }
        override fun onCaptureCancelled(reason: String) {
            controller.onCaptureStoppedFromCoordinator()
            nativeCaptureCancelled()
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        try {
            GStreamer.init(this)
        } catch (e: Exception) {
            Log.e(TAG, "Failed to init GStreamer $e")
            finish()
        }

        window.setBackgroundDrawable(android.graphics.drawable.ColorDrawable(android.graphics.Color.TRANSPARENT))
        window.setFormat(android.graphics.PixelFormat.TRANSLUCENT)

        Discoverer(this)

        coordinator = (application as FcastApp).graph.newCaptureCoordinator(captureCallbacks)
        coordinator.attach()

        cameraCoordinator = RealCameraCaptureCoordinator(applicationContext, cameraCallbacks)
        cameraCoordinator.attach()

        qr = org.fcast.android.sender.qr.RealQrScannerLauncher(this)
        controller = SenderController((application as FcastApp).graph.runtime, coordinator, qr)

        activityScope.launch {
            controller.uiState.collect { onUiStateChanged(it) }
        }

        displayManager = getSystemService(Context.DISPLAY_SERVICE) as DisplayManager
        displayManager.registerDisplayListener(this, Handler(mainLooper))

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            if (checkSelfPermission(android.Manifest.permission.POST_NOTIFICATIONS) != android.content.pm.PackageManager.PERMISSION_GRANTED) {
                requestPermissions(arrayOf(android.Manifest.permission.POST_NOTIFICATIONS), 101)
            }
        }

        if (checkSelfPermission(android.Manifest.permission.CAMERA) != android.content.pm.PackageManager.PERMISSION_GRANTED) {
            requestPermissions(arrayOf(android.Manifest.permission.CAMERA, android.Manifest.permission.RECORD_AUDIO), REQ_CAMERA_PERM)
        } else {
            cameraCoordinator.onPermissionResult(true)
            startDefaultCameraPreview()
        }
    }

    override fun onDestroy() {
        destroyCameraPreview()
        controller.shutdown()
        coordinator.shutdown()
        cameraCoordinator.shutdown()
        activityScope.cancel()
        runCatching { displayManager.unregisterDisplayListener(this) }
        super.onDestroy()
    }

    override fun onStop() {
        // onStop intentionally left out — capture must survive background.
        // See refactor step 03.4 for context.
        super.onStop()
    }

    @Suppress("OVERRIDE_DEPRECATION")
    override fun onBackPressed() {
        Log.d(TAG, "onBackPressed")
        nativeBackPressed()
    }

    override fun dispatchKeyEvent(event: KeyEvent): Boolean {
        if (event.keyCode == KeyEvent.KEYCODE_BACK && event.action == KeyEvent.ACTION_UP) {
            Log.d(TAG, "dispatchKeyEvent ACTION_UP KEYCODE_BACK")
            nativeBackPressed()
            return true
        }
        return super.dispatchKeyEvent(event)
    }

    override fun onKeyDown(keyCode: Int, event: KeyEvent): Boolean {
        if (keyCode == KeyEvent.KEYCODE_BACK) {
            Log.d(TAG, "onKeyDown KEYCODE_BACK")
            return true
        }
        return super.onKeyDown(keyCode, event)
    }

    override fun onKeyUp(keyCode: Int, event: KeyEvent): Boolean {
        if (keyCode == KeyEvent.KEYCODE_BACK) {
            Log.d(TAG, "onKeyUp KEYCODE_BACK")
            nativeBackPressed()
            return true
        }
        return super.onKeyUp(keyCode, event)
    }

    override fun onDisplayAdded(displayId: Int) {}
    override fun onDisplayRemoved(displayId: Int) {}
    override fun onDisplayChanged(displayId: Int) {}

    override fun onConfigurationChanged(newConfig: Configuration) {
        super.onConfigurationChanged(newConfig)
    }

    @Deprecated(
        "NativeActivity uses the legacy requestPermissions API; ActivityResultLauncher " +
            "is only available on ComponentActivity. Same pattern as REQ_PROJECTION.",
    )
    @Suppress("DEPRECATION")
    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray,
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        if (requestCode == REQ_CAMERA_PERM) {
            val cameraGranted = checkSelfPermission(android.Manifest.permission.CAMERA) == android.content.pm.PackageManager.PERMISSION_GRANTED
            val audioGranted = checkSelfPermission(android.Manifest.permission.RECORD_AUDIO) == android.content.pm.PackageManager.PERMISSION_GRANTED
            val granted = cameraGranted && audioGranted
            cameraCoordinator.onPermissionResult(cameraGranted)
            nativeCameraPermissionResult(granted)
            if (cameraGranted) {
                startDefaultCameraPreview()
            }
        }
    }

    @Deprecated("Use ActivityResultLauncher; see QrScannerLauncher.")
    @Suppress("DEPRECATION")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        when (requestCode) {
            REQ_PROJECTION -> {
                if (resultCode == RESULT_OK) {
                    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                        val serviceIntent = Intent(this, ScreenCaptureService::class.java).apply {
                            action = ScreenCaptureService.ACTION_RESULT
                            putExtra("resultCode", resultCode)
                            putExtra("data", data)
                        }
                        Log.d(TAG, "Starting foreground service SDK=${Build.VERSION.SDK_INT}")
                        runCatching { startForegroundService(serviceIntent) }
                            .onFailure { Log.e(TAG, "Failed to start foreground service: $it") }
                    } else {
                        Log.d(TAG, "Starting capture")
                        CaptureResultBus.deliver(resultCode, data ?: Intent())
                    }
                } else if (resultCode == RESULT_CANCELED) {
                    Log.d(TAG, "Media projection Canceled")
                    CaptureResultBus.deliver(resultCode, Intent())
                }
            }
            QrScannerLauncher.REQUEST_CODE -> {
                if (resultCode == RESULT_OK) {
                    val result = data?.getStringExtra("SCAN_RESULT")
                    nativeQrScanResult(result ?: "")
                }
            }
        }
    }

    // Called from native code
    private fun startScreenCapture(scaleWidth: Int, scaleHeight: Int, maxFramerate: Int) {
        Log.d(TAG, "Requesting screen capture permissions: scaleWidth=$scaleWidth scaleHeight=$scaleHeight maxFramerate=$maxFramerate")
        coordinator.startCapture(CaptureConfig(scaleWidth, scaleHeight, maxFramerate))
    }

    // Called from native code
    private fun stopCapture() {
        coordinator.stopCapture()
    }

    // Called from native code
    private fun scanQr() {
        controller.scanQr()
    }

    // Called from native code
    private fun finishApp() {
        runOnUiThread { finish() }
    }

    private fun onUiStateChanged(state: UiState) {
        // AppState ordinals: 0=Disconnected, 1=Connecting, 3=WaitingForMedia, 4=Casting.
        // BannerSeverity ordinals: 0=info, 1=success, 2=warning, 3=error.
        // Error maps to Disconnected + non-empty error banner so Slint can surface it.
        // TODO(step-12): banner writes here can race with Application::flash_banner's
        // auto-hide timer. Coordinate via the generation counter in Application.
        val (slintState, banner, severity) = when (state) {
            is UiState.Disconnected -> Triple(0, "", 0)
            is UiState.Starting     -> Triple(1, "", 0)
            is UiState.Connected    -> Triple(3, state.message ?: "", 0)
            is UiState.Casting      -> Triple(4, "", 0)
            is UiState.Error        -> Triple(0, state.message, 3)
        }
        nativeSlintApplyState(slintState, banner, severity)
    }

    // Called from Rust via JNI
    @Suppress("unused")
    private fun startCameraCapture(
        cameraIdx: Int, width: Int, height: Int, fps: Int,
        mirror: Boolean, stabilization: Boolean, zoom: Float,
    ) {
        runOnUiThread {
            cameraCoordinator.startCapture(
                CameraCaptureConfig(cameraIdx, width, height, fps, mirror, stabilization, zoom),
                cameraPreviewSurface?.takeIf { it.isValid },
            )
        }
    }

    // Called from Rust via JNI
    @Suppress("unused")
    private fun stopCameraCapture() {
        runOnUiThread {
            cameraCoordinator.stopCapture()
            maybeStartCameraPreview()
        }
    }

    // Called from Rust via JNI
    @Suppress("unused")
    private fun startCameraPreview(
        cameraIdx: Int, width: Int, height: Int, fps: Int,
        mirror: Boolean, stabilization: Boolean, zoom: Float,
    ) {
        runOnUiThread {
            cameraPreviewRequested = true
            cameraPreviewConfig = CameraCaptureConfig(cameraIdx, width, height, fps, mirror, stabilization, zoom)
            ensureCameraPreviewView().visibility = View.VISIBLE
            maybeStartCameraPreview()
        }
    }

    // Called from Rust via JNI
    @Suppress("unused")
    private fun stopCameraPreview() {
        runOnUiThread {
            cameraPreviewRequested = false
            if (!cameraCoordinator.isCapturing) {
                cameraCoordinator.stopPreview()
                cameraPreviewView?.visibility = View.GONE
            } else {
                cameraPreviewView?.visibility = View.INVISIBLE
            }
            if (controller.uiState.value is UiState.Disconnected) {
                startDefaultCameraPreview()
            }
        }
    }

    // Called from Rust via JNI
    @Suppress("unused")
    private fun probeCameraPermission(): Boolean {
        val cameraGranted = checkSelfPermission(android.Manifest.permission.CAMERA) ==
            android.content.pm.PackageManager.PERMISSION_GRANTED
        val audioGranted = checkSelfPermission(android.Manifest.permission.RECORD_AUDIO) ==
            android.content.pm.PackageManager.PERMISSION_GRANTED
        return cameraGranted && audioGranted
    }

    // Called from Rust via JNI. Wraps requestPermissions so the request code
    // (REQ_CAMERA_PERM) stays defined in one place and onRequestPermissionsResult
    // continues to receive it.
    @Suppress("unused")
    private fun requestCameraPermission() {
        requestPermissions(
            arrayOf(android.Manifest.permission.CAMERA, android.Manifest.permission.RECORD_AUDIO),
            REQ_CAMERA_PERM
        )
    }

    private fun ensureCameraPreviewView(): SurfaceView {
        cameraPreviewView?.let { return it }
        val view = SurfaceView(this)
        // Default z-order: SurfaceView surface sits BELOW the window surface.
        // Combined with PixelFormat.TRANSLUCENT on the window, transparent
        // Slint pixels let the camera show through underneath.
        view.holder.addCallback(object : SurfaceHolder.Callback {
            override fun surfaceCreated(holder: SurfaceHolder) {
                cameraPreviewSurface?.release()
                cameraPreviewSurface = holder.surface
                maybeStartCameraPreview()
            }

            override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {}

            override fun surfaceDestroyed(holder: SurfaceHolder) {
                if (!cameraCoordinator.isCapturing) {
                    cameraCoordinator.stopPreview()
                }
                cameraPreviewSurface = null
            }
        })
        val params = FrameLayout.LayoutParams(
            FrameLayout.LayoutParams.MATCH_PARENT,
            FrameLayout.LayoutParams.MATCH_PARENT,
            Gravity.CENTER
        )
        addContentView(view, params)
        view.visibility = View.VISIBLE
        cameraPreviewView = view
        return view
    }

    private fun startDefaultCameraPreview() {
        if (cameraPreviewRequested) return
        startCameraPreview(0, 1920, 1080, 30, false, false, 1.0f) // 0 = back camera
    }

    private fun maybeStartCameraPreview() {
        if (!cameraPreviewRequested || cameraCoordinator.isCapturing) return
        val config = cameraPreviewConfig ?: return
        val surface = cameraPreviewSurface?.takeIf { it.isValid } ?: return
        cameraCoordinator.startPreview(config, surface)
    }

    private fun destroyCameraPreview() {
        cameraPreviewRequested = false
        cameraCoordinator.stopPreview()
        cameraPreviewSurface?.release()
        cameraPreviewSurface = null
        cameraPreviewView = null
    }

    private fun dp(value: Int): Int =
        (value * resources.displayMetrics.density).toInt()

    // ── JNI symbol shims ─────────────────────────────────────────────────
    // Names match Rust's Java_org_fcast_android_sender_MainActivity_native*.
    external fun nativeBackPressed()
    external fun nativeCaptureStarted()
    external fun nativeCaptureStopped()
    external fun nativeCaptureCancelled()
    external fun nativeCameraCaptureStarted(width: Int, height: Int)
    external fun nativeCameraCaptureStopped()
    external fun nativeCameraCaptureFailed(reason: String)
    external fun nativeCameraPermissionResult(granted: Boolean)
    external fun nativeQrScanResult(result: String)
    external fun nativeSlintApplyState(state: Int, banner: String, severity: Int)

    companion object {
        init {
            System.loadLibrary("gstreamer_android")
            System.loadLibrary("fcastsender")
        }

        private const val TAG = "MainActivity"
        const val REQ_PROJECTION = 1
        const val REQ_CAMERA_PERM = 1002
        const val ACTION_MEDIA_PROJECTION_STARTED = "org.fcast.android.sender.MEDIA_PROJECTION_STARTED"

        @JvmStatic
        external fun nativeProcessFrame(width: Int, height: Int, timestampNs: Long, bufferY: ByteBuffer, bufferU: ByteBuffer, bufferV: ByteBuffer)

        @JvmStatic
        external fun nativeGraphCommand(payloadJson: String): String
    }

}
