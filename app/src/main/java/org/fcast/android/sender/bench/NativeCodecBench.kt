package org.fcast.android.sender.bench

object NativeCodecBench {
    init {
        System.loadLibrary("gstreamer_android")
        System.loadLibrary("fcastsender")
    }

    external fun nativeInit()
    external fun nativeRunBenchmarkPlanJson(requestJson: String): String
    external fun nativeCancel()
}
