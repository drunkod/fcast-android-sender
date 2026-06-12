package org.fcast.android.sender.capture

import android.content.Context
import android.view.OrientationEventListener
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/**
 * Wraps [OrientationEventListener] and exposes the device's physical rotation
 * snapped to the nearest 90° step as a [StateFlow].
 *
 * Equivalent to Moblin's `UIDevice.orientationDidChangeNotification` +
 * `getOrientation()` combo (UiUtils.swift:18 / Model.swift:1618).
 *
 * Values: 0 = natural portrait, 90 = 90° clockwise, 180 = inverted, 270 = 270° clockwise.
 */
class OrientationSensor(context: Context) {

    private val _deviceRotation = MutableStateFlow(0)
    val deviceRotation: StateFlow<Int> = _deviceRotation.asStateFlow()

    private val listener = object : OrientationEventListener(context) {
        override fun onOrientationChanged(degrees: Int) {
            if (degrees == ORIENTATION_UNKNOWN) return
            val snapped = when {
                degrees in 315..360 || degrees in 0..44   -> 0
                degrees in 45..134                        -> 90
                degrees in 135..224                       -> 180
                else                                      -> 270
            }
            _deviceRotation.value = snapped
        }
    }

    fun start() { listener.enable() }
    fun stop()  { listener.disable() }
}
