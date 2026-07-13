# Current implementation roadmap

> **Status:** active
> **Last reconciled:** 2026-07-13 against `origin/main` at `c816a6e`
> **Evidence:** Git branch audit, Graphify code graph (2,046 nodes / 4,931 edges),
> and targeted source verification.

This file is the source of truth for what should happen next. Completed milestone
plans are archived under [`../archive/plans/`](../archive/plans/); they are useful
implementation history, not open work.

## Current baseline

The following work is already on `main` and must not be re-planned:

- v0.1.0 destination work: SRT protocol model, pipeline, settings UI, and connect wiring.
- v0.2.0 scene/widget work: RIST, scene and widget models, scene CRUD/application,
  mixer-slot layout, image/text/clock/crop rendering, UI pages, quick switching, and
  persisted scene/widget configuration.
- StreamPack Phase 1 implementation through the UI/mode-selector wiring (steps 1-6).
- Adaptive SRT bitrate corrections for StreamPack 3.1.2.

Graphify confirms the active runtime path around `Scene`, `Widget`, `WidgetSourceNode`,
`MixerNode`, `NodeManager::apply_scene`, and `StoredBackendConfig`. It also confirms
the StreamPack path from `MainActivity` through `StreamPackCameraCaptureCoordinator`
to `StreamPackSenderBridge`.

## Active milestone: validate StreamPack Phase 1

The immediate next step is
[`step-07-validate-direct-srt`](../guides/streampack-migration/step-07-validate-direct-srt/README.md).
Do not begin the encoded-to-GStreamer path until this gate is green.

Success criteria:

- Direct SRT streams successfully on at least one arm64 Android device.
- Start/stop/restart works repeatedly without camera, codec, or surface leaks.
- Preview orientation, mirroring, and crop match the encoded output.
- CPU load, thermal behavior, and dropped frames improve over `LegacyRawI420Gstreamer`.
- Restart-required mode selection and the legacy fallback both work as documented.
- The result, device/API level, duration, and measurements are recorded in the step-07 doc.

## Next milestone: encoded StreamPack output to GStreamer

Only after the Phase 1 gate passes:

1. Step 08: add the encoded endpoint and Rust ingest boundary.
2. Step 09: add the H.264 camera source node.
3. Step 10: add the mux-only destination path.
4. Run an end-to-end SRT regression comparison against direct StreamPack SRT and the
   legacy raw-I420 path.

The core invariant remains: Android/StreamPack owns capture and encoding; Rust/GStreamer
owns encoded-frame muxing and transport.

## Later work

- Step 11: retire GL readback only after steps 8-10 are stable and the fallback policy
  is explicit.
- Step 12: add preview fanout through `SurfaceProcessor`.
- Step 13: treat the OBS-style compositor as a separate long-term project, not part of
  the current migration milestone.
- Re-triage the remaining codec backlog in [`codecs/README.md`](codecs/README.md) after
  the StreamPack validation gate; zero-copy work is now part of the StreamPack path.

## Explicitly not on the active plan

- Do not merge any of the four divergent local branches wholesale. Their disposition is
  documented in [`branch-audit-2026-07-13.md`](branch-audit-2026-07-13.md).
- Do not revive pre-extraction paths such as `src/migration/` or Java
  `MainActivity.java` patches against the current Kotlin host.
- Do not expand the gstpop daemon/desktop-tool experiment without a current product
  requirement and a new plan based on the extracted `crates/gstpop-runtime` API.

## Planning discipline

- Update this file when a gate changes state.
- Keep implementation detail in the corresponding guide, not duplicated here.
- Move completed plans to `docs/archive/plans/`.
- Record device-only verification explicitly; code presence is not evidence that an
  Android media path has passed its runtime gate.
