Да. Твой текущий `src/codec_perf.rs` уже хороший как **core benchmark module**: он pure-Rust, использует `gst = "gstreamer" 0.25`, считает buffers на `fakesink name=sink`, ищет `amcvidenc-*` / `amcviddec-*` и строит encode/decode pipelines. 

Я бы переписал план так: **benchmark logic остаётся в Rust**, но **Android Service всё равно нужен как тонкая оболочка**, потому что отдельный Android process создаётся через `android:process` у Android-компонента. Android-документация говорит, что `android:process=":name"` создаёт private process для service, а bound service даёт IPC-клиент/сервер между компонентами. ([Android Developers][1]) ([Android Developers][2])

## Оптимальный Rust-first план

```text
Main process
  Slint UI / Skia GL
  Rust calls only safe encode benchmark OR asks Android service to run full benchmark

:codec_bench process
  Android Service, thin Kotlin/Java shell
  loads same Rust .so
  Rust runs GStreamer benchmark plan
  decode tests run last
  optional GStreamer 1.26 foreign EGL wrapper
  returns JSON to UI
  process is killed after HW decode diagnostics
```

GStreamer 1.26+ полезен здесь по двум причинам: Android builds переходят на CMake-in-Gradle / `FindGStreamerMobile.cmake`, а `androidmedia` получил улучшения для H.264/H.265 profiles/levels и pixel-format mappings. Также в 1.26 добавлен `gst_gl_display_egl_set_foreign()` для контроля, должен ли GStreamer вызывать `eglTerminate()`. ([GStreamer][3]) ([GStreamer][3]) ([GStreamer][3])

---

# 1. Раздели Rust-модули

```text
src/codec_perf.rs
  Твой текущий core: run_pipeline_benchmark(), factory discovery, encode/decode pipelines.

src/codec_bench_plan.rs
  JSON request/result, решает какие тесты запускать.

src/codec_bench_jni.rs
  JNI exports для Android Service process.

src/codec_egl.rs
  Optional: GStreamer 1.26+ foreign EGLDisplay bridge.

src/codec_perf_ui.rs
  Safe UI-process helpers: no HW decode.
```

---

# 2. Измени главный принцип в `codec_perf.rs`

Сейчас у тебя `run_full_benchmark()` делает:

```rust
report.push_str(&list_codec_factories());
report.push_str(&run_encode_benchmarks());
report.push_str(&run_decode_benchmarks());
```

Это опасно для UI-process. Я бы заменил на два разных entrypoint:

```rust
/// Safe to call from Slint/UI process.
/// Never runs HW decode.
pub fn run_sender_safe_benchmark() -> String {
    let mut report = String::new();

    report.push_str(&list_codec_factories());
    report.push('\n');
    report.push_str(&run_encode_benchmarks());
    report.push('\n');

    report.push_str("===== DECODE BENCHMARK =====\n");
    report.push_str("Skipped in UI process: HW androidmedia decode may affect EGL state.\n");
    report.push_str("Run full benchmark through :codec_bench Android service.\n");

    report.push('\n');
    report.push_str(&encoder_recommendation());

    report
}

/// Only call inside :codec_bench process.
pub fn run_isolated_full_benchmark() -> String {
    let mut report = String::new();

    report.push_str(&list_codec_factories());
    report.push('\n');

    // Sender-relevant tests first.
    report.push_str(&run_encode_benchmarks());
    report.push('\n');

    // Dangerous diagnostics last.
    report.push_str(&run_decode_benchmarks());
    report.push('\n');

    report.push_str(&encoder_recommendation());

    report
}

fn encoder_recommendation() -> String {
    let mut report = String::new();
    report.push_str("===== RECOMMENDATION =====\n");

    let avc_enc = find_amc_encoder("avc").or_else(|| find_amc_encoder("h264"));
    let hevc_enc = find_amc_encoder("hevc").or_else(|| find_amc_encoder("h265"));

    if avc_enc.is_some() && hevc_enc.is_some() {
        report.push_str("Both AVC and HEVC HW encoders available.\n");
        report.push_str("AVC: best compatibility / usually safer latency.\n");
        report.push_str("HEVC: better compression if receiver supports it.\n");
    } else if avc_enc.is_some() {
        report.push_str("Only AVC HW encoder available. Use H.264 for streaming.\n");
    } else if hevc_enc.is_some() {
        report.push_str("Only HEVC HW encoder available. Use H.265 for streaming.\n");
    } else {
        report.push_str("No HW video encoder found. Software fallback may be slow.\n");
    }

    report
}
```

---

# 3. Добавь JSON benchmark plan в Rust

`src/codec_bench_plan.rs`

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodecBenchRequest {
    pub include_factory_list: bool,
    pub include_encode_perf: bool,
    pub include_decode_perf: bool,

    /// If true, decode tests are run last and caller should kill :codec_bench
    /// process after result is delivered.
    pub kill_process_after_decode: bool,

    /// Optional GStreamer 1.26+ EGL protection inside benchmark process.
    pub use_foreign_egl: bool,
}

impl Default for CodecBenchRequest {
    fn default() -> Self {
        Self {
            include_factory_list: true,
            include_encode_perf: true,
            include_decode_perf: false,
            kill_process_after_decode: true,
            use_foreign_egl: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodecBenchResponse {
    pub ok: bool,
    pub report: String,
    pub ran_decode: bool,
    pub should_kill_process: bool,
    pub error: Option<String>,
}

impl CodecBenchResponse {
    pub fn ok(report: String, ran_decode: bool, should_kill_process: bool) -> Self {
        Self {
            ok: true,
            report,
            ran_decode,
            should_kill_process,
            error: None,
        }
    }

    pub fn error(error: impl Into<String>) -> Self {
        Self {
            ok: false,
            report: String::new(),
            ran_decode: false,
            should_kill_process: false,
            error: Some(error.into()),
        }
    }
}

pub fn run_benchmark_plan_json(request_json: &str) -> String {
    let request: CodecBenchRequest = match serde_json::from_str(request_json) {
        Ok(v) => v,
        Err(e) => {
            let resp = CodecBenchResponse::error(format!("Invalid request JSON: {e}"));
            return serde_json::to_string(&resp).unwrap_or_else(|_| "{}".into());
        }
    };

    let result = std::panic::catch_unwind(|| run_benchmark_plan(request));

    let response = match result {
        Ok(resp) => resp,
        Err(_) => CodecBenchResponse::error("Rust benchmark panicked"),
    };

    serde_json::to_string(&response).unwrap_or_else(|e| {
        format!(
            "{{\"ok\":false,\"report\":\"\",\"ranDecode\":false,\"shouldKillProcess\":false,\"error\":\"serialize failed: {e}\"}}"
        )
    })
}

pub fn run_benchmark_plan(request: CodecBenchRequest) -> CodecBenchResponse {
    let mut report = String::new();

    if request.include_factory_list {
        report.push_str(&crate::codec_perf::list_codec_factories());
        report.push('\n');
    }

    if request.include_encode_perf {
        report.push_str(&crate::codec_perf::run_encode_benchmarks());
        report.push('\n');
    }

    /*
     * Decode last. If androidmedia HW decoder damages EGL state, we already
     * collected factory + encode results.
     */
    if request.include_decode_perf {
        if request.use_foreign_egl {
            let _guard = crate::codec_egl::try_install_foreign_egl_for_current_process();
            report.push_str("Foreign EGL guard: enabled if supported.\n\n");
        }

        report.push_str(&crate::codec_perf::run_decode_benchmarks());
        report.push('\n');
    }

    report.push_str(&crate::codec_perf::encoder_recommendation());

    CodecBenchResponse::ok(
        report,
        request.include_decode_perf,
        request.include_decode_perf && request.kill_process_after_decode,
    )
}
```

Dependencies:

```toml
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

---

# 4. Rust JNI exports for the Android Service

Да, это JNI, но только boundary. GStreamer benchmark остаётся Rust.

`src/codec_bench_jni.rs`

```rust
use jni::objects::{JClass, JString};
use jni::sys::jstring;
use jni::JNIEnv;
use std::sync::atomic::{AtomicBool, Ordering};

static CANCELLED: AtomicBool = AtomicBool::new(false);

#[no_mangle]
pub extern "system" fn Java_com_example_bench_NativeCodecBench_nativeInit(
    mut env: JNIEnv,
    _class: JClass,
) {
    if let Err(e) = crate::platform::gst_init::ensure_gstreamer_initialized() {
        let _ = env.throw_new(
            "java/lang/RuntimeException",
            format!("GStreamer init failed: {e}"),
        );
    }
}

#[no_mangle]
pub extern "system" fn Java_com_example_bench_NativeCodecBench_nativeRunBenchmarkPlanJson(
    mut env: JNIEnv,
    _class: JClass,
    request_json: JString,
) -> jstring {
    CANCELLED.store(false, Ordering::SeqCst);

    let request: String = match env.get_string(&request_json) {
        Ok(s) => s.into(),
        Err(e) => {
            let fallback = format!(
                "{{\"ok\":false,\"report\":\"\",\"ranDecode\":false,\"shouldKillProcess\":false,\"error\":\"JNI get_string failed: {e}\"}}"
            );
            return env.new_string(fallback).unwrap().into_raw();
        }
    };

    let response = crate::codec_bench_plan::run_benchmark_plan_json(&request);

    env.new_string(response)
        .unwrap_or_else(|_| env.new_string("{\"ok\":false,\"error\":\"new_string failed\"}").unwrap())
        .into_raw()
}

#[no_mangle]
pub extern "system" fn Java_com_example_bench_NativeCodecBench_nativeCancel(
    _env: JNIEnv,
    _class: JClass,
) {
    CANCELLED.store(true, Ordering::SeqCst);
}

pub fn is_cancelled() -> bool {
    CANCELLED.load(Ordering::SeqCst)
}
```

Dependencies:

```toml
jni = "0.21"
```

Then in long benchmark loops you can periodically check:

```rust
if crate::codec_bench_jni::is_cancelled() {
    report.push_str("Benchmark cancelled.\n");
    return report;
}
```

---

# 5. Thin Kotlin service shell

Ты просил Rust, поэтому Kotlin оставляем минимальным. Полностью pure-Rust Android Service practically не стоит делать: Android framework создаёт `Service` component через Java/Kotlin class.

`AndroidManifest.xml`

```xml
<service
    android:name=".bench.CodecBenchmarkService"
    android:process=":codec_bench"
    android:exported="false"
    android:stopWithTask="true" />
```

AIDL нужен, если хочешь normal IPC/progress/cancel. Android описывает AIDL как способ определить интерфейс, по которому client и service общаются через IPC; при этом вызовы из remote process приходят из thread pool, так что реализация должна быть thread-safe. ([Android Developers][4])

`NativeCodecBench.kt`

```kotlin
package com.example.bench

object NativeCodecBench {
    init {
        System.loadLibrary("your_rust_lib_name")
    }

    external fun nativeInit()
    external fun nativeRunBenchmarkPlanJson(requestJson: String): String
    external fun nativeCancel()
}
```

`CodecBenchmarkService.kt`

```kotlin
package com.example.bench

import android.app.Service
import android.content.Intent
import android.os.IBinder
import android.os.Process
import java.util.concurrent.Executors

class CodecBenchmarkService : Service() {
    private val executor = Executors.newSingleThreadExecutor()

    override fun onCreate() {
        super.onCreate()
        NativeCodecBench.nativeInit()
    }

    override fun onBind(intent: Intent): IBinder {
        return binder
    }

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

---

# 6. AIDL files

`ICodecBenchmarkService.aidl`

```aidl
package com.example.bench;

import com.example.bench.ICodecBenchmarkCallback;

interface ICodecBenchmarkService {
    void runBenchmark(String requestJson, ICodecBenchmarkCallback callback);
    void cancel();
}
```

`ICodecBenchmarkCallback.aidl`

```aidl
package com.example.bench;

interface ICodecBenchmarkCallback {
    void onResult(String resultJson);
}
```

---

# 7. Rust-side optional GStreamer 1.26 EGL guard

Rust bindings expose `GLDisplayEGL::with_egl_display()` and `GLDisplayEGLExt::set_foreign()`, with `set_foreign()` available behind the `v1_26` feature. ([docs.rs][5]) ([docs.rs][5])

I would make this **optional** because the separate process is already the main protection.

`Cargo.toml`

```toml
gstreamer = "0.25"
gstreamer-gl = "0.25"
gstreamer-gl-egl = { version = "0.25", features = ["v1_26"] }
glib = "0.22"
```

`src/codec_egl.rs`

```rust
#[cfg(target_os = "android")]
pub struct ForeignEglGuard {
    _display: Option<gstreamer_gl_egl::GLDisplayEGL>,
}

#[cfg(not(target_os = "android"))]
pub struct ForeignEglGuard;

pub fn try_install_foreign_egl_for_current_process() -> ForeignEglGuard {
    #[cfg(target_os = "android")]
    {
        match try_create_foreign_egl_display() {
            Ok(display) => ForeignEglGuard {
                _display: Some(display),
            },
            Err(e) => {
                tracing::warn!("foreign EGL guard disabled: {e}");
                ForeignEglGuard { _display: None }
            }
        }
    }

    #[cfg(not(target_os = "android"))]
    {
        ForeignEglGuard
    }
}

#[cfg(target_os = "android")]
fn try_create_foreign_egl_display() -> Result<gstreamer_gl_egl::GLDisplayEGL, String> {
    use gstreamer_gl_egl::prelude::*;

    /*
     * Option A: let GStreamer create EGL display object.
     * In separate process this is often enough.
     */
    let display = gstreamer_gl_egl::GLDisplayEGL::new()
        .map_err(|e| format!("GLDisplayEGL::new failed: {e}"))?;

    /*
     * Critical GStreamer 1.26+ line.
     * If this display wraps/owns an EGLDisplay, mark it foreign so finalization
     * does not call eglTerminate().
     */
    display.set_foreign(true);

    Ok(display)
}
```

If you want to inject the context into a particular pipeline, add this helper:

```rust
pub fn set_gl_display_context_on_pipeline(
    pipeline: &gst::Pipeline,
    display: &gstreamer_gl_egl::GLDisplayEGL,
) -> Result<(), String> {
    use gst::prelude::*;
    use gstreamer_gl::prelude::*;

    let ctx = gst::Context::new(gstreamer_gl::GL_DISPLAY_CONTEXT_TYPE, true);

    {
        let s = ctx.structure();
        let Some(mut s) = s else {
            return Err("context has no writable structure".into());
        };

        /*
         * Depending on exact gstreamer-rs version, this may need adjustment.
         * If this does not compile, use the C helper for gst_context_set_gl_display().
         */
        s.set("gst.gl.GLDisplay", display);
    }

    pipeline.set_context(&ctx);
    Ok(())
}
```

Important: if the safe Rust API does not expose exactly the `gst_context_set_gl_display()` helper you need, keep the EGL guard in a tiny C helper or use `gstreamer-gl-egl-sys`. The separate process is the real safety layer; `set_foreign(true)` is the extra 1.26+ hygiene.

---

# 8. Rewrite your `run_pipeline_benchmark` slightly

Your current runner is fine, but I would add:

1. `decode_is_dangerous` flag only at plan level.
2. Always set `State::Null`.
3. Drop pipeline before returning.
4. Decode tests last.
5. No direct decode from Slint process.

Patch idea:

```rust
pub enum ProcessSafety {
    UiProcess,
    IsolatedBenchmarkProcess,
}

pub fn run_decode_benchmarks_checked(process_safety: ProcessSafety) -> String {
    match process_safety {
        ProcessSafety::UiProcess => {
            "===== DECODE BENCHMARK =====\nSkipped: run through :codec_bench service.\n".into()
        }
        ProcessSafety::IsolatedBenchmarkProcess => run_decode_benchmarks(),
    }
}
```

Then:

```rust
pub fn run_full_benchmark_for_ui_process() -> String {
    let mut report = String::new();

    report.push_str(&list_codec_factories());
    report.push('\n');
    report.push_str(&run_encode_benchmarks());
    report.push('\n');
    report.push_str(&run_decode_benchmarks_checked(ProcessSafety::UiProcess));
    report.push('\n');
    report.push_str(&encoder_recommendation());

    report
}

pub fn run_full_benchmark_for_service_process() -> String {
    let mut report = String::new();

    report.push_str(&list_codec_factories());
    report.push('\n');
    report.push_str(&run_encode_benchmarks());
    report.push('\n');
    report.push_str(&run_decode_benchmarks_checked(
        ProcessSafety::IsolatedBenchmarkProcess,
    ));
    report.push('\n');
    report.push_str(&encoder_recommendation());

    report
}
```

---

# 9. Slint UI side: call service, not Rust decode directly

Pseudo-Rust side:

```rust
fn run_benchmark_from_slint(ui_weak: slint::Weak<AppWindow>) {
    let request = serde_json::json!({
        "includeFactoryList": true,
        "includeEncodePerf": true,
        "includeDecodePerf": true,
        "killProcessAfterDecode": true,
        "useForeignEgl": true
    })
    .to_string();

    /*
     * This function should be your Android/Kotlin bridge.
     * It binds to CodecBenchmarkService in :codec_bench process.
     */
    crate::android_bridge::run_codec_benchmark_service(request, move |result_json| {
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_report_text(result_json.into());
            }
        });
    });
}
```

UI-process direct safe version:

```rust
fn run_safe_sender_benchmark_from_slint(ui_weak: slint::Weak<AppWindow>) {
    std::thread::spawn(move || {
        let report = crate::codec_perf::run_sender_safe_benchmark();

        let _ = slint::invoke_from_event_loop(move || {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_report_text(report.into());
            }
        });
    });
}
```

---

## Итоговая рекомендация

Для твоего Rust-кода я бы сделал так:

```text
Keep:
  src/codec_perf.rs
    run_pipeline_benchmark()
    list_codec_factories()
    run_encode_benchmarks()
    run_decode_benchmarks()

Change:
  run_full_benchmark() -> НЕ использовать из UI process

Add:
  run_sender_safe_benchmark()
  run_isolated_full_benchmark()
  codec_bench_plan.rs
  codec_bench_jni.rs
  optional codec_egl.rs

Android:
  CodecBenchmarkService in :codec_bench process
  thin Kotlin/AIDL shell
  all real work in Rust
  kill :codec_bench after HW decode
```

Самое важное правило: **Slint process никогда не вызывает `run_decode_benchmarks()` напрямую**. Decode benchmark остаётся, но только через `:codec_bench` service process.

[1]: https://developer.android.com/guide/topics/manifest/service-element "<service>  |  App architecture  |  Android Developers"
[2]: https://developer.android.com/develop/background-work/services/bound-services?utm_source=chatgpt.com "Bound services overview | Background work"
[3]: https://gstreamer.freedesktop.org/releases/1.26/ "GStreamer 1.26 release notes"
[4]: https://developer.android.com/develop/background-work/services/aidl "Android Interface Definition Language (AIDL)  |  Background work  |  Android Developers"
[5]: https://docs.rs/gstreamer-gl-egl/latest/gstreamer_gl_egl/struct.GLDisplayEGL.html "GLDisplayEGL in gstreamer_gl_egl - Rust"
