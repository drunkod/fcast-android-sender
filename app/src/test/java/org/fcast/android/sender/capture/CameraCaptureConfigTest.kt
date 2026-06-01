package org.fcast.android.sender.capture

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Test

class CameraCaptureConfigTest {

    @Test
    fun validParameters_constructsSuccessfully() {
        val config = CameraCaptureConfig(
            cameraIdx = 1,
            width = 1920,
            height = 1080,
            maxFps = 30,
            mirror = true,
            stabilization = false,
            zoom = 2.0f
        )
        assertEquals(1, config.cameraIdx)
        assertEquals(1920, config.width)
        assertEquals(1080, config.height)
        assertEquals(30, config.maxFps)
        assertTrue(config.mirror)
        assertFalse(config.stabilization)
        assertEquals(2.0f, config.zoom, 0.001f)
        assertEquals(33_333_333L, config.minIntervalNanos)
    }

    @Test
    fun invalidCameraIdx_throwsException() {
        try {
            CameraCaptureConfig(cameraIdx = -1, width = 1920, height = 1080, maxFps = 30)
            fail("Expected IllegalArgumentException for negative cameraIdx")
        } catch (e: IllegalArgumentException) {
            assertTrue(e.message?.contains("cameraIdx") == true)
        }

        try {
            CameraCaptureConfig(cameraIdx = 3, width = 1920, height = 1080, maxFps = 30)
            fail("Expected IllegalArgumentException for cameraIdx > 2")
        } catch (e: IllegalArgumentException) {
            assertTrue(e.message?.contains("cameraIdx") == true)
        }
    }

    @Test
    fun invalidMaxFps_throwsException() {
        try {
            CameraCaptureConfig(cameraIdx = 1, width = 1920, height = 1080, maxFps = 0)
            fail("Expected IllegalArgumentException for maxFps = 0")
        } catch (e: IllegalArgumentException) {
            assertTrue(e.message?.contains("maxFps") == true)
        }
    }

    @Test
    fun invalidResolution_throwsException() {
        try {
            CameraCaptureConfig(cameraIdx = 1, width = 0, height = 1080, maxFps = 30)
            fail("Expected IllegalArgumentException for width = 0")
        } catch (e: IllegalArgumentException) {
            assertTrue(e.message?.contains("resolution") == true)
        }

        try {
            CameraCaptureConfig(cameraIdx = 1, width = 1920, height = -5, maxFps = 30)
            fail("Expected IllegalArgumentException for height < 0")
        } catch (e: IllegalArgumentException) {
            assertTrue(e.message?.contains("resolution") == true)
        }
    }

    @Test
    fun invalidZoom_throwsException() {
        try {
            CameraCaptureConfig(cameraIdx = 1, width = 1920, height = 1080, maxFps = 30, zoom = 0.4f)
            fail("Expected IllegalArgumentException for zoom < 0.5")
        } catch (e: IllegalArgumentException) {
            assertTrue(e.message?.contains("zoom") == true)
        }

        try {
            CameraCaptureConfig(cameraIdx = 1, width = 1920, height = 1080, maxFps = 30, zoom = 10.1f)
            fail("Expected IllegalArgumentException for zoom > 10.0")
        } catch (e: IllegalArgumentException) {
            assertTrue(e.message?.contains("zoom") == true)
        }
    }
}
