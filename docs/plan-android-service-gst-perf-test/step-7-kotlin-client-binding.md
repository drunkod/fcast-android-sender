# Step 7 — Client binding (main process) + Rust upcall

← [Step 6](step-6-kotlin-service-aidl.md) · [Index](README.md) · Next → [Step 8](step-8-android-main-rewire.md)

The main (UI) process binds the `:codec_bench` service, runs the plan, and feeds
the JSON result back to Rust via the `nativeCodecBenchmarkResult` downcall (Step 4c).
Rust starts it with a single static upcall — mirrors the camera upcall pattern.

### 7a — Store an app `Context` (so the static client can bind without an Activity)

`FcastApp.kt` (the existing `android:name=".FcastApp"` Application):

```kotlin
package org.fcast.android.sender

import android.app.Application
import android.content.Context

class FcastApp : Application() {
    override fun onCreate() {
        super.onCreate()
        appContext = applicationContext
    }
    companion object {
        @JvmStatic lateinit var appContext: Context
            private set
    }
}
```

### 7b — `CodecBenchmarkClient.kt` (static entry the Rust upcall calls)

```kotlin
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
                            // Service self-kills after decode; just unbind here.
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
```

### 7c — `MainActivity` — declare the result downcall as `external`/native

Add to `MainActivity.kt` (companion or top-level `@JvmStatic`), matching the Rust
export `Java_org_fcast_android_sender_MainActivity_nativeCodecBenchmarkResult`:

```kotlin
companion object {
    @JvmStatic external fun nativeCodecBenchmarkResult(resultJson: String)
}
```

### 7d — Rust upcall (main process) — add to `src/codec_bench_jni.rs`

```rust
#[cfg(target_os = "android")]
pub fn request_codec_benchmark(request: &crate::codec_bench_plan::CodecBenchRequest) -> Result<(), String> {
    use crate::jni_bridge::helpers::{load_app_class, vm};

    let json = serde_json::to_string(request).map_err(|e| format!("serialize request: {e}"))?;

    let vm = vm();
    let mut env = vm.attach_current_thread().map_err(|e| format!("attach: {e}"))?;
    let class = load_app_class(&mut env, "org/fcast/android/sender/bench/CodecBenchmarkClient")
        .map_err(|e| format!("load CodecBenchmarkClient: {e}"))?;
    let arg = env.new_string(&json).map_err(|e| format!("new_string: {e}"))?;
    env.call_static_method(class, "start", "(Ljava/lang/String;)V", &[(&arg).into()])
        .map_err(|e| format!("call start: {e}"))?;
    Ok(())
}
```

> `load_app_class` uses the cached app ClassLoader (set in `android_main` at startup),
> so this works from the worker thread — same mechanism `src/secret/jni.rs` uses.

---

← [Step 6](step-6-kotlin-service-aidl.md) · [Index](README.md) · Next → [Step 8](step-8-android-main-rewire.md)
