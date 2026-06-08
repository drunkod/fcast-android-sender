# Moblin → FCast Versioned Feature Mapping

> Ties together **SRT**, **RTMP**, and the **Scene & Widget system** — mapping
> Moblin features to FCast versions/phases with Rust/Slint/GStreamer equivalents.
>
> **Legend:** `[x]` done · `[-]` in-progress · `[ ]` planned · `[~]` stretch · `[—]` not-applicable

---

## v0.1.0 — MVP Cast Loop (Current target)

### Streaming Protocols

| Feature | Status | FCast location | Notes |
|---|---|---|---|
| RTMP destination (`DestinationFamily::Rtmp`) | `[x]` done | `crates/migration-runtime/src/nodes/destination.rs` | `rtmp2sink` + `flvmux` |
| WHEP destination (`DestinationFamily::Whep`) | `[x]` done | `nodes/destination.rs` | WebRTC egress for local preview |
| UDP destination (`DestinationFamily::Udp`) | `[x]` done | `nodes/destination.rs` | `mpegtsmux` → `udpsink` |
| SRT destination (`DestinationFamily::Srt`) | `[ ]` planned | MVP-PHASE-8 | `mpegtsmux` → `srtsink` (mirrors UDP) |
| RTMP settings UI page | `[ ]` planned | Phase 31-A | `ui/pages/protocol_rtmp_settings_page.slint` |
| SRT settings UI page | `[ ]` planned | Phase 31-B | `ui/pages/protocol_srt_settings_page.slint` |

### Scenes & Widgets

Not included in v0.1.0 MVP — camera + encoder + single destination is the focus.

### Rust Backend

| Component | Status | Location |
|---|---|---|
| `DestinationFamily` enum (Rtmp/Udp/Whep/LocalFile/LocalPlayback) | `[x]` done | `protocol.rs:191` |
| `Command::CreateDestination` | `[x]` done | `protocol.rs:114` |
| `Command::CreateCameraSource` | `[x]` done | `protocol.rs:97` |
| `Command::CreateScreenCaptureSource` | `[x]` done | `protocol.rs:88` |
| `NodeManager` graph lifecycle | `[x]` done | `crates/migration-runtime/src/` |
| Connect-receiver wiring in UI | `[ ]` planned | MVP-PHASE-1, `ui/pages/connect_page.slint` |

### Moblin Swift files mapped (v0.1.0)

| Moblin source | FCast target | Status |
|---|---|---|
| `View/Settings/Streams/Stream/Rtmp/StreamRtmpSettingsView.swift` | `ui/pages/protocol_rtmp_settings_page.slint` | `[ ]` Phase 31-A |
| `View/Settings/Streams/Stream/Srt/StreamSrtSettingsView.swift` | `ui/pages/protocol_srt_settings_page.slint` | `[ ]` Phase 31-B |
| N/A (connect/cast flow) | `ui/pages/connect_page.slint` | `[-]` partial |

---

## v0.2.0 — Scene System + Basic Widgets

> **Depends on:** v0.1.0 complete (SRT destination + connect wiring + protocol UI)
>
> **Phases:** PHASE-40 (scene system) + PHASE-41 (widget system)

### Streaming Protocols

| Feature | Status | FCast location | Notes |
|---|---|---|---|
| SRT adaptive bitrate (FastIRL algorithm) | `[ ]` planned | New phase (post-31) | `srtsink` bitrate negotiation via stats callback |
| RTMP reconnect-on-disconnect | `[ ]` planned | New phase | Auto-reconnect logic in destination node |
| RIST destination | `[ ]` planned | New `DestinationFamily::Rist` variant | `ristsink` element |

### Scenes & Widgets — Data Model

| Feature | Status | FCast location | GStreamer |
|---|---|---|---|
| `Scene` struct in protocol.rs | `[ ]` planned | `crates/migration-runtime/src/protocol.rs` | — |
| `SceneWidgetPlacement` struct | `[ ]` planned | `protocol.rs` | — |
| `Widget` struct + `WidgetType` enum | `[ ]` planned | `protocol.rs` | — |
| `WidgetLayout` struct (x/y/w/h/rotation/opacity) | `[ ]` planned | `protocol.rs` | Compositor pad props |
| Scene switching command (`Command::SetScene`) | `[ ]` planned | `protocol.rs` | Compositor pad reconfigure |
| Widget layout update command | `[ ]` planned | `protocol.rs` | Pad property update |

### Scenes & Widgets — UI

| Feature | Status | FCast Slint file | Notes |
|---|---|---|---|
| Scene list settings page | `[ ]` planned | `ui/pages/scene_list_page.slint` | `for scene in Bridge.scenes:` |
| Scene edit page | `[ ]` planned | `ui/pages/scene_edit_page.slint` | Video source + mic + widget list |
| Widget creation wizard | `[ ]` planned | `ui/pages/wizard_widget_create_page.slint` | Type picker → config steps |
| Widget layout editor | `[ ]` planned | `ui/pages/scene_widget_edit_page.slint` | Drag handles via `TouchArea` |
| Scene buttons in stream page bottom bar | `[ ]` planned | `ui/pages/stream_page.slint` | `HorizontalLayout` of `SceneButton` |

### Scenes & Widgets — GStreamer Integration

| Feature | Status | Element(s) | Notes |
|---|---|---|---|
| GStreamer `compositor` for widget layering | `[ ]` planned | `compositor` / `glvideomixer` | One sink pad per active widget |
| Widget: Text overlay | `[ ]` planned | `textoverlay` or `pango` | Format string with variable substitution |
| Widget: Image overlay | `[ ]` planned | `gdkpixbufoverlay` or `imagefreeze` → `appsrc` | Static PNG/JPEG |
| Widget: Crop | `[ ]` planned | `videocrop` | Applied to main source before compositor |
| Widget: Clock/time overlay | `[ ]` planned | `clockoverlay` / `timeoverlay` | GStreamer builtin |
| Widget: QR code (stretch) | `[~]` stretch | `qrencode` crate → RGBA → `appsrc` | Useful for stream info |

### Rust Backend (v0.2.0 additions)

| Component | Location |
|---|---|
| `Scene` struct | `protocol.rs` |
| `SceneWidgetPlacement` struct | `protocol.rs` |
| `Widget` struct | `protocol.rs` |
| `WidgetType` enum (`Text`, `Image`, `Crop`, `Clock`) | `protocol.rs` |
| `WidgetLayout` struct | `protocol.rs` |
| `Command::SetScene { scene_id }` | `protocol.rs` |
| `Command::UpdateWidgetLayout { scene_id, widget_id, layout }` | `protocol.rs` |
| `Command::CreateWidget { widget }` | `protocol.rs` |
| `Command::RemoveWidget { widget_id }` | `protocol.rs` |
| Compositor node in `NodeManager` | `crates/migration-runtime/src/nodes/compositor.rs` (new) |
| Scene state persistence | App-level config (serde JSON, stored via `SecretStore` or local file) |

### Moblin Swift files mapped (v0.2.0)

| Moblin source | FCast target | Status |
|---|---|---|
| `View/Settings/Scenes/ScenesSettingsView.swift` | `ui/pages/scene_list_page.slint` | `[ ]` |
| `View/Settings/Scenes/Scene/SceneSettingsView.swift` | `ui/pages/scene_edit_page.slint` | `[ ]` |
| `View/Settings/Scenes/Scene/SceneWidgetSettingsView.swift` | `ui/pages/scene_widget_edit_page.slint` | `[ ]` |
| `View/Settings/Scenes/Widgets/Widget/WidgetWizardSettingsView.swift` | `ui/pages/wizard_widget_create_page.slint` | `[ ]` |
| `View/Settings/Scenes/Widgets/Widget/WidgetSettingsView.swift` | `ui/pages/scene_widget_edit_page.slint` (layout section) | `[ ]` |
| `View/Settings/Scenes/Widgets/Widget/Text/WidgetTextSettingsView.swift` | `ui/pages/widget_text_settings_page.slint` | `[ ]` |
| `View/Settings/Scenes/Widgets/Widget/Image/WidgetImageSettingsView.swift` | `ui/pages/widget_image_settings_page.slint` | `[ ]` |
| `View/Settings/Scenes/Widgets/Widget/Crop/WidgetCropSettingsView.swift` | `ui/pages/widget_crop_settings_page.slint` | `[ ]` |
| `View/ControlBar/ControlBarPortraitView.swift` (scene buttons) | `ui/pages/stream_page.slint` bottom bar | `[ ]` |
| `View/ControlBar/QuickButtonsView.swift` (widget panel toggle) | `ui/components/control_bar.slint` | `[ ]` |
| `View/Settings/Streams/Stream/Srt/StreamSrtAdaptiveBitrateSettingsView.swift` | `ui/pages/protocol_srt_adaptive_bitrate_page.slint` | `[ ]` |

---

## v0.3.0 — Advanced Widgets + Protocol Extensions

> **Depends on:** v0.2.0 scene system working end-to-end
>
> **Focus:** Browser source (WebView bridge), video effects, advanced protocols

### Streaming Protocols

| Feature | Status | FCast location | Notes |
|---|---|---|---|
| SRTLA bonding (multi-path SRT) | `[ ]` planned | New `DestinationFamily::Srtla` | Requires `libsrtla` or Rust reimpl |
| SRT connection priority / multi-path | `[ ]` planned | Extension to `DestinationFamily::Srt` | Priority-based failover |
| WHIP WebRTC ingress | `[ ]` planned | New `DestinationFamily::Whip` | WebRTC publish to media server |

### Scenes & Widgets — Advanced

| Feature | Status | GStreamer | Notes |
|---|---|---|---|
| Widget: Browser source | `[ ]` planned | Android WebView → SurfaceTexture → `appsrc` | Requires JNI texture bridge |
| Widget: Video effects chain | `[ ]` planned | `videobalance`, `coloreffects`, `glshader` | Per-widget or per-scene |
| Widget: Map/GPS overlay | `[ ]` planned | Map tile render → `appsrc` | Requires `FusedLocationProvider` |
| Scene transitions (crossfade/cut) | `[ ]` planned | `compositor` alpha interpolation or `glvideomixer` blend | Timed transition between scenes |
| Widget: Slideshow (rotating set) | `[ ]` planned | Timer-driven widget swap | Auto-cycle through widget subset |
| Widget: Snapshot (freeze frame) | `[ ]` planned | Last-buffer hold on `appsrc` | Capture current frame |

### Rust Backend (v0.3.0 additions)

| Component | Location |
|---|---|
| `WidgetType::Browser { url, width, height }` | `protocol.rs` |
| `WidgetType::VideoEffect { effect_name, params }` | `protocol.rs` |
| `WidgetType::Map { style, zoom }` | `protocol.rs` |
| `Command::SetSceneTransition { from, to, transition_type, duration_ms }` | `protocol.rs` |
| WebView texture bridge (JNI ↔ GStreamer) | `src/jni_bridge.rs` + Kotlin `WebViewCaptureService` |
| SRTLA bonding logic | `crates/migration-runtime/src/nodes/srtla.rs` (new) |

### Moblin Swift files mapped (v0.3.0)

| Moblin source | FCast target | Status |
|---|---|---|
| `View/Settings/Streams/Stream/Srt/StreamSrtConnectionPriority2View.swift` | `ui/pages/protocol_srt_connection_priority_page.slint` | `[ ]` |
| `View/Settings/Streams/Stream/Whip/StreamWhipSettingsView.swift` | `ui/pages/protocol_whip_settings_page.slint` | `[ ]` |
| `View/Settings/Scenes/Widgets/Widget/Browser/WidgetBrowserSettingsView.swift` | `ui/pages/widget_browser_settings_page.slint` | `[ ]` |
| `View/Settings/Scenes/Widgets/Widget/Effects/WidgetEffectsView.swift` | `ui/pages/widget_effects_settings_page.slint` | `[ ]` |
| `View/Settings/Scenes/Widgets/Widget/Map/WidgetMapSettingsView.swift` | `ui/pages/widget_map_settings_page.slint` | `[ ]` |
| `View/Settings/Scenes/Widgets/Widget/Slideshow/WidgetSlideshowSettingsView.swift` | `ui/pages/widget_slideshow_settings_page.slint` | `[ ]` |
| `View/Settings/Scenes/AutoSwitchers/AutoSwitchersSettingsView.swift` | `ui/pages/auto_scene_switcher_page.slint` | `[ ]` |

---

## Summary Table — Feature × Version

| Feature domain | v0.1.0 MVP | v0.2.0 Scenes | v0.3.0 Advanced |
|---|---|---|---|
| **RTMP** | ✅ Destination | ✅ Reconnect | ✅ Multi-stream |
| **SRT** | ✅ Destination (Phase 8) | ✅ Adaptive bitrate | ✅ SRTLA bonding |
| **RIST** | — | ✅ Basic destination | — |
| **WHIP/WHEP** | ✅ WHEP (done) | — | ✅ WHIP publish |
| **Scenes** | — | ✅ Full system | ✅ Transitions |
| **Widgets: Text** | — | ✅ MVP | ✅ Variables/timers |
| **Widgets: Image** | — | ✅ MVP | ✅ Animated GIF |
| **Widgets: Crop** | — | ✅ MVP | — |
| **Widgets: Clock** | — | ✅ MVP | — |
| **Widgets: Browser** | — | — | ✅ WebView bridge |
| **Widgets: Effects** | — | — | ✅ Filter chain |
| **Widgets: Map** | — | — | ✅ GPS overlay |

---

## Cross-References

| Document | Scope |
|---|---|
| `draft/moblin-scene-widget-mapping.md` | Architecture deep-dive + Rust struct proposals |
| `draft/slint-ui/phases/MVP-PHASE-8-srt-destination-family.md` | SRT destination implementation |
| `draft/slint-ui/phases/PHASE-31-streaming-protocols.md` | SRT/RTMP UI pages |
| `draft/slint-ui/phases/PHASE-40-scene-system.md` | Scene data model + UI (v0.2.0) |
| `draft/slint-ui/phases/PHASE-41-widget-system.md` | Widget types + GStreamer compositor (v0.2.0) |
| `draft/slint-ui/docs/swiftui-to-slint-guide.md` | General SwiftUI→Slint patterns |
| `crates/migration-runtime/src/protocol.rs` | Current `DestinationFamily` + `Command` enum |
| `crates/migration-runtime/src/nodes/destination.rs` | Current RTMP/UDP/WHEP pipelines |
