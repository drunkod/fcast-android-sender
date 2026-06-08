package org.fcast.android.sender.bench

import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.ServiceConnection
import android.os.IBinder
import org.fcast.android.sender.FcastApp
import org.fcast.android.sender.MainActivity

object CodecBenchmarkClient {

    // Rust → Kotlin upcall entry. Binds :codec_bench, runs the plan, returns the
    // result to Rust via MainActivity.nativeCodecBenchmarkResult(...).
    @JvmStatic
    fun start(requestJson: String) {
        val ctx: Context = FcastApp.appContext
        val intent = Intent(ctx, CodecBenchmarkService::class.java)

        val conn = object : ServiceConnection {
            override fun onServiceConnected(name: ComponentName, binder: IBinder) {
                val svc = ICodecBenchmarkService.Stub.asInterface(binder)
                val self = this
                try {
                    svc.runBenchmark(requestJson, object : ICodecBenchmarkCallback.Stub() {
                        override fun onResult(resultJson: String) {
                            MainActivity.nativeCodecBenchmarkResult(resultJson)
                            try { ctx.unbindService(self) } catch (_: Throwable) {}
                        }
                    })
                } catch (t: Throwable) {
                    MainActivity.nativeCodecBenchmarkResult(
                        """{"ok":false,"report":"","ranDecode":false,"shouldKillProcess":false,"error":"bind/run failed: ${t.message}"}"""
                    )
                    try { ctx.unbindService(self) } catch (_: Throwable) {}
                }
            }

            override fun onServiceDisconnected(name: ComponentName) {}
        }

        ctx.bindService(intent, conn, Context.BIND_AUTO_CREATE)
    }
}
