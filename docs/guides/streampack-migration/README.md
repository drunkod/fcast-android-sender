# StreamPack migration — step-by-step

This folder splits [`../streampack-migration-plan.md`](../streampack-migration-plan.md)
into per-sub-step, independently reviewable work items. Each `step-NN/` folder is
self-contained: goal, dependencies, files touched, **full code**, how to verify, and
the risks pulled forward from the master plan.

> **Status:** steps 01-06 implemented; step 07 is the active on-device validation
> gate. Steps 08-13 remain planned. See the
> [current roadmap](../../plans/README.md) before starting later phases.

## Order & dependencies

```
Phase 1 — StreamPack direct SRT (validation)
  step-01  ✅ Dependencies + manifest check          (no deps)
  step-02  ✅ StreamPackSenderBridge                  (needs 01)
  step-03  ✅ StreamPackCameraCaptureCoordinator      (needs 02)
  step-04  ✅ AppGraph factory + MainActivity wiring  (needs 03)
  step-05  ✅ JNI upcall + pipeline-mode flag (Rust)  (needs 04)
  step-06  ✅ Slint mode selector + app.rs wiring     (needs 05)
  step-07  ▶ Validate StreamPackDirectSrt             (needs 01–06)   ← active gate

Phase 2 — StreamPack encoded → Rust/GStreamer (target)
  step-08  Custom endpoint + Rust encoded ingest   (needs 07 green)
  step-09  H.264 camera source node (Rust)         (needs 08)
  step-10  Mux-only destination path (Rust)        (needs 09)

Phase 3+ — cleanup & future
  step-11  Retire GL readback for streaming        (needs 10 stable)
  step-12  Preview via SurfaceProcessor            (needs 10)
  step-13  OBS-style SurfaceProcessor compositor   (long-term)
```

| Step | Status | Title | Master §| Lang | Touches |
|------|--------|-------|---------|------|---------|
| 01 | Done | [Dependencies + manifest](step-01-dependencies/README.md) | §3, §4 | Gradle | `libs.versions.toml`, `app/build.gradle` |
| 02 | Done | [StreamPackSenderBridge](step-02-streampack-bridge/README.md) | §5.2, §6 | Kotlin | `stream/StreamPackSenderBridge.kt` |
| 03 | Done | [Coordinator](step-03-coordinator/README.md) | §5.1 | Kotlin | `capture/StreamPackCameraCaptureCoordinator.kt` |
| 04 | Done | [AppGraph + MainActivity wiring](step-04-mainactivity-wiring/README.md) | §5.3 | Kotlin | `AppGraph.kt`, `MainActivity.kt` |
| 05 | Done | [JNI upcall + mode flag](step-05-jni-and-mode-flag/README.md) | §5.4, §8 | Kotlin/Rust | `MainActivity.kt`, `camera.rs`, `lib.rs`, `config/mod.rs` |
| 06 | Done | [Slint surface](step-06-slint-surface/README.md) | §9 | Slint/Rust | `bridge.slint`, `camera_page.slint`, `app.rs` |
| 07 | **Active** | [Validate direct SRT](step-07-validate-direct-srt/README.md) | §5.5 | — | on-device test |
| 08 | Blocked by 07 | [Encoded endpoint + ingest](step-08-encoded-endpoint/README.md) | §7.1, §7.2 | Kotlin/Rust | new `stream/RustGStreamerEndpoint.kt`, `main_activity.rs`, `helpers.rs` |
| 09 | Blocked by 08 | [H.264 camera source node](step-09-h264-camera-source/README.md) | §7.3 | Rust | `camera_source.rs` |
| 10 | Blocked by 09 | [Mux-only destination](step-10-muxonly-destination/README.md) | §7.4 | Rust | `destination.rs` |
| 11 | Later | [Retire GL readback](step-11-retire-gl-readback/README.md) | §10 | — | docs/policy |
| 12 | Later | [Preview SurfaceProcessor](step-12-preview-surfaceprocessor/README.md) | §11 | Kotlin | `StreamPackSenderBridge.kt` |
| 13 | Deferred | [OBS SurfaceProcessor](step-13-obs-surfaceprocessor/README.md) | §12 | Kotlin/Rust | new compositor |

## Golden rule (every step honours it)

```
BEFORE encoding:  Android Surface / GL / MediaCodec   (StreamPack owns this)
AFTER  encoding:  Rust / GStreamer / SRT              (we keep this)
```

Do **not** start Phase 2 (step-08+) until step-07 proves lower CPU/thermal load and
clean start/stop. Flag OFF must remain byte-for-byte identical to today at every step.
