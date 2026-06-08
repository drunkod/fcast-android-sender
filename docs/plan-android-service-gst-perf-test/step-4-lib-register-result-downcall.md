# Step 4 — `src/lib.rs`: register modules + result downcall

← [Step 3](step-3-codec-bench-jni.md) · [Index](README.md) · Next → [Step 5](step-5-manifest-service.md)

### 4a — Register the new modules

Next to the existing `pub mod codec_perf;`:

```rust
pub mod codec_perf;
pub mod codec_bench_plan;     // ← ADD
pub mod codec_bench_jni;      // ← ADD
#[cfg(feature = "foreign-egl")]
pub mod codec_egl;            // ← ADD (only with Step 9)
```

### 4b — A global the result downcall can reach the UI through

The benchmark result comes back into the **main** process from the service. Stash
the UI handle so the downcall can post to the event loop (Step 8 sets it):

```rust
#[cfg(target_os = "android")]
pub static PERF_UI_WEAK: once_cell::sync::OnceCell<slint::Weak<MainWindow>> =
    once_cell::sync::OnceCell::new();
```

(`once_cell` and `slint`/`MainWindow` are already used in this crate.)

### 4c — Result downcall (main process)

Mirrors the existing `Java_org_fcast_android_sender_MainActivity_native*` exports.
The Kotlin client (Step 7) calls this with the service's JSON result:

```rust
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "C" fn Java_org_fcast_android_sender_MainActivity_nativeCodecBenchmarkResult<'local>(
    mut env: jni::JNIEnv<'local>,
    _class: jni::objects::JClass<'local>,
    result_json: jni::objects::JString<'local>,
) {
    use crate::codec_bench_plan::CodecBenchResponse;

    let json: String = match env.get_string(&result_json) {
        Ok(s) => s.into(),
        Err(_) => return,
    };

    let text = match serde_json::from_str::<CodecBenchResponse>(&json) {
        Ok(resp) if resp.ok => resp.report,
        Ok(resp) => format!("Benchmark failed: {}", resp.error.unwrap_or_default()),
        Err(e) => format!("Bad benchmark result JSON: {e}\nraw: {json}"),
    };

    if let Some(weak) = PERF_UI_WEAK.get() {
        let weak = weak.clone();
        let _ = weak.upgrade_in_event_loop(move |ui| {
            use slint::ComponentHandle;
            let bridge = ui.global::<Bridge>();
            let lines: Vec<slint::SharedString> = text.lines().map(|l| l.into()).collect();
            bridge.set_perf_test_log_lines(std::rc::Rc::new(slint::VecModel::from(lines)).into());
            bridge.set_perf_test_log(text.into());
            bridge.set_perf_test_running(false);
        });
    }
}
```

> If `MainWindow` / `Bridge` aren't already in scope at the crate root where you add
> this, import them (they come from `slint::include_modules!()`): `use crate::{MainWindow, Bridge};`.

---

← [Step 3](step-3-codec-bench-jni.md) · [Index](README.md) · Next → [Step 5](step-5-manifest-service.md)
