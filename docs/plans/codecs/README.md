# GStreamer Android Prebuilt SDK Compatibility — Task Index

This directory preserves a legacy per-task codec audit. The individual files are
useful design context, but this index is the current status source of truth; the
old `TODO.codecs.md` source no longer exists.

All tasks relate to the migration code under
[`crates/migration-runtime/src/`](../../../crates/migration-runtime/src/) — primarily the
nodes in `crates/migration-runtime/src/nodes/` and the runtime in
`crates/migration-runtime/src/runtime.rs`.

## Summary Matrix

| ID | Current status | Component | File |
|----|----------------|-----------|------|
| 1 | Done | H.264 encoder selection uses Android factory/caps discovery and ranked candidates | [detail](01-h264-encoder-fallback-chain.md) |
| 2 | Done | RTMP chooses `rtmp2sink` with `rtmpsink` fallback | [detail](02-rtmp2sink-absence.md) |
| 3 | Done | Source creation falls back from `fallbacksrc` to `uridecodebin` | [detail](03-fallbacksrc-absence.md) |
| 4 | Done | Android encoder input negotiates `NV12` through a caps filter | [detail](04-videoconvert-color-space.md) |
| 5 | Partial | Per-pipeline profiles and element errors exist; no single startup preflight covers every path | [detail](05-startup-element-validation.md) |
| 6 | Not applicable now | No Rust GStreamer plugin is shipped by the current app | [detail](06-rust-plugin-jni-registration.md) |
| 7 | Done | Android encoder discovery uses factory capabilities and rank | [detail](07-rank-based-encoder-discovery.md) |
| 8 | Done | Encoder bitrate/keyframe properties are capability-checked | [detail](08-amcvidenc-properties.md) |
| 9 | Done | Timecode elements are optional and conditionally linked | [detail](09-timecodestamper-timeoverlay.md) |
| 10 | Open | `uridecodebin` fallback lacks the reconnection behavior of `fallbacksrc` | [detail](10-uridecodebin-reconnection.md) |
| 11 | Open audit | `deinterlace` remains mandatory in source and generator paths | [detail](11-deinterlace-audit.md) |
| 12 | Deferred | WHIP is a product/protocol decision, not an Android SDK compatibility fix | [detail](12-whip-as-rtmp-replacement.md) |
| 13 | Active elsewhere | Zero-copy camera work is covered by StreamPack steps 08-11 | [detail](13-zero-copy-video-frame-path.md) |
| 14 | Deferred | APK size optimization | [detail](14-apk-size-static-linking.md) |
| 15 | Done for app boundary | Android-only JNI/platform code is gated; node crates remain intentionally portable | [detail](15-cfg-target-os-android-gating.md) |
| 16 | Not applicable now | Revisit only if the app starts shipping `gst-plugins-rs` artifacts | [detail](16-gst-plugins-rs-cross-compile.md) |
| 17 | Partial | Unit/profile coverage exists; device SDK element-availability coverage remains open | [detail](17-element-availability-tests.md) |

## Remaining order

1. Finish StreamPack step 07 before changing the camera media path.
2. Treat task 13 as StreamPack steps 08-11, not as a separate implementation.
3. Then address task 10, task 11, and the device portion of task 17.
4. Keep tasks 12 and 14 deferred until product or release constraints justify them.

## Priority Legend

- **P0** — Critical: app crashes or features completely broken on Android.
- **P1** — High: functional gaps that will be hit in production.
- **P2** — Medium: robustness and quality improvements.
- **P3** — Low: optimizations and future-proofing.
