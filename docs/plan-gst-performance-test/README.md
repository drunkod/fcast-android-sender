# Plan — GStreamer Codec Performance Page

A new **Codec Performance** panel that runs real GStreamer encode/decode
pipelines (`androidmedia` / AMC elements), counts buffers at `fakesink`, and
reports throughput in FPS. The benchmark is **pure Rust** via the `gst` crate
(GStreamer is already initialised in-process) — **no Kotlin/JNI**.

Based on `docs/draft-plan-gst-performance-test.md` and
`docs/code-examples-gst-performance-test.md`, with the UI updated to the repo's
current **slintcn** idiom.

> Nothing here is applied. Each step file is a build sheet with drop-in snippets.

---

## ⚠️ Deviation from the research (read first)

The research page (`code-examples-…md` §File 5) renders the log as a single
`Text` inside a std-widgets `ScrollView`. **That is the exact pattern that froze
on scroll for the codec-test page** — a monolithic `Text` in a `Flickable` lays
out/renders the whole report every frame. This plan instead:

- renders the log in a **virtualised `ListView`** over a `[string]` line model
  (`perf-test-log-lines`), so only on-screen lines render — same fix applied to
  `codec_test_page.slint`;
- uses slintcn **`Badge`** (running/idle) + **`Separator`** + a **`FocusScope`**
  Escape handler, matching the migrated pages;
- keeps the slintcn-backed **`PrimaryButton`** wrappers (48px touch target + a11y).

Everything Rust-side (`src/codec_perf.rs`) is kept as the research provides.

---

## Steps

| # | File | What |
|---|------|------|
| 1 | [step-1-bridge-panel-enum.md](step-1-bridge-panel-enum.md) | `ui/bridge.slint` — add `codec-perf` to `Panel` |
| 2 | [step-2-bridge-props.md](step-2-bridge-props.md) | `ui/bridge.slint` — perf props (+ line model) + 4 callbacks |
| 3 | [step-3-codec-perf-page-slint.md](step-3-codec-perf-page-slint.md) | `ui/pages/codec_perf_page.slint` — new slintcn page |
| 4 | [step-4-main-slint-route.md](step-4-main-slint-route.md) | `ui/main.slint` — import + PanelHost route |
| 5 | [step-5-settings-entry.md](step-5-settings-entry.md) | `ui/pages/settings_page.slint` — DEBUG-section row |
| 6 | [step-6-codec-perf-rs.md](step-6-codec-perf-rs.md) | `src/codec_perf.rs` — GStreamer benchmark module |
| 7 | [step-7-lib-rs-register.md](step-7-lib-rs-register.md) | `src/lib.rs` — `pub mod codec_perf;` |
| 8 | [step-8-android-main-handlers.md](step-8-android-main-handlers.md) | `src/android_main.rs` — 4 handlers + `set_perf_log` helper |
| 9 | [step-9-verification.md](step-9-verification.md) | Build / verify + risks |

---

## slintcn components used (already vendored)

`ui/slintcn/components/badge.slint`, `separator.slint` — no `slintcn add` needed.
`PrimaryButton` lives in `ui/components/buttons.slint` (slintcn `Button` wrapper).
`ListView` is std-widgets — the **deliberate** virtualisation exception the repo
already makes in `debug_log_page.slint` and `codec_test_page.slint`.

## Build note (important)

`./gradlew assembleDebug` only repackages a prebuilt `.so`; it does **not**
compile Rust or Slint. To actually build this feature:

```bash
nix develop .#android -c bash scripts/build-deploy.sh            # build + install
nix develop .#android -c bash scripts/build-deploy.sh --no-install  # build only
```

## Files to touch

| Action | File | Step |
|--------|------|------|
| **Edit** | `ui/bridge.slint` — `Panel` enum (after `codec-test,` line 117) | 1 |
| **Edit** | `ui/bridge.slint` — perf props after the codec-test block (after line 478) | 2 |
| **New**  | `ui/pages/codec_perf_page.slint` | 3 |
| **Edit** | `ui/main.slint` — import after line 70, route after line 178 | 4 |
| **Edit** | `ui/pages/settings_page.slint` — DEBUG section, after the H.264 row (line 344) | 5 |
| **New**  | `src/codec_perf.rs` | 6 |
| **Edit** | `src/lib.rs` — after `pub mod application;` | 7 |
| **Edit** | `src/android_main.rs` — after the codec-test callbacks, before the `use jni_bridge::camera` block | 8 |
