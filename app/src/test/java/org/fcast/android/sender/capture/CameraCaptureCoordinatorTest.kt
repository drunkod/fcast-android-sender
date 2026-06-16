package org.fcast.android.sender.capture

import android.Manifest
import android.app.Application
import android.content.Context
import android.os.Looper
import androidx.test.core.app.ApplicationProvider
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config
import org.robolectric.shadows.ShadowApplication

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = android.app.Application::class)
class CameraCaptureCoordinatorTest {

    private lateinit var context: Context
    private lateinit var shadowApp: ShadowApplication
    private lateinit var fakeEngine: FakeCameraCaptureEngine
    private lateinit var callbacks: FakeCallbacks
    private lateinit var coordinator: RealCameraCaptureCoordinator

    private val config = CameraCaptureConfig(
        cameraIdx = 1,
        width = 640,
        height = 480,
        maxFps = 30
    )

    class FakeCameraCaptureEngine : CameraCaptureEngine() {
        var started = false
        var shutdownCalled = false
        var startConfig: CameraCaptureConfig? = null
        var onStartedCallback: ((Int, Int, Int) -> Unit)? = null
        var onFatalErrorCallback: ((String) -> Unit)? = null

        override fun start(
            context: Context,
            config: CameraCaptureConfig,
            previewSurface: android.view.Surface?,
            captureFrames: Boolean,
            onStarted: (Int, Int, Int) -> Unit,
            onFatalError: (String) -> Unit
        ) {
            started = true
            startConfig = config
            onStartedCallback = onStarted
            onFatalErrorCallback = onFatalError
            onStarted(config.width, config.height, 0)
        }

        override fun shutdown() {
            shutdownCalled = true
        }
    }

    class FakeCallbacks : CameraCaptureCoordinator.Callbacks {
        var permissionNeededCalled = false
        var startedWidth = 0
        var startedHeight = 0
        var startedRotationDeg = 0
        var stoppedCalled = false
        var failedReason: String? = null

        override fun onCameraPermissionNeeded() {
            permissionNeededCalled = true
        }

        override fun onCameraCaptureStarted(width: Int, height: Int, rotationDeg: Int) {
            startedWidth = width
            startedHeight = height
            startedRotationDeg = rotationDeg
        }

        override fun onCameraCaptureStopped() {
            stoppedCalled = true
        }

        override fun onCameraCaptureFailed(reason: String) {
            failedReason = reason
        }
    }

    @Before
    fun setUp() {
        val app = ApplicationProvider.getApplicationContext<Application>()
        context = app
        shadowApp = shadowOf(app)
        fakeEngine = FakeCameraCaptureEngine()
        callbacks = FakeCallbacks()
        coordinator = RealCameraCaptureCoordinator(
            applicationContext = context,
            callbacks = callbacks,
            engineFactory = { fakeEngine }
        )
    }

    @Test
    fun startCapture_whenPermissionGranted_startsEngine() {
        shadowApp.grantPermissions(Manifest.permission.CAMERA)

        assertFalse(coordinator.isCapturing)
        coordinator.startCapture(config)
        
        // Idle the main looper to execute callbacks posted to the main thread handler
        shadowOf(Looper.getMainLooper()).idle()

        assertTrue(coordinator.isCapturing)
        assertTrue(fakeEngine.started)
        assertEquals(config, fakeEngine.startConfig)
        assertEquals(640, callbacks.startedWidth)
        assertEquals(480, callbacks.startedHeight)
    }

    @Test
    fun startCapture_whenPermissionDenied_requestsPermission() {
        shadowApp.denyPermissions(Manifest.permission.CAMERA)

        coordinator.startCapture(config)

        assertFalse(coordinator.isCapturing)
        assertFalse(fakeEngine.started)
        assertTrue(callbacks.permissionNeededCalled)
    }

    @Test
    fun onPermissionResult_granted_startsEngine() {
        shadowApp.denyPermissions(Manifest.permission.CAMERA)
        coordinator.startCapture(config)

        assertFalse(coordinator.isCapturing)

        // Grant permission and trigger callback
        shadowApp.grantPermissions(Manifest.permission.CAMERA)
        coordinator.onPermissionResult(true)
        
        // Idle the main looper to execute callbacks posted to the main thread handler
        shadowOf(Looper.getMainLooper()).idle()

        assertTrue(coordinator.isCapturing)
        assertTrue(fakeEngine.started)
        assertEquals(640, callbacks.startedWidth)
    }

    @Test
    fun onPermissionResult_denied_triggersFailure() {
        shadowApp.denyPermissions(Manifest.permission.CAMERA)
        coordinator.startCapture(config)

        coordinator.onPermissionResult(false)

        assertFalse(coordinator.isCapturing)
        assertFalse(fakeEngine.started)
        assertEquals("Camera permission denied", callbacks.failedReason)
    }

    @Test
    fun stopCapture_stopsEngine() {
        shadowApp.grantPermissions(Manifest.permission.CAMERA)
        coordinator.startCapture(config)

        // Idle the main looper to execute callbacks posted to the main thread handler
        shadowOf(Looper.getMainLooper()).idle()

        assertTrue(coordinator.isCapturing)
        coordinator.stopCapture()

        assertFalse(coordinator.isCapturing)
        assertTrue(fakeEngine.shutdownCalled)
        assertTrue(callbacks.stoppedCalled)
    }
}
