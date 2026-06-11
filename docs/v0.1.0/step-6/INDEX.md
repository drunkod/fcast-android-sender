# STEP-6 — RTMP settings page (sub-steps)

> Split of the original STEP-6 into four self-contained sub-steps.
> **Moblin analogue:** `View/Settings/Streams/Stream/Rtmp/StreamRtmpSettingsView.swift`
> **Depends on:** [STEP-5A](../step-5/STEP-5A-panel-enum.md) (`Panel.protocol-rtmp-settings`).

---

## Sub-step map

| # | File | Scope | slintcn used |
|---|------|-------|--------------|
| 6A | [STEP-6A-page-scaffold.md](STEP-6A-page-scaffold.md) | Imports, component shell, FocusScope, PanelHeader, Flickable body | `Card` (referenced) |
| 6B | [STEP-6B-connection-section.md](STEP-6B-connection-section.md) | Server URL row + stream-key reveal/hide row | `Card` |
| 6C | [STEP-6C-reliability-performance.md](STEP-6C-reliability-performance.md) | Reconnect `Switch` + disabled adaptive-bitrate `Switch` | `Switch`, `Card` |
| 6D | [STEP-6D-status-and-registration.md](STEP-6D-status-and-registration.md) | Live-status `Badge` + Stop `Button` + `main.slint` registration + verify | `Badge`, `Button`, `Card` |

Single file created (6A–6D together): `ui/pages/protocol_rtmp_settings_page.slint`.

---

## slintcn components used (verified via slintcn registry)

All present in `ui/slintcn/components/`. Per the registry:

| Component | Import | API used here |
|---|---|---|
| `Switch` | `../slintcn/components/switch.slint` | `checked <=> bool`, `disabled` |
| `Badge` | `../slintcn/components/badge.slint` | `text`, `variant: BadgeVariant.{default,secondary}` |
| `Button` | `../slintcn/components/button.slint` | `text`, `variant: ButtonVariant.destructive`, `size: ButtonSize.lg`, `clicked` |
| `Card` | `../slintcn/components/card.slint` | `variant: CardVariant.solid`, `card-padding: CardPadding.none` |

Plus `SettingsValueRow` + `RowColors` from `../components/settings_rows.slint`.

> **Why some rows are hand-built `Rectangle`+`TouchArea` instead of slintcn?**
> The stream-key reveal row and the Switch rows need a label + trailing control
> in a settings-row frame. slintcn has no "settings row" primitive, so we
> compose a thin `Rectangle` (using `RowColors` for the shared press/normal
> hex) and drop the slintcn `Switch`/`Badge` into the trailing slot. The list
> *frame* is the slintcn `Card`; the *controls* are slintcn widgets.

---

## Build order

The four sub-steps assemble one file top-to-bottom: 6A is the shell, 6B–6D fill
the `body` VerticalLayout in order. The complete file appears in
[STEP-6D](STEP-6D-status-and-registration.md).

→ Next top-level step: [../step-7/INDEX.md](../step-7/INDEX.md)
