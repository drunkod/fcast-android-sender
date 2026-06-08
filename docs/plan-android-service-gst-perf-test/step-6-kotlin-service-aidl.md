# Step 6 — Kotlin service + AIDL (runs in `:codec_bench`)

← [Step 5](step-5-manifest-service.md) · [Index](README.md) · Next → [Step 7](step-7-kotlin-client-binding.md)

All new files under `app/src/main/java/org/fcast/android/sender/bench/` and AIDL
under `app/src/main/aidl/org/fcast/android/sender/bench/`.

### 6a — `NativeCodecBench.kt` (JNI shim; loads the libs in the service process)

Mirror the existing load order (`GstPopServiceBridge.java`):
**`gstreamer_android` first, then `fcastsender`.**

```kotlin
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
```

### 6b — AIDL: `ICodecBenchmarkService.aidl` + `ICodecBenchmarkCallback.aidl`

```aidl
// app/src/main/aidl/org/fcast/android/sender/bench/ICodecBenchmarkService.aidl
package org.fcast.android.sender.bench;

import org.fcast.android.sender.bench.ICodecBenchmarkCallback;

interface ICodecBenchmarkService {
    void runBenchmark(String requestJson, ICodecBenchmarkCallback callback);
    void cancel();
}
```

```aidl
// app/src/main/aidl/org/fcast/android/sender/bench/ICodecBenchmarkCallback.aidl
package org.fcast.android.sender.bench;

interface ICodecBenchmarkCallback {
    void onResult(String resultJson);
}
```

> Ensure AIDL is enabled in `app/build.gradle`:
> ```groovy
> android { buildFeatures { aidl true } }
> ```

### 6c — `CodecBenchmarkService.kt`

```kotlin
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
```

### ⚠️ Contingency — GStreamer in the service process

`nativeInit()` → `ensure_gstreamer_initialized()` → `gst::init()`. The `amc*`
factories register via `gstreamer_android`'s per-process `JNI_OnLoad` (each process
loads its own copy in 6a). **Validate in Step 10:** if the service's
`list_codec_factories` shows **0 amc** entries, GStreamer needs the service
`Context` at init. In that case add a JNI helper that calls
`gst_android_init(env, context)` (or the `org.freedesktop.gstreamer.GStreamer.init(Context)`
Java initializer that `gstreamer_android` ships) from `onCreate` **before**
`nativeInit()`, passing `this` (the Service Context).

---

← [Step 5](step-5-manifest-service.md) · [Index](README.md) · Next → [Step 7](step-7-kotlin-client-binding.md)
