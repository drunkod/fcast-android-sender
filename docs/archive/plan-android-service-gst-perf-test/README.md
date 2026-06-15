# Plan — Isolated Codec Benchmark via a Separate Android Service Process

Fixes the **decode/Full-benchmark crash** by running the GStreamer HW-decode
diagnostics in a separate Android process (`:codec_bench`), so they can never
touch the UI process's EGL state. Adapted from
`docs/draft-plan-android-service-gst-perf-test.md` to this codebase.

> Nothing here is applied. Each step file is a build sheet with drop-in snippets.

---

## Why (confirmed by on-device debugging)

The crash is **not** in the codec logic. logcat at the moment of death:

```
W libEGL : eglTerminate() called w/ 2 objects remaining        ← gst-amc HW decoder, on the benchmark thread
E libEGL : validate_display error 3001 (EGL_NOT_INITIALIZED)    ← Slint Skia renderer tries to draw
RustStdoutStderr: panicked at src/android_main.rs:2082:14
  Skia OpenGL Renderer: Error swapping buffers: [3001] initialization failed
```

The GStreamer **hardware video decoder** (`amcviddec` → Android codec2/gst-amc),
used only in the decode pipelines, calls `eglTerminate()` on the process's shared
`EGLDisplay`. Slint's Skia GL renderer shares that display, so its next
`eglSwapBuffers` fails → `ui.run().unwrap()` (android_main.rs:2082) panics.
Matches the symptom table exactly: **List ✅ / Encode ✅ / Decode 💥 / Full 💥**.

**Fix:** put HW decode in a separate OS process. `eglTerminate()` there is harmless
to the UI process. Kill `:codec_bench` after decode to release everything.

---

## ⚠️ This is the heavy solution — consider the lighter one first

This app is a **sender**: it encodes and streams, it never decodes video in
production. So the simplest correct fix is to **drop the decode benchmark** from
the UI process entirely (keep encode + factory list, which never crash) — a
~30-line change, no new process, no AIDL, no JNI. See
[step-0-lighter-alternative.md](step-0-lighter-alternative.md).

The plan below (steps 1–10) implements the draft's **full isolated-service**
design, which *keeps* HW-decode diagnostics available but safely. It is
significantly more machinery (new process, AIDL, JNI boundary, Kotlin service).
Pick based on whether decode FPS numbers are worth the complexity.

---

## Architecture (adapted to this repo)

```text
Main process (org.fcast.android.sender, NativeActivity + Slint/Skia GL)
  "List factories" / "Encode only"  → Rust in-process (SAFE: no HW decoder)
  "Full benchmark" / "Decode only"  → upcall MainActivity.startCodecBenchmark(json)
                                       → binds the :codec_bench service (AIDL)
                                       ← nativeCodecBenchmarkResult(json) downcall
                                       → set_perf_test_log_lines on the UI thread

:codec_bench process (android:process=":codec_bench")
  CodecBenchmarkService (thin Kotlin) loads gstreamer_android + fcastsender
  → NativeCodecBench.nativeRunBenchmarkPlanJson(json)
  → Rust run_benchmark_plan(): factory list + encode, then decode LAST
  → returns JSON; service calls Process.killProcess() after decode
```

The same Rust `.so` runs in both processes; each process has its own EGLDisplay,
GStreamer registry, and JavaVM (captured per-process by `gstreamer_android`'s
`JNI_OnLoad`).

---

## Deviations from the draft (adapted to real names/conventions)

| Draft (generic) | This repo (actual) |
|---|---|
| `com.example.bench` | `org.fcast.android.sender.bench` |
| `System.loadLibrary("your_rust_lib_name")` | `loadLibrary("gstreamer_android")` **then** `loadLibrary("fcastsender")` |
| `extern "system"` / `#[no_mangle]` | `extern "C"` / `#[unsafe(no_mangle)]` + `<'local>` (matches `src/lib.rs`) |
| `AppWindow` / `set_report_text` | `MainWindow` / `Bridge.set_perf_test_log_lines` (+ `set_perf_test_log`) |
| Bind service *from Rust* | Bind service *from Kotlin*; Rust upcalls to start + receives a downcall result (mirrors camera-capture flow) |
| new serde/serde_json/jni deps | already present (`serde 1.0`, `serde_json 1.0`, `jni 0.21.1`) — no Cargo change for the core |
| Foreign-EGL guard (step 7) | **optional**, gated on GStreamer 1.26 + new `gstreamer-gl-egl` dep — the separate process is the real fix |

---

## Steps

| # | File | What |
|---|------|------|
| 0 | [step-0-lighter-alternative.md](step-0-lighter-alternative.md) | (Alternative) drop decode from the UI process — no service |
| 1 | [step-1-codec-perf-entrypoints.md](step-1-codec-perf-entrypoints.md) | `src/codec_perf.rs` — sender-safe + isolated entrypoints, `encoder_recommendation`, `ProcessSafety` |
| 2 | [step-2-codec-bench-plan.md](step-2-codec-bench-plan.md) | `src/codec_bench_plan.rs` (new) — JSON request/response + `run_benchmark_plan` |
| 3 | [step-3-codec-bench-jni.md](step-3-codec-bench-jni.md) | `src/codec_bench_jni.rs` (new) — JNI exports for the service process |
| 4 | [step-4-lib-register-result-downcall.md](step-4-lib-register-result-downcall.md) | `src/lib.rs` — register modules + `nativeCodecBenchmarkResult` downcall |
| 5 | [step-5-manifest-service.md](step-5-manifest-service.md) | `AndroidManifest.xml` — `:codec_bench` service |
| 6 | [step-6-kotlin-service-aidl.md](step-6-kotlin-service-aidl.md) | Kotlin `NativeCodecBench`, `CodecBenchmarkService` + AIDL |
| 7 | [step-7-kotlin-client-binding.md](step-7-kotlin-client-binding.md) | Kotlin client bind + `MainActivity.startCodecBenchmark` + Rust upcall |
| 8 | [step-8-android-main-rewire.md](step-8-android-main-rewire.md) | `src/android_main.rs` — route Full/Decode to the service, keep Encode/List in-process |
| 9 | [step-9-optional-foreign-egl.md](step-9-optional-foreign-egl.md) | (Optional) `src/codec_egl.rs` foreign-EGL guard (GStreamer 1.26) |
| 10 | [step-10-verification.md](step-10-verification.md) | Build, on-device verify, confirm no `eglTerminate` in the main process |

## Build note

`./gradlew assembleDebug` only repackages a prebuilt `.so`. Rust/Slint/Kotlin
changes need:
```bash
nix develop .#android -c bash scripts/build-deploy.sh
```

## Biggest risk to validate early (Step 10)

GStreamer init + `amc*` factory registration in the **`:codec_bench` process**,
where `android_main`/NativeActivity never runs. `ensure_gstreamer_initialized()`
calls `gst::init()`, which on Android relies on `gstreamer_android`'s per-process
`JNI_OnLoad` having captured that process's JavaVM/Context. If the service process
lists **0 amc factories**, GStreamer needs the service `Context` passed at init —
covered as a contingency in Step 6.
