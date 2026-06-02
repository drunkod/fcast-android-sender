# Plan: Camera RTMP Stream Page

Step-by-step plan for adding a **Camera RTMP Stream** page to the FCast Android sender. Background, design references, and the canonical code snippets live in [`research-how-create-rtmp-page.md`](./research-how-create-rtmp-page.md); this document is the execution checklist that maps that research onto the current repo layout.

## Goal

A new Slint screen, reachable from Settings → Advanced, that:

1. Shows a camera preview of the phone's camera (front/back/external).
2. Lets the user edit camera source settings (resolution, framerate, mirror, stabilization, zoom).
3. Lets the user enter an RTMP URL + stream key.
4. Starts/stops a live RTMP publish driven by either the **migration runtime** (in-process node graph) or the **gst-pop daemon** (WebSocket JSON-RPC), selected by the existing `Bridge.media-backend` setting.

## Design references

- `draft/moblin-ui/Moblin/View/Stream/StreamView.swift` — camera preview composition
- `draft/moblin-ui/Moblin/View/Settings/Stream/Wizard/Custom/StreamWizardCustomRtmpSettingsView.swift` — RTMP URL + key form
- `draft/moblin-ui/Moblin/View/Settings/Stream/Url/StreamUrlSettingsView.swift` — URL placeholder examples
- `ui/pages/mixer_page.slint` — RTMP destination card, state chip, start/stop bar pattern
- `ui/pages/test_functionality_page.slint` — camera source rows, live state handling
- `ui/pages/camera_page.slint` — existing camera settings idioms

## Repo orientation (for path accuracy)

| Concern | Path |
|---|---|
| Slint UI | `ui/pages/`, `ui/bridge.slint`, `ui/main.slint` |
| Rust app glue | `src/android_main.rs` |
| Migration runtime (in-process) | `crates/migration-runtime/src/` (nodes in `nodes/`, protocol in `protocol.rs`, manager in `node_manager.rs`) |
| gst-pop client | `crates/gstpop-runtime/src/` |
| Android capture (Kotlin) | `app/src/main/java/org/fcast/android/sender/capture/` |
| JNI bridge | `src/jni_bridge/` |

> Note: the research file refers to `senders/android/src/migration/...`. In this repo the migration runtime is the `crates/migration-runtime` crate and the existing screen capture node is `crates/migration-runtime/src/nodes/screen_capture.rs`. There is currently **no camera capture Kotlin** (only `ScreenCaptureCoordinator`), so a new Kotlin coordinator must be added.

---

## Step-by-step plan

### Phase 1 — UI skeleton (compile-only, no backend)

Goal: page exists, navigable, all Bridge state plumbed, but Start/Stop are no-ops.

1. **Add Bridge state and callbacks** in `ui/bridge.slint`:
   - Properties: `cam-rtmp-camera-idx`, `cam-rtmp-resolution-idx`, `cam-rtmp-framerate-idx`, `cam-rtmp-mirror`, `cam-rtmp-stabilization`, `cam-rtmp-zoom`, `cam-rtmp-url`, `cam-rtmp-stream-key`, `cam-rtmp-state: MixerState`, `cam-rtmp-error-text`.
   - Callbacks: `start-camera-rtmp-stream()`, `stop-camera-rtmp-stream()`.
   - Add `camera-rtmp-stream` variant to the `Panel` enum.

2. **Create `ui/pages/camera_rtmp_stream_page.slint`** using the structure in the research doc (header, scrollable body with sections: Camera Preview, Camera Source, Destination, Backend; sticky action bar with state chip + Go Live / Stop). Reuse `SettingsSection`, `SettingsValueRow`, `SettingsToggleRow`, `SettingsSliderRow`, `CapturePreview`, `PanelHeader`, `PrimaryButton`, `DestructiveButton`.

3. **Route the panel** in `ui/main.slint`:
   - Import `CameraRtmpStreamPage`.
   - Add `if PanelBridge.active == Panel.camera-rtmp-stream: CameraRtmpStreamPage { }` inside `PanelHost`.

4. **Add nav entry** in `ui/pages/settings_page.slint` Advanced section: row "Camera RTMP Stream" that calls `PanelBridge.push(Panel.camera-rtmp-stream)`.

5. **Wire empty callbacks** in `src/android_main.rs`: register `on_start_camera_rtmp_stream` / `on_stop_camera_rtmp_stream` that just set `cam-rtmp-state` to `Starting`/`Stopping` then back to `Idle`. This lets us verify the UI end-to-end before any media plumbing exists.

**Verify:** build, open Settings → Advanced → Camera RTMP Stream, edit fields, click Go Live / Stop, see state chip cycle.

### Phase 2 — Android camera capture (Kotlin + JNI)

Goal: a global YUV frame channel mirrors `FRAME_PAIR` for the camera, populated by Camera2 callbacks. No streaming yet.

6. **Kotlin**: add `app/src/main/java/org/fcast/android/sender/capture/CameraCaptureCoordinator.kt`:
   - Opens Camera2 device for selected lens (front/back/external).
   - Configures an `ImageReader` for `YUV_420_888` at requested width/height/fps.
   - Applies mirror/stabilization/zoom via `CaptureRequest` keys.
   - On each frame: converts to I420 contiguous buffer, calls a new JNI entry `nativePushCameraFrame(buf, width, height, ptsNs)`.

7. **Manifest**: add `android.permission.CAMERA` to `app/src/main/AndroidManifest.xml` and request it at runtime via the existing permission flow (mirror `CapturePermissionResult` pattern).

8. **JNI bridge**: in `src/jni_bridge/` add `native_push_camera_frame` that writes into a new global `CAMERA_FRAME_PAIR: (Mutex<Option<VideoFrame>>, Condvar)` and an `AtomicBool CAMERA_CAPTURE_ACTIVE`. Expose `set_camera_capture_active(bool)` to Kotlin via JNI.

9. **Lifecycle helpers** in `src/lib.rs` (or wherever `FRAME_PAIR` lives): export `CAMERA_FRAME_PAIR`, `CAMERA_CAPTURE_ACTIVE`, and a `set_camera_capture_active` wrapper that toggles the Kotlin coordinator via JNI upcall.

**Verify:** add a temp debug log in `nativePushCameraFrame` that prints frame size every 30 frames; toggle capture from Phase 1's stub callback and confirm frames flow.

### Phase 3 — Migration-runtime path

Goal: `start-camera-rtmp-stream` builds a node graph that publishes to RTMP using existing `DestinationFamily::Rtmp`.

10. **Protocol** (`crates/migration-runtime/src/protocol.rs`): add `Command::CreateCameraSource { id, camera_idx, width, height, fps, mirror, stabilization, zoom }` (serde tag matching existing convention).

11. **Node** (`crates/migration-runtime/src/nodes/camera_source.rs`): port the `CameraSourceNode` from the research doc. Pipeline: `appsrc(I420) → videoconvert → appsink`. The `appsrc` `need-data` callback drains `CAMERA_FRAME_PAIR`. Mirror the structure of `nodes/screen_capture.rs` exactly for fan-out, state transitions, and teardown.

12. **Wire node** in `crates/migration-runtime/src/nodes/mod.rs` and `node_manager.rs`:
    - `pub mod camera_source;`
    - Add `NodeRecord::CameraSource(CameraSourceNode)` variant.
    - Extend the command dispatch and downstream `Connect`/`Start`/`Remove` arms.

13. **Verify destination compatibility**: confirm `DestinationFamily::Rtmp` element list in `destination.rs` matches what the research doc references. If `timecodestamper`/`timeoverlay` cause failures on Android GStreamer builds, make them optional (try-create, skip on failure).

14. **Rust glue** in `src/android_main.rs`: implement `on_start_camera_rtmp_stream` migration arm — issue `CreateCameraSource` + `CreateDestination(Rtmp)` + `Connect` + `Start(dest)` + `Start(src)`, then `set_camera_capture_active(true)`. Implement `on_stop_camera_rtmp_stream` to `Remove` both nodes and `set_camera_capture_active(false)`. Set `cam-rtmp-state` and `cam-rtmp-error-text` from results.

**Verify:** point at a local nginx-rtmp or `rtmp://localhost/live/test`, hit Go Live, confirm video arrives.

### Phase 4 — gst-pop daemon path

Goal: same UI button works when `Bridge.media-backend == GstPop`.

15. **Helpers** in `src/android_main.rs` (or a new `src/camera_rtmp_gstpop.rs`):
    - `start_camera_rtmp_via_gstpop(ui, w, h, fps, url)` — start embedded daemon, connect `TypedGstPopClient`, build pipeline string `appsrc … ! videoconvert ! x264enc … ! h264parse ! queue ! flvmux ! rtmp2sink|rtmpsink location=…`, call `create_pipeline` + `play`, then `set_camera_capture_active(true)`, store the returned pid in a `Mutex<Option<String>>`.
    - `stop_camera_rtmp_via_gstpop(ui)` — disable capture, `stop` + `remove_pipeline` for the stored pid.

16. **Sink fallback**: probe `gst::ElementFactory::find("rtmp2sink")` and fall back to `rtmpsink` (see `TODO.codecs/02-rtmp2sink-absence.md`).

17. **Frame ingress for gst-pop**: gst-pop runs in the same process, so `appsrc` in the pipeline description needs a `need-data` handler. Two options — pick during Phase 4 kickoff:
    - **Option A (preferred):** extend gst-pop client/daemon with a "push frame" RPC that the Rust app calls when the camera coordinator emits a frame. This decouples UI thread from frame timing.
    - **Option B:** use `ahcsrc` (Android Hardware Camera Source) directly in the pipeline string, removing the appsrc entirely. Only viable if `ahcsrc` is built into the GStreamer Android bundle — verify with `gst-inspect-1.0 ahcsrc` in the build artifact before committing to this path.

18. **Dispatch** in `on_start_camera_rtmp_stream`: branch on `Bridge.media-backend` between migration and gst-pop paths.

**Verify:** flip `media-backend` to gst-pop, repeat the local RTMP test, confirm parity.

### Phase 5 — Polish

19. **Error surface**: on every failure path, set `cam-rtmp-error-text` and transition state to `Error`; clear text on next successful `Start` or on `Stop`.
20. **Pre-flight validation**: if `cam-rtmp-url` is empty or missing scheme (`rtmp://`/`rtmps://`), disable Go Live and show inline hint instead of attempting connection.
21. **Persist user inputs**: save URL + stream key (key in encrypted store via `src/secret/`) so they survive app restart. Camera/resolution/fps choices to normal config.
22. **Permission UX**: if camera permission denied, the page shows an inline "Grant Camera Access" CTA in place of the preview.
23. **Backwards-compat scan**: search for unused stubs introduced during Phase 1; delete or fold in.

## Files touched (final summary)

| File | Change |
|---|---|
| `ui/pages/camera_rtmp_stream_page.slint` | **New** |
| `ui/bridge.slint` | Add Panel variant, properties, callbacks |
| `ui/main.slint` | Import + panel routing |
| `ui/pages/settings_page.slint` | Nav row in Advanced |
| `crates/migration-runtime/src/protocol.rs` | `Command::CreateCameraSource` |
| `crates/migration-runtime/src/nodes/camera_source.rs` | **New** |
| `crates/migration-runtime/src/nodes/mod.rs` | Re-export |
| `crates/migration-runtime/src/node_manager.rs` | Dispatch + `NodeRecord` variant |
| `crates/migration-runtime/src/nodes/destination.rs` | (Maybe) optional timecodestamper/overlay |
| `src/jni_bridge/` | `native_push_camera_frame`, capture active toggle |
| `src/lib.rs` | `CAMERA_FRAME_PAIR`, `CAMERA_CAPTURE_ACTIVE` |
| `src/android_main.rs` | Start/stop callbacks + gst-pop helpers |
| `app/src/main/java/org/fcast/android/sender/capture/CameraCaptureCoordinator.kt` | **New** |
| `app/src/main/AndroidManifest.xml` | `CAMERA` permission |

## Open questions resolved

- **Does the GStreamer Android bundle ship `ahcsrc`?**
  - Resolved: No. We proceeded with Option A (Binary WebSocket ingress via custom `push_buffer` JSON-RPC method), feeding contiguous YUV frames over binary WebSockets from Rust to the daemon.
- **Does it ship `rtmp2sink` from `gst-plugins-rs`, or are we stuck with `rtmpsink`?**
  - Resolved: Both may be present or absent depending on build configurations. We implemented runtime probing via `pick_rtmp_sink()`, which falls back to `rtmpsink` if `rtmp2sink` is not registered.
- **Is the encrypted secret store (`src/secret/`) wired on Android already, or only iOS?**
  - Resolved: Yes, it is fully wired on Android using the encrypted key store and file-based fallback depending on the environment, allowing secure persistence of the RTMP stream key.

