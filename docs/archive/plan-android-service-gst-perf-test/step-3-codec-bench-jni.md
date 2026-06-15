# Step 3 — `src/codec_bench_jni.rs` (new): JNI exports for the service

← [Step 2](step-2-codec-bench-plan.md) · [Index](README.md) · Next → [Step 4](step-4-lib-register-result-downcall.md)

These run **inside the `:codec_bench` process**, called by the Kotlin service
(Step 6). Names match the package `org.fcast.android.sender.bench.NativeCodecBench`,
and the style matches the existing exports in `src/lib.rs`
(`#[unsafe(no_mangle)] extern "C" … <'local>`).

```rust
//! JNI exports for the :codec_bench service process. The Kotlin shell calls these;
//! all real benchmark work stays in Rust (codec_bench_plan / codec_perf).

use jni::objects::{JClass, JString};
use jni::sys::jstring;
use jni::JNIEnv;
use std::sync::atomic::{AtomicBool, Ordering};

static CANCELLED: AtomicBool = AtomicBool::new(false);

/// Initialise GStreamer in THIS process (the service process has its own
/// JavaVM + registry, captured by gstreamer_android's per-process JNI_OnLoad).
#[unsafe(no_mangle)]
pub extern "C" fn Java_org_fcast_android_sender_bench_NativeCodecBench_nativeInit<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
) {
    if let Err(e) = crate::platform::gst_init::ensure_gstreamer_initialized() {
        let _ = env.throw_new("java/lang/RuntimeException", format!("GStreamer init failed: {e}"));
    }
}

/// Run the JSON benchmark plan and return a JSON response string.
#[unsafe(no_mangle)]
pub extern "C" fn Java_org_fcast_android_sender_bench_NativeCodecBench_nativeRunBenchmarkPlanJson<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    request_json: JString<'local>,
) -> jstring {
    CANCELLED.store(false, Ordering::SeqCst);

    let request: String = match env.get_string(&request_json) {
        Ok(s) => s.into(),
        Err(e) => {
            let fallback = format!(
                "{{\"ok\":false,\"report\":\"\",\"ranDecode\":false,\"shouldKillProcess\":false,\"error\":\"JNI get_string failed: {e}\"}}"
            );
            return env.new_string(fallback).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut());
        }
    };

    let response = crate::codec_bench_plan::run_benchmark_plan_json(&request);

    env.new_string(response)
        .or_else(|_| env.new_string("{\"ok\":false,\"error\":\"new_string failed\"}"))
        .map(|s| s.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

#[unsafe(no_mangle)]
pub extern "C" fn Java_org_fcast_android_sender_bench_NativeCodecBench_nativeCancel<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
) {
    CANCELLED.store(true, Ordering::SeqCst);
}

/// Check inside long benchmark loops (optional) to abort early.
pub fn is_cancelled() -> bool {
    CANCELLED.load(Ordering::SeqCst)
}
```

> Optional cancellation: in `codec_perf::run_*_benchmarks`, between sub-tests, add
> `if crate::codec_bench_jni::is_cancelled() { report.push_str("Cancelled.\n"); return report; }`.

---

← [Step 2](step-2-codec-bench-plan.md) · [Index](README.md) · Next → [Step 4](step-4-lib-register-result-downcall.md)
