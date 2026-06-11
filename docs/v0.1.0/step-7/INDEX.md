# STEP-7 — SRT settings page (sub-steps)

> Split of the original STEP-7 (largest UI step) into five self-contained
> sub-steps.
> **Moblin analogue:** `View/Settings/Streams/Stream/Srt/StreamSrtSettingsView.swift`
> **Depends on:** [STEP-5](../step-5/INDEX.md) (`Panel.protocol-srt-settings`,
> `SrtDestination`, `srt-destination-*` properties).

---

## Sub-step map

| # | File | Scope | slintcn used |
|---|------|-------|--------------|
| 7A | [STEP-7A-cyclerrow-and-scaffold.md](STEP-7A-cyclerrow-and-scaffold.md) | In-file `CyclerRow` + imports + shell + local state + encryption→pbkeylen contract | — (CyclerRow is hand-built) |
| 7B | [STEP-7B-connection-section.md](STEP-7B-connection-section.md) | SRT URL `Input` | `Input` |
| 7C | [STEP-7C-transport-section.md](STEP-7C-transport-section.md) | Latency cycler, encryption cycler, passphrase `Input`, big-packets `Switch` | `Input`, `Switch` |
| 7D | [STEP-7D-bitrate-and-save.md](STEP-7D-bitrate-and-save.md) | Disabled adaptive-bitrate `Switch` + Save `Button` | `Switch`, `Button` |
| 7E | [STEP-7E-status-pipeline-registration.md](STEP-7E-status-pipeline-registration.md) | Live-status `Badge`/`Button` + Pipeline reference + `main.slint` reg + verify | `Badge`, `Button`, `Card` |

Single file created (7A–7E together): `ui/pages/protocol_srt_settings_page.slint`.

---

## Why no `Select` (the one slintcn deviation)

`ui/slintcn/components/` does **not** ship `select.slint`, and adding it drags
`lucide-paths` + a `PopupWindow`. For two short fixed lists (latency,
encryption) the dependency-free in-file **`CyclerRow`** (`(idx+1) mod len` on
tap) is lighter and matches the existing cycler idiom in `recording_page.slint`.
So STEP-7 uses slintcn `Input` / `Switch` / `Badge` / `Button` / `Card` (all
installed) **plus** a local `CyclerRow`.

| Component | Import | API used |
|---|---|---|
| `Input` | `../slintcn/components/input.slint` | `text <=> string`, `placeholder`, `password: bool`, `edited(string)` |
| `Switch` | `../slintcn/components/switch.slint` | `checked <=> bool`, `disabled` |
| `Badge` | `../slintcn/components/badge.slint` | `text`, `variant: BadgeVariant.{default,secondary}` |
| `Button` | `../slintcn/components/button.slint` | `text`, `variant: ButtonVariant.{default,destructive}`, `size: ButtonSize.lg`, `clicked` |
| `Card` | `../slintcn/components/card.slint` | `variant: CardVariant.solid`, `card-padding: CardPadding.none` |

---

## Build order

The sub-steps assemble one file top-to-bottom: 7A defines `CyclerRow` + the
shell with the `body` container; 7B–7E append sections into `body` in order.

→ Next top-level step: [../step-8/INDEX.md](../step-8/INDEX.md)
