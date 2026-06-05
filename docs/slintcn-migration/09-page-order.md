# Step 9 — Page-by-page migration order

← [Step 8: Pages std-widgets](08-pages-std-widgets.md) · [Index](README.md) · Next → [Step 10: Audit](10-audit.md)

Migrate in this order (most leverage / lowest risk first). After each page: `cargo check`.

## Order

1. **`settings_page.slint`** — exercises Input + Spinner + ScrollView + all three buttons + all
   settings rows. Migrating it validates Steps 3–8 end-to-end.
2. **`audio_page.slint`, `camera_page.slint`, `mixer_page.slint`** — slider/toggle heavy.
3. **`bitrate_preset_edit_page.slint`, `macro_edit_page.slint`, `receiver_rename_page.slint`** —
   `Input` heavy.
4. **All ScrollView-only pages** — mechanical `ScrollView` → `ScrollArea` (+ `content-height`).
5. Remaining pages.

## Per-page cheat-sheet

Because the wrappers (Steps 4–6) preserve the button/row APIs, for most pages **only the raw
std-widgets lines change**:

```
LineEdit          → Input            (placeholder-text → placeholder)
CheckBox          → Checkbox
Slider            → Slider           (slintcn)
ScrollView        → ScrollArea       (+ content-height)
Spinner           → keep, or Skeleton
VerticalBox       → VerticalLayout   (+ padding/spacing)
PrimaryButton     → unchanged call site (wrapper handles it)
TextButton        → unchanged call site
DestructiveButton → unchanged call site
```

## All pages (31 import std-widgets)

```
audio_page.slint                bitrate_preset_edit_page.slint
backup_reset_page.slint         bitrate_presets_page.slint
camera_page.slint               camera_rtmp_stream_page.slint
cast_history_detail_page.slint  cast_history_page.slint
casting_page.slint              codec_test_page.slint
connecting_page.slint           debug_log_page.slint
debug_page.slint                debug_video_page.slint
macro_edit_page.slint           macros_page.slint
media_backend_page.slint        mixer_page.slint
network_page.slint              quick_actions_page.slint
receiver_rename_page.slint      recording_page.slint
settings_page.slint             test_functionality_page.slint
```

(Plus `control_bar.slint` in `ui/components/`.)

## Tight loop per page

```bash
# 1. edit one page
# 2. fast check (host build)
cargo check
# 3. if it has visual scroll/list changes, run the viewer or snapshot test
cargo test --test ui_snapshots   # if present
```

← [Step 8: Pages std-widgets](08-pages-std-widgets.md) · [Index](README.md) · Next → [Step 10: Audit](10-audit.md)
