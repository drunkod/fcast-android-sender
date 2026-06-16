# Plan — Codec Test Page (end-to-end, slintcn UI)

Wire `ui/pages/codec_test_page.slint` to a real Android `MediaCodecList` probe:
Kotlin `CodecDump` → JNI bridge → Slint `Bridge` callbacks → slintcn-styled UI.

This supersedes `docs/draft-plan-codecs-test-page.md` and the UI section of
`docs/examples-code-for-plan-codecs-test-page.md`. **Difference:** the UI here
uses the project's **slintcn** components (`ScrollArea`, slintcn-backed
`PrimaryButton`/`DestructiveButton`, `Badge`, `Separator`, `FocusScope` Escape)
instead of `std-widgets`' `ScrollView`, so it does not regress the migration.

> Nothing in this plan has been applied. Each step file is a build sheet with
> drop-in snippets.

---

## Steps

| # | File | What |
|---|------|------|
| 1 | [step-1-kotlin-codecdump.md](step-1-kotlin-codecdump.md) | New Kotlin `CodecDump.kt` — `MediaCodecList` probe |
| 2 | [step-2-bridge-slint.md](step-2-bridge-slint.md) | `ui/bridge.slint` — 2 props + 3 callbacks |
| 3 | [step-3-codec-test-page-slint.md](step-3-codec-test-page-slint.md) | `ui/pages/codec_test_page.slint` — full slintcn rewrite |
| 4 | [step-4-codec-test-rs.md](step-4-codec-test-rs.md) | New `src/jni_bridge/codec_test.rs` — JNI upcalls |
| 5 | [step-5-mod-rs.md](step-5-mod-rs.md) | `src/jni_bridge/mod.rs` — register module |
| 6 | [step-6-android-main-rs.md](step-6-android-main-rs.md) | `src/android_main.rs` — 3 callback handlers |
| 7 | [step-7-verification.md](step-7-verification.md) | Build / install / verify + risks |

---

## slintcn components used (already vendored)

These live under `ui/slintcn/components/` already — **no `slintcn add` needed**.
For reference, the equivalent install commands are:

| Component | Import | Install (if missing) |
|-----------|--------|----------------------|
| `ScrollArea` | `../slintcn/components/scroll-area.slint` | `npx slintcn@latest add scroll-area` |
| `Badge`, `BadgeVariant` | `../slintcn/components/badge.slint` | `npx slintcn@latest add badge` |
| `Separator`, `SeparatorOrientation` | `../slintcn/components/separator.slint` | `npx slintcn@latest add separator` |
| `Button` (via wrappers) | `../components/buttons.slint` | `npx slintcn@latest add button` |

`PrimaryButton` / `DestructiveButton` / `LoadingView` in `ui/components/buttons.slint`
are thin wrappers over slintcn `Button` that re-add accessibility props and a
48px (`Theme.row-height`) Material touch target. **Prefer the wrappers over a raw
`Button`** — call-sites stay `label:` + `enabled:` and get TalkBack for free.

`ScrollArea` API (important — differs from std `ScrollView`):

```slint
// You must give it the total content height; it scrolls when that exceeds the viewport.
ScrollArea {
    content-height: content.preferred-height;   // bind to the inner layout's height
    content := VerticalLayout { /* tall content */ }
}
```

---

## Files to touch

| Action | File | Step |
|--------|------|------|
| **New** | `app/src/main/java/org/fcast/android/sender/codec/CodecDump.kt` | 1 |
| **Edit** | `ui/bridge.slint` — add 2 properties + 3 callbacks before the global's closing `}` (line 466) | 2 |
| **Replace** | `ui/pages/codec_test_page.slint` — full slintcn rewrite | 3 |
| **New** | `src/jni_bridge/codec_test.rs` | 4 |
| **Edit** | `src/jni_bridge/mod.rs` — add `pub mod codec_test;` | 5 |
| **Edit** | `src/android_main.rs` — insert 3 callback handlers after `on_pick_test_overlay_image` (line ~1199) | 6 |
