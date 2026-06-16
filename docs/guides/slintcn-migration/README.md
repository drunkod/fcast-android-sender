# slintcn Migration — fcast-android-sender

> **Status:** Planning only. No source code is modified by these documents.
> **Verified against:** the live `slintcn` MCP registry (`npx slintcn@latest`) on 2026-06-05.
> Split out of `docs/slintcn-migration-plan.md` — one step per file.

## Read order

| File | Step | What it covers |
|---|---|---|
| [00-corrections.md](00-corrections.md) | 0 | Where the original research doc's API guesses were wrong (read first) |
| [01-install.md](01-install.md) | 1 | Install slintcn CLI config + vendor components |
| [02-build-rs.md](02-build-rs.md) | 2 | Why `build.rs` is intentionally untouched |
| [03-theme.md](03-theme.md) | 3 | Migrate `ui/theme.slint` color tokens |
| [04-buttons.md](04-buttons.md) | 4 | Migrate `ui/components/buttons.slint` |
| [05-settings-rows.md](05-settings-rows.md) | 5 | Migrate `ui/components/settings_rows.slint` |
| [06-panel-chrome.md](06-panel-chrome.md) | 6 | Migrate `ui/components/panel_chrome.slint` |
| [07-info-banner.md](07-info-banner.md) | 7 | Migrate `ui/components/info_banner.slint` |
| [08-pages-std-widgets.md](08-pages-std-widgets.md) | 8 | Migrate raw `std-widgets` usage in pages |
| [09-page-order.md](09-page-order.md) | 9 | Page-by-page migration order |
| [10-audit.md](10-audit.md) | 10 | Audit + remove leftover std-widgets |
| [11-validate.md](11-validate.md) | 11 | Build + verification checklist |

## Strategy in one paragraph

The custom components (`PrimaryButton`, `SettingsToggleRow`, …) are consumed in **132+ call
sites**. Instead of rewriting every call site, **keep the public component names and property
surfaces** (`label`, `clicked`, `checked`, `toggled`, …) and **re-implement their internals** on
top of slintcn primitives. Call sites change only where a raw `std-widgets` widget is used
directly in a page (e.g. `LineEdit`, `ScrollView`).

## Scope (measured in the current tree)

```
std-widgets imports:   33 files
LineEdit:              28   → Input
ScrollView:            52   → ScrollArea  (+ content-height — riskiest)
Spinner:                6   → keep std Spinner OR Skeleton (NOT Progress)
CheckBox:               4   → Checkbox / Switch
Slider:                 4   → Slider
VerticalBox:            7   → VerticalLayout (+ padding/spacing)
PrimaryButton:         34   → Button (ButtonVariant.default)
TextButton:            38   → Button (ButtonVariant.ghost)
DestructiveButton:     25   → Button (ButtonVariant.destructive)
LoadingView:            6   → keep (internally swap Spinner)
```

**Do not touch:** `ui/components/std/`, `ui/components/mcore/`, `VENDORING.md` — a separately
vendored Slint helper set with its own pinned upstream commit.
