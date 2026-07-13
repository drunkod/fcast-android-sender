# FCast Android Sender — v0.2.0 Implementation Plan

> **Scope:** full scene system + basic widgets (text/image/crop/clock) + RIST destination.
> Derived from `draft/moblin-fcast-version-map.md`, `draft/moblin-scene-widget-mapping.md`,
> `draft/slint-ui/phases/PHASE-40-scene-system.md`, `PHASE-41-widget-system.md`,
> and adapted to the **actual codebase** for best performance.
>
> **Do not modify the code base** — this is a planning document. Each step cites
> real file paths/anchors and ships full Rust + Slint snippets.
>
> **Depends on:** v0.1.0 (SRT/RTMP/WHEP destinations) — see `docs/v0.1.0/`.

---

## 0. The one big performance decision: reuse `MixerNode`, don't build a new compositor

The research docs (PHASE-41-B) propose a brand-new `nodes/compositor.rs`. **Your
codebase already has the compositor** — `crates/migration-runtime/src/nodes/mixer.rs`
builds a `compositor`-based pipeline with **dynamic sink pads** whose
`x / y / width / height / zorder / alpha` properties are set through
`MixerNode::set_dynamic_pad_property` (`mixer.rs:323`). Those are exactly the
`WidgetLayout` fields.

So a **widget is a mixer slot**:

```
CameraSource ──► Mixer(slot 0 = camera)         ┐
TextWidget(appsrc) ──► Mixer(slot 1)            ├─► Destination (RTMP/SRT/RIST)
ImageWidget(appsrc) ──► Mixer(slot 2)           │
ClockWidget(appsrc) ──► Mixer(slot 3)           ┘
(Crop is special: videocrop on the camera source, not a slot)
```

A **scene** is an app-level composition (config + Bridge) that the NodeManager
translates into existing graph commands: `CreateMixer`, `Connect` (each widget
source → a mixer slot), and `apply-mixer-slot-config` (layout → pad props).
`Command::SetScene` diffs scenes and re-issues those existing primitives.

**Why this matters:** zero new pipeline machinery, reuses tested pad-property
plumbing, no second compositor competing for the GPU, and scene switching is
just slot add/remove/reconfigure on a running mixer (no pipeline rebuild —
matches Moblin's "no full rebuild" behavior).

Each step presents the research-doc architecture **and** the codebase-reuse
adaptation, and recommends the latter.

---

## 1. Plugin / dependency matrix (verified against `app/jni/Android.mk`)

| Need | Element | In Android.mk? | Action |
|---|---|---|---|
| Compositor | `compositor` | ✅ (line 41) | reuse via MixerNode |
| Crop widget | `videocrop` | ✅ (line 52) | none |
| Widget feed | `appsrc` (`app`) | ✅ (line 34) | none |
| **RIST destination** | `ristsink` (`rist`) | ❌ **missing** | **add `rist`** (STEP-1) |
| Text/Clock (pango option) | `textoverlay`/`clockoverlay` (`pango`) | ❌ missing | only if you pick the pango route (STEP-4) |

`GSTREAMER_INCLUDE_FONTS := yes` is already set (line 93), so fonts are bundled
either way. **Recommended widget rendering** uses `appsrc` + a Rust rasterizer
(no `pango`/`gdkpixbuf` plugin needed) — see STEP-4. The **only mandatory plugin
add for v0.2.0 is `rist`.**

---

## 2. Step map

| Step | File | Scope | Layer |
|------|------|-------|-------|
| 1 | [step-1/](step-1/INDEX.md) (1A–1E) | `DestinationFamily::Rist` + pipeline + `rist` plugin + tests | Rust + build |
| 2 | [step-2/](step-2/INDEX.md) (2A–2C) | `Scene` / `SceneWidgetPlacement` / `Widget` / `WidgetType` / `WidgetLayout` + `Command` variants + serde tests | Rust |
| 3 | [step-3/](step-3/INDEX.md) (3A–3D) | `SetScene` → mixer-slot graph translation in `NodeManager` (reuse MixerNode) | Rust |
| 4 | [step-4/](step-4/INDEX.md) (4A–4E) | Crop (`videocrop`), Image (`image` crate → appsrc), Text/Clock (rasterizer → appsrc) | Rust |
| 5 | [step-5/](step-5/INDEX.md) (5A–5D) | `SceneItem` / `WidgetItem` structs, Bridge props/callbacks, `Panel` variants | Slint |
| 6 | [step-6/](step-6/INDEX.md) (6A–6C) | `scene_list_page.slint` + `scene_edit_page.slint` (slintcn) | Slint |
| 7 | [step-7/](step-7/INDEX.md) (7A–7C) | widget wizard + text/image/crop/clock settings + layout editor (slintcn) | Slint |
| 8 | [step-8/](step-8/INDEX.md) (8A–8C) | scene quick-switch buttons + settings nav entry points | Slint |
| 9 | [step-9/](step-9/INDEX.md) (9A–9D) | scenes/widgets JSON config + Rust callback handlers | Rust |

### Landing order

```
STEP-1 (RIST — fully independent, ship anytime)
STEP-2 ─► STEP-3 ─► STEP-4   (Rust core: model → mixer translation → renderers; squash)
STEP-5 ─► STEP-6 ─► STEP-7 ─► STEP-8   (UI; STEP-5 first)
STEP-9 (persistence + handler wiring — ties UI ↔ runtime; last)
```

---

## 3. Current-state snapshot (verified)

| Component | Status | Location |
|---|---|---|
| `DestinationFamily` (Rtmp/Udp/Whep/LocalFile/LocalPlayback/Srt) | ✅ | `protocol.rs` (Srt from v0.1.0) |
| `DestinationFamily::Rist` | ❌ STEP-1 | to add |
| `compositor` mixer with dynamic pads + `x/y/width/height/zorder/alpha` | ✅ | `nodes/mixer.rs:323` (`set_dynamic_pad_property`) |
| `CreateMixer` / `Connect` / `apply-mixer-slot-config` | ✅ | `protocol.rs`, `node_manager.rs`, `ui/bridge.slint` |
| `Scene` / `Widget` / `WidgetType` / `WidgetLayout` | ❌ STEP-2 | to add |
| `Command::SetScene` / scene→graph translation | ❌ STEP-3 | to add |
| Widget renderers (crop/image/text/clock) | ❌ STEP-4 | to add |
| Scene/Widget Bridge + Panels | ❌ STEP-5 | to add |
| Scene/Widget UI pages | ❌ STEP-6/7 | to create |
| `rist` plugin | ❌ STEP-1 | `app/jni/Android.mk` |

---

## 4. Known limitations baked into the plan

- **Rotation:** `WidgetLayout.rotation` is stored in the model, but plain
  `compositor` pads support `xpos/ypos/width/height/alpha/zorder` only — **no
  arbitrary rotation**. v0.2.0 honors everything except rotation; arbitrary
  rotation is deferred to v0.3.0 (GL transform / `glvideomixer`). STEP-3/4 note this.
- **Browser / chat / alerts / map / vtuber / scoreboard widgets:** out of scope
  (deferred per `moblin-scene-widget-mapping.md` §3) — not applicable or v0.3.0.
- **Scene transitions (crossfade):** out of scope (v0.3.0).
- **Shared camera:** scenes drive the camera→mixer→destination path; only one
  live composition at a time (v0.2.0 single-stream assumption, as in v0.1.0).

---

## 5. Cross-references

| Document | Scope |
|---|---|
| `draft/moblin-fcast-version-map.md` | v0.1.0→v0.3.0 feature matrix |
| `draft/moblin-scene-widget-mapping.md` | scene/widget architecture + Rust structs |
| `draft/slint-ui/phases/PHASE-40-scene-system.md` | scene data model + UI spec |
| `draft/slint-ui/phases/PHASE-41-widget-system.md` | widget types + compositor spec |
| `draft/moblin-rust-conversion-plan.md` | Swift→Rust type mapping tables |
| `crates/migration-runtime/src/nodes/mixer.rs` | the compositor we reuse |
| `crates/migration-runtime/src/node_manager.rs` | graph/command dispatch |
| `docs/v0.1.0/` | prior milestone (SRT/RTMP/WHEP) |
