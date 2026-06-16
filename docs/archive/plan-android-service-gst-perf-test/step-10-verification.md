# Step 10 — Verification

← [Step 9](step-9-optional-foreign-egl.md) · [Index](README.md)

## Build

```bash
nix develop .#android -c bash scripts/build-deploy.sh
```
(Compiles Rust + Slint, AIDL stubs, Kotlin; packages the `.so`; installs.)

## Functional checks (Settings → DEBUG → Codec performance test)

1. **List factories** — in-process, instant, lists `amc*`. ✅ (unchanged)
2. **Encode only** — in-process, AVC/HEVC encode FPS. ✅ (unchanged)
3. **Decode only** — should now spawn the service process, run decode, return a
   report, and **the UI must NOT crash**.
4. **Full benchmark** — factory + encode + decode via the service, returns the
   full report; UI survives.

## The decisive checks

**A. A separate process actually spawned and was killed:**
```bash
nix develop .#android -c bash -c '
  while true; do adb shell ps -A | grep "org.fcast.android.sender:codec_bench" && echo "--alive--"; sleep 1; done'
```
You should see `…:codec_bench` appear during the run and disappear after (the
service self-kills post-decode).

**B. `eglTerminate()` happens in the BENCH process, not the UI process — and the
UI process never panics:**
```bash
nix develop .#android -c bash -c 'adb logcat -c'
# tap Full benchmark, let it finish
nix develop .#android -c bash -c '
  adb logcat -d | grep -E "eglTerminate|EGL_NOT_INITIALIZED|panicked|swapping buffers|codec_perf"'
```
Pass = the UI-process pid shows **no** `EGL_NOT_INITIALIZED` / `swapping buffers`
panic, and the app stays foreground. (The old crash was
`android_main.rs:2082` — it must be gone.)

**C. 🔑 amc factories register in the service process** (the biggest risk):
the returned report's factory section (or `adb logcat -s fcastsender | grep amc`
from the `:codec_bench` pid) must list `amcvidenc-*` / `amcviddec-*`. If it shows
**0**, apply the Step 6 contingency (pass the service `Context` to GStreamer init).

## Logs
```bash
nix develop .#android -c bash -c 'adb logcat -s fcastsender RustStdoutStderr | grep -E "perf-bench|codec_bench"'
```

## Rollback / fallback
If the service path misbehaves on a device, the **lighter alternative**
([step-0](step-0-lighter-alternative.md)) — dropping decode from the UI process —
remains a one-commit safe fallback that also eliminates the crash.

## Summary of files

| Action | File | Step |
|--------|------|------|
| Edit | `src/codec_perf.rs` (entrypoints, `ProcessSafety`, `encoder_recommendation`) | 1 |
| New  | `src/codec_bench_plan.rs` | 2 |
| New  | `src/codec_bench_jni.rs` (+ `request_codec_benchmark` upcall) | 3,7 |
| Edit | `src/lib.rs` (modules, `PERF_UI_WEAK`, result downcall) | 4 |
| Edit | `app/src/main/AndroidManifest.xml` (`:codec_bench` service) | 5 |
| New  | Kotlin `bench/NativeCodecBench.kt`, `CodecBenchmarkService.kt`, 2 AIDL | 6 |
| New/Edit | Kotlin `bench/CodecBenchmarkClient.kt`, `FcastApp` ctx, `MainActivity` native decl | 7 |
| Edit | `src/android_main.rs` (route Full/Decode to service) | 8 |
| Opt  | `src/codec_egl.rs` + deps/feature | 9 |

---

← [Step 9](step-9-optional-foreign-egl.md) · [Index](README.md)
