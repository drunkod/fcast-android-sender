# Step 0 — (Alternative) Drop HW decode from the UI process

[Index](README.md) · if you take this, you can skip steps 1–10.

This app **sends** (encodes + streams) and never decodes video. The decode
roundtrip is the only thing that instantiates a HW decoder — the element that
calls `eglTerminate()` and crashes the UI. Removing it is the smallest correct
fix: no new process, no AIDL, no JNI.

### a) `src/codec_perf.rs` — make `run_full_benchmark` skip decode

```rust
pub fn run_full_benchmark() -> String {
    let mut report = String::new();

    report.push_str(&list_codec_factories());
    report.push('\n');
    report.push_str(&run_encode_benchmarks());
    report.push('\n');

    // HW decode (amcviddec) calls eglTerminate() on the shared EGLDisplay, which
    // crashes Slint's Skia GL renderer. This app only encodes, so we skip it.
    report.push_str("===== DECODE BENCHMARK =====\n");
    report.push_str("Skipped: HW decode disturbs the UI EGL context (sender app does not decode).\n\n");

    // … keep the existing RECOMMENDATION block …
    report
}
```

### b) `ui/pages/codec_perf_page.slint` — drop the "Decode only" button

Remove the `PrimaryButton { label: @tr("Decode only"); … run-perf-decode-only … }`
from the second button row (leave "Encode only").

### c) `src/android_main.rs` — drop the `on_run_perf_decode_only` handler

Delete that one handler block. Optionally also remove the now-unused
`run_decode_benchmarks` / `encode_decode_pipeline` / `find_amc_decoder` from
`codec_perf.rs` (ask before deleting — they're harmless if left).

### d) `ui/bridge.slint` — optional

Leave `run-perf-decode-only` or remove it; an unused callback is harmless.

**Result:** Full + Encode + List all run safely in-process; the crash is gone.
You lose HW-decode FPS numbers (which this app never needed). If you *do* want
those numbers safely, use the full isolated-service plan (steps 1–10).

---

[Index](README.md)
