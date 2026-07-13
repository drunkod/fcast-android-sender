# STEP-8 — Connect page wiring (sub-steps)

> Split of the original STEP-8 into four self-contained sub-steps.
> **Moblin analogue:** none (FCast-specific connection flow).
> **Depends on:** nothing — `Bridge.devices` + `Bridge.connect-receiver` already
> exist in `ui/bridge.slint`.

---

## Sub-step map

| # | File | Scope | slintcn used |
|---|------|-------|--------------|
| 8A | [STEP-8A-scaffold-and-header.md](STEP-8A-scaffold-and-header.md) | Replace placeholder; imports + `ConnectView` shell + header | — |
| 8B | [STEP-8B-empty-state.md](STEP-8B-empty-state.md) | "No receivers found" empty state | — |
| 8C | [STEP-8C-receiver-list.md](STEP-8C-receiver-list.md) | `Card` + `for` loop receiver rows + `Badge` (Default) | `Card`, `Badge` |
| 8D | [STEP-8D-manual-button-and-done.md](STEP-8D-manual-button-and-done.md) | Manual-connect `Button` + registration + verify + v0.1.0 wrap-up | `Button` |

Single file replaced (8A–8D together): `ui/pages/connect_page.slint`.

---

## Pre-flight (shared)

| Exists (do not re-create) | Location |
|---|---|
| `Bridge.devices: [ReceiverItem]` | `ui/bridge.slint:234` |
| `Bridge.connect-receiver(string)` callback | `ui/bridge.slint:328` |
| `ReceiverItem` struct | `ui/bridge.slint:160` |
| Current placeholder (5 lines) | `ui/pages/connect_page.slint` |

`ReceiverItem` fields available in 8C's `for` loop:

```
id: string · name: string · address: string · ip: string
port: int · kind: string · is-default: bool
```

## slintcn components used (verified present in `ui/slintcn/components/`)

| Component | Import | API used |
|---|---|---|
| `Card` | `../slintcn/components/card.slint` | `variant: CardVariant.solid`, `card-padding: CardPadding.none` |
| `Badge` | `../slintcn/components/badge.slint` | `text`, `variant: BadgeVariant.secondary`, `size: BadgeSize.sm` |
| `Button` | `../slintcn/components/button.slint` | `text`, `variant: ButtonVariant.outline`, `size: ButtonSize.default`, `clicked` |

> The receiver rows are hand-built `Rectangle`+`TouchArea` (using `RowColors`)
> because each row is a custom three-part layout (icon · name+address · chevron)
> with a whole-row tap — slintcn has no list-row primitive. The `Card` is the
> group frame; `Badge`/`Button` are the slintcn widgets dropped in.

---

## Build order

8A replaces the file with the shell; 8B–8D append siblings into the outer
`VerticalLayout`.

→ This is the **final step** — see [STEP-8D](STEP-8D-manual-button-and-done.md)
for the v0.1.0 completion summary.
