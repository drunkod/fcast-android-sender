package org.fcast.android.sender.capture

/** Desired stream orientation, mirroring Moblin's `stream.portrait: Bool` but more expressive. */
enum class OrientationMode {
    /** Lock stream to portrait (9:16). Screen also locks to portrait. */
    PORTRAIT,
    /** Lock stream to landscape (16:9). Screen also locks to landscape. */
    LANDSCAPE,
    /** Follow device physical orientation. Screen rotation is unrestricted. */
    AUTO,
}

/**
 * Camera-specific capture parameters supplied by the Slint UI.
 *
 * Resolution and FPS are requested values; the engine resolves them against
 * CameraCharacteristics.SCALER_STREAM_CONFIGURATION_MAP and
 * CONTROL_AE_AVAILABLE_TARGET_FPS_RANGES.
 *
 * Best practice (android/camera-samples Camera2Basic): validate inputs early,
 * defer device-capability resolution to the engine.
 */
data class CameraCaptureConfig(
    /** 0 = front, 1 = back, 2 = external */
    val cameraIdx: Int,
    val width: Int,
    val height: Int,
    val maxFps: Int,
    val mirror: Boolean = false,
    val stabilization: Boolean = true,
    val zoom: Float = 1.0f,
    val orientationMode: OrientationMode = OrientationMode.PORTRAIT,
) {
    init {
        require(cameraIdx in 0..2)             { "cameraIdx must be 0..2, got $cameraIdx" }
        require(maxFps > 0)                    { "maxFps must be > 0, got $maxFps" }
        require(width > 0 && height > 0)       { "resolution must be positive ($width x $height)" }
        require(zoom in 0.5f..10.0f)           { "zoom must be in [0.5, 10.0], got $zoom" }
    }

    /** Minimum inter-frame interval for the GL throttle. */
    val minIntervalNanos: Long
        get() = 1_000_000_000L / maxFps
}
