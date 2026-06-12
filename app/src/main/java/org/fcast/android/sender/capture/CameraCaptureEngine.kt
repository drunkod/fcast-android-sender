package org.fcast.android.sender.capture

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.SurfaceTexture
import android.hardware.camera2.*
import android.hardware.camera2.params.OutputConfiguration
import android.hardware.camera2.params.SessionConfiguration
import android.opengl.EGL14
import android.opengl.EGLContext
import android.opengl.EGLDisplay
import android.opengl.EGLSurface
import android.os.Handler
import android.os.HandlerThread
import android.util.Log
import android.util.Range
import android.util.Size
import android.view.Surface
import androidx.annotation.WorkerThread
import org.fcast.android.sender.MainActivity
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicInteger

/**
 * Camera capture engine — mirrors [CaptureEngine] but sources frames from
 * Camera2 instead of MediaProjection.
 *
 * Public methods (start/shutdown) are callable from the main thread.
 * Internals:
 *   - cameraThread / cameraHandler ........ Camera2 StateCallbacks
 *   - glThread / glHandler ................ EGL + shader pipeline (same as CaptureEngine)
 *
 * Frames delivered via MainActivity.nativeProcessFrame(w, h, timestampNs, Y, U, V).
 *
 */
@SuppressLint("MissingPermission", "NewApi")
open class CameraCaptureEngine {

    @Volatile private var running = false
    private val shouldCapture = AtomicBoolean(false)

    private val glThread = HandlerThread("CameraCaptureGL").also { it.start() }
    private val glHandler = Handler(glThread.looper)
    private val cameraThread = HandlerThread("CameraOps").also { it.start() }
    private val cameraHandler = Handler(cameraThread.looper)

    // Orientation: sensor + device rotation tracking
    private var orientationSensor: OrientationSensor? = null
    // Current rotation in degrees reported to Rust (0/90/180/270).
    private val currentRotationDeg = AtomicInteger(0)

    // Camera2 state
    private var cameraDevice: CameraDevice? = null
    private var captureSession: CameraCaptureSession? = null

    // GL state — same fields as CaptureEngine
    private var surfaceTexture: SurfaceTexture? = null
    private var cameraSurface: Surface? = null
    private var eglDisplay: EGLDisplay? = null
    private var eglContext: EGLContext? = null
    private var eglSurface: EGLSurface? = null
    private var yFramebuffer: CaptureEngine.Framebuffer? = null
    private var uFramebuffer: CaptureEngine.Framebuffer? = null
    private var vFramebuffer: CaptureEngine.Framebuffer? = null
    private var yProg: CaptureEngine.Program? = null
    private var uProg: CaptureEngine.Program? = null
    private var vProg: CaptureEngine.Program? = null
    private var vboId = 0
    private var oesTexId = 0

    private var lastFrameNanos = 0L
    private var minIntervalNanos = 0L
    private var frameCountDelivered = 0

    open fun start(
        context: Context,
        config: CameraCaptureConfig,
        previewSurface: Surface? = null,
        captureFrames: Boolean = true,
        onStarted: (width: Int, height: Int, rotationDeg: Int) -> Unit,
        onFatalError: (reason: String) -> Unit,
    ) {
        check(!running) { "CameraCaptureEngine.start called twice" }
        running = true
        minIntervalNanos = config.minIntervalNanos

        val sensor = OrientationSensor(context).also { orientationSensor = it }
        sensor.start()

        val cm = context.getSystemService(Context.CAMERA_SERVICE) as CameraManager

        val cameraId = resolveCameraId(cm, config.cameraIdx) ?: run {
            running = false; onFatalError("No camera found for idx=${config.cameraIdx}"); return
        }
        val chars = cm.getCameraCharacteristics(cameraId)
        val streamMap = chars.get(CameraCharacteristics.SCALER_STREAM_CONFIGURATION_MAP)
            ?: run { running = false; onFatalError("No stream configuration map"); return }

        val chosen = chooseBestSize(
            streamMap.getOutputSizes(SurfaceTexture::class.java),
            config.width, config.height,
        )
        val outDims = CaptureEngine.Dimensions(chosen.width, chosen.height)
        val uvDims  = CaptureEngine.Dimensions(chosen.width / 2, chosen.height / 2)

        val fpsRange = chooseBestFpsRange(
            chars.get(CameraCharacteristics.CONTROL_AE_AVAILABLE_TARGET_FPS_RANGES) ?: emptyArray(),
            config.maxFps,
        )
        val zoomRange = chars.get(CameraCharacteristics.CONTROL_ZOOM_RATIO_RANGE)

        // Compute the rotation that the GStreamer videoflip element must apply so
        // the encoded stream is upright. Mirrors Moblin's updateOrientation() in Model.swift.
        //
        // SENSOR_ORIENTATION: degrees the sensor image is rotated relative to the device's
        // natural orientation (typically 90° for back cameras on most phones).
        // deviceRotation: current physical rotation of the device (0/90/180/270).
        val sensorOrientation = chars.get(CameraCharacteristics.SENSOR_ORIENTATION) ?: 0
        val isFront = chars.get(CameraCharacteristics.LENS_FACING) == CameraCharacteristics.LENS_FACING_FRONT
        val deviceRotation = when (config.orientationMode) {
            OrientationMode.PORTRAIT  -> 0
            OrientationMode.LANDSCAPE -> 90
            OrientationMode.AUTO      -> sensor.deviceRotation.value
        }
        val rotationDeg = calcVideoRotation(sensorOrientation, isFront, deviceRotation)
        currentRotationDeg.set(rotationDeg)

        val stateCallback = object : CameraDevice.StateCallback() {
            override fun onOpened(camera: CameraDevice) {
                cameraDevice = camera
                glHandler.post {
                    try {
                        if (captureFrames) {
                            initGlAndCreateSession(
                                camera, outDims, uvDims, config, rotationDeg, fpsRange, zoomRange,
                                previewSurface, onStarted, onFatalError,
                            )
                        } else {
                            initPreviewOnlySession(
                                camera, outDims, config, fpsRange, zoomRange,
                                previewSurface, onStarted, onFatalError,
                            )
                        }
                    } catch (t: Throwable) {
                        Log.e(TAG, "GL init after onOpened failed", t)
                        running = false
                        onFatalError(t.message ?: "GL init failed")
                    }
                }
            }
            override fun onDisconnected(camera: CameraDevice) {
                Log.w(TAG, "Camera disconnected"); camera.close(); cameraDevice = null
            }
            override fun onError(camera: CameraDevice, error: Int) {
                Log.e(TAG, "Camera error: $error"); camera.close(); cameraDevice = null
                running = false; onFatalError("Camera error code=$error")
            }
        }

        try {
            cameraManagerOpen(cm, cameraId, stateCallback)
        } catch (e: SecurityException) {
            running = false; onFatalError("CAMERA permission not granted")
        } catch (e: CameraAccessException) {
            running = false; onFatalError("Camera access: ${e.message}")
        }
    }

    private fun cameraManagerOpen(
        cm: CameraManager, id: String, cb: CameraDevice.StateCallback,
    ) = cm.openCamera(id, cb, cameraHandler)

    private fun resolveCameraId(cm: CameraManager, idx: Int): String? {
        val want = when (idx) {
            0 -> CameraCharacteristics.LENS_FACING_FRONT
            1 -> CameraCharacteristics.LENS_FACING_BACK
            2 -> CameraCharacteristics.LENS_FACING_EXTERNAL
            else -> CameraCharacteristics.LENS_FACING_BACK
        }
        return cm.cameraIdList.firstOrNull { id ->
            cm.getCameraCharacteristics(id).get(CameraCharacteristics.LENS_FACING) == want
        } ?: cm.cameraIdList.firstOrNull()
    }

    private fun chooseBestSize(sizes: Array<Size>, w: Int, h: Int): Size {
        val targetAspect = w.toFloat() / h
        // Largest size that fits within target and matches aspect ratio closely.
        return sizes
            .filter { it.width <= w && it.height <= h }
            .minByOrNull { Math.abs(it.width.toFloat() / it.height - targetAspect) +
                           (w * h - it.width * it.height) / (w.toFloat() * h) * 0.5f }
            ?: sizes.minByOrNull { it.width * it.height }
            ?: Size(w, h)
    }

    private fun chooseBestFpsRange(ranges: Array<Range<Int>>, target: Int): Range<Int>? =
        ranges.minByOrNull { kotlin.math.abs(it.upper - target) }

    @WorkerThread
    private fun initGlAndCreateSession(
        camera: CameraDevice,
        outDims: CaptureEngine.Dimensions, uvDims: CaptureEngine.Dimensions,
        config: CameraCaptureConfig,
        rotationDeg: Int,
        fpsRange: Range<Int>?, zoomRange: Range<Float>?,
        previewSurface: Surface?,
        onStarted: (Int, Int, Int) -> Unit, onFatalError: (String) -> Unit,
    ) {
        // 1. Set up EGL + shaders — delegate to CaptureEngine helpers.
        //    Mirror is applied here via the VBO geometry (see setupGlForCapture).
        //    Downstream pipelines (Phase 3 CameraSourceNode) must NOT insert an
        //    additional `videoflip` — frames already arrive pre-flipped.
        val gl = CaptureEngine.setupGlForCapture(outDims, uvDims, mirror = config.mirror)
        eglDisplay = gl.display; eglContext = gl.context; eglSurface = gl.surface
        oesTexId = gl.oesTexId; vboId = gl.vboId
        yFramebuffer = gl.yFb; uFramebuffer = gl.uFb; vFramebuffer = gl.vFb
        yProg = gl.yProg; uProg = gl.uProg; vProg = gl.vProg

        // 2. SurfaceTexture wired to the GL OES texture.
        val st = SurfaceTexture(oesTexId).also { surfaceTexture = it }
        st.setDefaultBufferSize(outDims.width, outDims.height)
        st.setOnFrameAvailableListener({ onFrameAvailable() }, glHandler)
        val surf = Surface(st).also { cameraSurface = it }

        // Release the GL context on this thread before camera-thread session creation;
        // we re-acquire it inside pumpOneFrame.
        EGL14.eglMakeCurrent(eglDisplay ?: EGL14.EGL_NO_DISPLAY, EGL14.EGL_NO_SURFACE,
                             EGL14.EGL_NO_SURFACE, EGL14.EGL_NO_CONTEXT)

        // 3. Create capture session via the modern SessionConfiguration API.
        val targetSurfaces = buildList {
            add(surf)
            previewSurface?.takeIf { it.isValid }?.let { add(it) }
        }
        val outputs = targetSurfaces.map { OutputConfiguration(it) }
        val sessionCfg = SessionConfiguration(
            SessionConfiguration.SESSION_REGULAR,
            outputs,
            { it.run() },
            object : CameraCaptureSession.StateCallback() {
                override fun onConfigured(session: CameraCaptureSession) {
                    captureSession = session
                    val req = camera.createCaptureRequest(CameraDevice.TEMPLATE_PREVIEW).apply {
                        targetSurfaces.forEach { addTarget(it) }
                        if (config.stabilization) {
                            set(CaptureRequest.CONTROL_VIDEO_STABILIZATION_MODE,
                                CaptureRequest.CONTROL_VIDEO_STABILIZATION_MODE_ON)
                        }
                        fpsRange?.let { set(CaptureRequest.CONTROL_AE_TARGET_FPS_RANGE, it) }
                        if (android.os.Build.VERSION.SDK_INT >= 30 && zoomRange != null) {
                            set(CaptureRequest.CONTROL_ZOOM_RATIO,
                                config.zoom.coerceIn(zoomRange.lower, zoomRange.upper))
                        }
                    }
                    try {
                        session.setRepeatingRequest(req.build(), null, cameraHandler)
                        shouldCapture.set(true)
                        onStarted(outDims.width, outDims.height, rotationDeg)
                    } catch (e: CameraAccessException) {
                        onFatalError("setRepeatingRequest failed: ${e.message}")
                    }
                }
                override fun onConfigureFailed(session: CameraCaptureSession) {
                    onFatalError("Camera session configuration failed")
                }
            },
        )
        camera.createCaptureSession(sessionCfg)
    }

    @WorkerThread
    private fun initPreviewOnlySession(
        camera: CameraDevice,
        outDims: CaptureEngine.Dimensions,
        config: CameraCaptureConfig,
        fpsRange: Range<Int>?, zoomRange: Range<Float>?,
        previewSurface: Surface?,
        onStarted: (Int, Int, Int) -> Unit, onFatalError: (String) -> Unit,
    ) {
        val preview = previewSurface?.takeIf { it.isValid }
            ?: run {
                onFatalError("Camera preview surface is not ready")
                return
            }
        val sessionCfg = SessionConfiguration(
            SessionConfiguration.SESSION_REGULAR,
            listOf(OutputConfiguration(preview)),
            { it.run() },
            object : CameraCaptureSession.StateCallback() {
                override fun onConfigured(session: CameraCaptureSession) {
                    captureSession = session
                    val req = camera.createCaptureRequest(CameraDevice.TEMPLATE_PREVIEW).apply {
                        addTarget(preview)
                        if (config.stabilization) {
                            set(
                                CaptureRequest.CONTROL_VIDEO_STABILIZATION_MODE,
                                CaptureRequest.CONTROL_VIDEO_STABILIZATION_MODE_ON,
                            )
                        }
                        fpsRange?.let { set(CaptureRequest.CONTROL_AE_TARGET_FPS_RANGE, it) }
                        if (android.os.Build.VERSION.SDK_INT >= 30 && zoomRange != null) {
                            set(
                                CaptureRequest.CONTROL_ZOOM_RATIO,
                                config.zoom.coerceIn(zoomRange.lower, zoomRange.upper),
                            )
                        }
                    }
                    try {
                        session.setRepeatingRequest(req.build(), null, cameraHandler)
                        onStarted(outDims.width, outDims.height, 0)
                    } catch (e: CameraAccessException) {
                        onFatalError("setRepeatingRequest failed: ${e.message}")
                    }
                }

                override fun onConfigureFailed(session: CameraCaptureSession) {
                    onFatalError("Camera preview session configuration failed")
                }
            },
        )
        camera.createCaptureSession(sessionCfg)
    }

    @WorkerThread
    private fun onFrameAvailable() {
        if (!running || !shouldCapture.get()) return
        val now = System.nanoTime()
        val deliver = now - lastFrameNanos >= minIntervalNanos
        try {
            pumpOneFrame(deliver)
            if (deliver) {
                lastFrameNanos = now
                frameCountDelivered++
                if (frameCountDelivered <= 10 || frameCountDelivered % 30 == 0) {
                    Log.d(TAG, "onFrameAvailable: frame #$frameCountDelivered")
                }
            }
        } catch (e: RuntimeException) {
            Log.e(TAG, "pumpOneFrame failed", e)
        }
    }

    @WorkerThread
    private fun pumpOneFrame(deliver: Boolean) {
        val st = surfaceTexture ?: return
        val yFb = yFramebuffer ?: return
        val uFb = uFramebuffer ?: return
        val vFb = vFramebuffer ?: return
        val yP = yProg ?: return; val uP = uProg ?: return; val vP = vProg ?: return

        val disp = eglDisplay ?: return
        val surf = eglSurface ?: return
        val ctx = eglContext ?: return

        if (!EGL14.eglMakeCurrent(disp, surf, surf, ctx)) {
            throw RuntimeException("EGL make current failed: ${EGL14.eglGetError()}")
        }
        st.updateTexImage()
        val timestampNs = st.timestamp
        if (!deliver) {
            EGL14.eglMakeCurrent(disp, EGL14.EGL_NO_SURFACE,
                                 EGL14.EGL_NO_SURFACE, EGL14.EGL_NO_CONTEXT)
            return
        }
        val tex = FloatArray(16); st.getTransformMatrix(tex)

        CaptureEngine.renderToFb(oesTexId, yFb, yP, tex, vboId)
        CaptureEngine.renderToFb(oesTexId, uFb, uP, tex, vboId)
        CaptureEngine.renderToFb(oesTexId, vFb, vP, tex, vboId)
        yFb.readPixels(); uFb.readPixels(); vFb.readPixels()

        EGL14.eglMakeCurrent(disp, EGL14.EGL_NO_SURFACE,
                             EGL14.EGL_NO_SURFACE, EGL14.EGL_NO_CONTEXT)

        MainActivity.nativeProcessFrame(
            yFb.dims.width, yFb.dims.height,
            timestampNs,
            yFb.buf, uFb.buf, vFb.buf,
        )
    }

    /**
     * Computes the rotation (degrees) the GStreamer videoflip element must apply so
     * the encoded frame is upright. Equivalent to Moblin's AVCaptureVideoOrientation logic.
     *
     * @param sensorOrientation SENSOR_ORIENTATION from CameraCharacteristics (degrees)
     * @param isFront           true for front-facing camera (mirror flips rotation axis)
     * @param deviceRotation    current physical device rotation: 0/90/180/270
     */
    private fun calcVideoRotation(sensorOrientation: Int, isFront: Boolean, deviceRotation: Int): Int =
        if (isFront) (sensorOrientation + deviceRotation) % 360
        else         (sensorOrientation - deviceRotation + 360) % 360

    open fun shutdown() {
        orientationSensor?.stop()
        orientationSensor = null
        if (!running) return
        running = false
        shouldCapture.set(false)

        // Ensure both teardown tasks run to completion before we tear down the
        // loopers — otherwise quitSafely() can race with a frame still being
        // processed on the GL thread and leak GL resources on slow devices.
        val done = CountDownLatch(2)

        cameraHandler.post {
            try { captureSession?.close() } catch (_: Exception) {}
            captureSession = null
            try { cameraDevice?.close() } catch (_: Exception) {}
            cameraDevice = null
            done.countDown()
        }
        glHandler.post {
            try {
                cameraSurface?.release(); cameraSurface = null
                surfaceTexture?.release(); surfaceTexture = null
                CaptureEngine.releaseGl(
                    eglDisplay ?: EGL14.EGL_NO_DISPLAY,
                    eglContext ?: EGL14.EGL_NO_CONTEXT,
                    eglSurface ?: EGL14.EGL_NO_SURFACE,
                    oesTexId, vboId,
                    listOf(yFramebuffer, uFramebuffer, vFramebuffer),
                    listOf(yProg, uProg, vProg),
                )
            } catch (t: Throwable) {
                Log.w(TAG, "GL shutdown raced with a frame", t)
            } finally {
                done.countDown()
            }
        }

        try {
            if (!done.await(2, TimeUnit.SECONDS)) {
                Log.w(TAG, "shutdown teardown tasks did not complete within 2s")
            }
        } catch (_: InterruptedException) {
            Thread.currentThread().interrupt()
        }

        cameraThread.quitSafely(); glThread.quitSafely()
        try { cameraThread.join(1000L) } catch (_: InterruptedException) {}
        try { glThread.join(1000L) } catch (_: InterruptedException) {}
    }

    companion object { private const val TAG = "CameraCaptureEngine" }
}
