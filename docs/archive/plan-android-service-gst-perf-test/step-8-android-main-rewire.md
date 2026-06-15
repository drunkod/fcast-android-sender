# Step 8 — `src/android_main.rs`: route Full/Decode to the service

← [Step 7](step-7-kotlin-client-binding.md) · [Index](README.md) · Next → [Step 9](step-9-optional-foreign-egl.md)

Keep **Encode** and **List factories** in-process (safe — no HW decoder). Route
**Full** and **Decode** to the `:codec_bench` service (the result returns via the
`nativeCodecBenchmarkResult` downcall, Step 4c).

### 8a — Publish the UI handle for the downcall (once, near the other setup)

```rust
let _ = crate::PERF_UI_WEAK.set(ui.as_weak());
```

### 8b — Rewire `on_run_perf_test` (Full) → service

Replace the body that called `run_full_benchmark()`:

```rust
ui.global::<Bridge>().on_run_perf_test({
    let ui_weak = ui.as_weak();
    move || {
        let _ = ui_weak.upgrade_in_event_loop(|ui| {
            ui.global::<Bridge>().set_perf_test_running(true);
            set_perf_log(&ui, "Running full benchmark in :codec_bench process…\nThis may take 1–2 minutes.");
        });
        // Full = factory + encode + decode, kill process after.
        let req = crate::codec_bench_plan::CodecBenchRequest {
            include_factory_list: true,
            include_encode_perf: true,
            include_decode_perf: true,
            kill_process_after_decode: true,
            use_foreign_egl: true,
        };
        if let Err(e) = crate::codec_bench_jni::request_codec_benchmark(&req) {
            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                set_perf_log(&ui, &format!("Failed to start benchmark service: {e}"));
                ui.global::<Bridge>().set_perf_test_running(false);
            });
        }
        // Result/running=false arrive via nativeCodecBenchmarkResult.
    }
});
```

### 8c — Rewire `on_run_perf_decode_only` → service (decode only)

```rust
ui.global::<Bridge>().on_run_perf_decode_only({
    let ui_weak = ui.as_weak();
    move || {
        let _ = ui_weak.upgrade_in_event_loop(|ui| {
            ui.global::<Bridge>().set_perf_test_running(true);
            set_perf_log(&ui, "Running decode benchmark in :codec_bench process…");
        });
        let req = crate::codec_bench_plan::CodecBenchRequest {
            include_factory_list: false,
            include_encode_perf: false,
            include_decode_perf: true,
            kill_process_after_decode: true,
            use_foreign_egl: true,
        };
        if let Err(e) = crate::codec_bench_jni::request_codec_benchmark(&req) {
            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                set_perf_log(&ui, &format!("Failed to start benchmark service: {e}"));
                ui.global::<Bridge>().set_perf_test_running(false);
            });
        }
    }
});
```

### 8d — Leave Encode + List in-process (unchanged)

`on_run_perf_encode_only` → `crate::codec_perf::run_encode_benchmarks()` and
`on_run_perf_list_factories` → `crate::codec_perf::list_codec_factories()` stay
exactly as they are (safe in the UI process). `set_perf_log` helper is unchanged.

> Net effect: the UI process never calls `run_decode_benchmarks()`. The `eglTerminate()`
> now happens only inside `:codec_bench`, which is killed right after — the UI's
> EGLDisplay is never touched.

---

← [Step 7](step-7-kotlin-client-binding.md) · [Index](README.md) · Next → [Step 9](step-9-optional-foreign-egl.md)
