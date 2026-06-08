package org.fcast.android.sender.bench

import android.app.Service
import android.content.Intent
import android.os.IBinder
import android.os.Process
import java.util.concurrent.Executors

class CodecBenchmarkService : Service() {
    private val executor = Executors.newSingleThreadExecutor()

    override fun onCreate() {
        super.onCreate()
        // Runs in the :codec_bench process. Initialises GStreamer for THIS process.
        NativeCodecBench.nativeInit()
    }

    override fun onBind(intent: Intent): IBinder = binder

    override fun onDestroy() {
        NativeCodecBench.nativeCancel()
        executor.shutdownNow()
        super.onDestroy()
    }

    private val binder = object : ICodecBenchmarkService.Stub() {
        override fun runBenchmark(requestJson: String, callback: ICodecBenchmarkCallback) {
            executor.execute {
                val resultJson = try {
                    NativeCodecBench.nativeRunBenchmarkPlanJson(requestJson)
                } catch (t: Throwable) {
                    """{"ok":false,"report":"","ranDecode":false,"shouldKillProcess":false,"error":"${t.javaClass.name}: ${t.message ?: ""}"}"""
                }
                try {
                    callback.onResult(resultJson)
                } finally {
                    // After HW decode, kill this process to release all codec + EGL state.
                    if (resultJson.contains("\"shouldKillProcess\":true")) {
                        stopSelf()
                        Process.killProcess(Process.myPid())
                    }
                }
            }
        }

        override fun cancel() {
            NativeCodecBench.nativeCancel()
        }
    }
}
