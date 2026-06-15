# Step 1 — `src/codec_perf.rs`: safe vs isolated entrypoints

← [Index](README.md) · Next → [Step 2](step-2-codec-bench-plan.md)

Your current `run_full_benchmark()` ends with an inline RECOMMENDATION block and
calls `run_decode_benchmarks()` unconditionally. Refactor so the **UI process**
can never run HW decode, while the **service process** runs everything.

### 1a — Extract `encoder_recommendation()` (pub)

Pull the existing RECOMMENDATION tail of `run_full_benchmark` into a reusable
`pub fn` (so both `codec_perf` and `codec_bench_plan` can call it):

```rust
/// Recommendation block (AVC vs HEVC HW encoder availability). Safe in any process.
pub fn encoder_recommendation() -> String {
    let mut report = String::new();
    report.push_str("===== RECOMMENDATION =====\n");

    let avc_enc = find_amc_encoder("avc").or_else(|| find_amc_encoder("h264"));
    let hevc_enc = find_amc_encoder("hevc").or_else(|| find_amc_encoder("h265"));

    if avc_enc.is_some() && hevc_enc.is_some() {
        report.push_str("Both AVC and HEVC HW encoders available.\n");
        report.push_str("  AVC:  best compatibility, lower latency on most devices.\n");
        report.push_str("  HEVC: better compression at same quality, use if receiver supports it.\n");
        report.push_str("Compare FPS numbers above to decide.\n");
    } else if avc_enc.is_some() {
        report.push_str("Only AVC HW encoder available. Use H.264 for streaming.\n");
    } else if hevc_enc.is_some() {
        report.push_str("Only HEVC HW encoder available. Use H.265 for streaming.\n");
    } else {
        report.push_str("No HW encoder found! Software x264enc fallback will have lower performance.\n");
    }
    report
}
```

### 1b — Process-safety guard around decode

```rust
/// Which process is calling. HW decode is only allowed in the isolated benchmark
/// process, because amcviddec calls eglTerminate() on the shared EGLDisplay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessSafety {
    UiProcess,
    IsolatedBenchmarkProcess,
}

/// Decode benchmark gated by process. In the UI process it returns a note
/// instead of running HW decode (which would crash the Slint renderer).
pub fn run_decode_benchmarks_checked(process_safety: ProcessSafety) -> String {
    match process_safety {
        ProcessSafety::UiProcess => {
            "===== DECODE BENCHMARK =====\n\
             Skipped in UI process: HW androidmedia decode calls eglTerminate() and\n\
             disturbs the Slint/Skia EGL context. Run via the :codec_bench service.\n"
                .into()
        }
        ProcessSafety::IsolatedBenchmarkProcess => run_decode_benchmarks(),
    }
}
```

### 1c — Two explicit entrypoints

Replace the old `run_full_benchmark()` callers (Step 8 rewires `android_main`):

```rust
/// Safe to call from the Slint/UI process. Never runs HW decode.
pub fn run_sender_safe_benchmark() -> String {
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

/// Only call inside the :codec_bench process. Decode runs LAST so factory + encode
/// results are already captured before any EGL damage.
pub fn run_isolated_full_benchmark() -> String {
    let mut report = String::new();
    report.push_str(&list_codec_factories());
    report.push('\n');
    report.push_str(&run_encode_benchmarks());
    report.push('\n');
    report.push_str(&run_decode_benchmarks_checked(ProcessSafety::IsolatedBenchmarkProcess));
    report.push('\n');
    report.push_str(&encoder_recommendation());
    report
}
```

> Keep your existing `run_pipeline_benchmark`, `list_codec_factories`,
> `run_encode_benchmarks`, `run_decode_benchmarks`, `find_amc_*`, pipeline builders
> unchanged. You may delete the old inline `run_full_benchmark` (now superseded) —
> but check Step 8 rewires its only caller first.

---

← [Index](README.md) · Next → [Step 2](step-2-codec-bench-plan.md)
