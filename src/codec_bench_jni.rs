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
        let _ = env.throw_new(
            "java/lang/RuntimeException",
            format!("GStreamer init failed: {e}"),
        );
    }
}

/// Run the JSON benchmark plan and return a JSON response string.
#[unsafe(no_mangle)]
pub extern "C" fn Java_org_fcast_android_sender_bench_NativeCodecBench_nativeRunBenchmarkPlanJson<
    'local,
>(
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
            return env
                .new_string(fallback)
                .map(|s| s.into_raw())
                .unwrap_or(std::ptr::null_mut());
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

#[cfg(target_os = "android")]
pub fn request_codec_benchmark(
    request: &crate::codec_bench_plan::CodecBenchRequest,
) -> Result<(), String> {
    use crate::jni_bridge::helpers::{load_app_class, vm};

    let json = serde_json::to_string(request).map_err(|e| format!("serialize request: {e}"))?;

    let vm = vm();
    let mut env = vm
        .attach_current_thread()
        .map_err(|e| format!("attach: {e}"))?;
    let class = load_app_class(
        &mut env,
        "org/fcast/android/sender/bench/CodecBenchmarkClient",
    )
    .map_err(|e| format!("load CodecBenchmarkClient: {e}"))?;
    let arg = env
        .new_string(&json)
        .map_err(|e| format!("new_string: {e}"))?;
    env.call_static_method(class, "start", "(Ljava/lang/String;)V", &[(&arg).into()])
        .map_err(|e| format!("call start: {e}"))?;
    Ok(())
}
