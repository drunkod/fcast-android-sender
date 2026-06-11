# STEP-6C — Reliability + Performance sections

> Two more `Card` sections in `body`: a **Reconnect** `Switch` (interactive)
> and a **disabled** Adaptive-bitrate `Switch` (v0.2.0 placeholder).

---

## Goal

Demonstrate the slintcn `Switch` in both its interactive and disabled forms
inside settings rows.

---

## Moblin → FCast mapping

| Moblin field | FCast equivalent | State |
|---|---|---|
| Reconnect on disconnect | `reconnect-on-disconnect` (local) | interactive |
| Adaptive bitrate toggle | `adaptive-bitrate` (local) | disabled — v0.2.0 |

---

## slintcn `Switch` API (from registry)

```slint
import { Switch } from "slintcn/components/switch.slint";
Switch { label: "Dark mode"; checked <=> dark; }
```

Key points for this step:
- **`checked <=> root.x`** — use the **two-way** bind. The installed `Switch`
  mutates its own `checked` in its `clicked` handler, so a one-way `checked:`
  binding would fight that assignment.
- **`disabled: true`** dims the control and suppresses its internal mutation —
  so a one-way `checked:` is *safe* on a disabled switch (the adaptive-bitrate
  placeholder below relies on this).

---

## The change — append to `body`

**File:** `ui/pages/protocol_rtmp_settings_page.slint` (inside `body`, after 6B)

```slint
                    // ── RELIABILITY ──────────────────────────────────────────
                    Text {
                        text: @tr("RELIABILITY");
                        color: Theme.text-secondary;
                        font-size: Theme.font-size-label;
                        font-weight: 600;
                    }
                    Card {
                        variant: CardVariant.solid;
                        card-padding: CardPadding.none;
                        clip: true;
                        VerticalLayout {
                            spacing: 1px;
                            Rectangle {
                                height: Theme.row-height;
                                background: RowColors.normal;
                                HorizontalLayout {
                                    padding-left: Theme.padding-screen;
                                    padding-right: Theme.padding-screen;
                                    alignment: stretch;
                                    Text {
                                        text: @tr("Reconnect on disconnect");
                                        color: Theme.text-primary;
                                        font-size: Theme.font-size-body;
                                        vertical-alignment: center;
                                        horizontal-stretch: 1;
                                    }
                                    // Two-way: the Switch mutates its own `checked`.
                                    Switch {
                                        checked <=> root.reconnect-on-disconnect;
                                    }
                                }
                            }
                        }
                    }

                    // ── PERFORMANCE ──────────────────────────────────────────
                    Text {
                        text: @tr("PERFORMANCE");
                        color: Theme.text-secondary;
                        font-size: Theme.font-size-label;
                        font-weight: 600;
                    }
                    Card {
                        variant: CardVariant.solid;
                        card-padding: CardPadding.none;
                        clip: true;
                        VerticalLayout {
                            spacing: 1px;
                            // Adaptive bitrate — disabled, planned for v0.2.0.
                            Rectangle {
                                height: Theme.row-height;
                                background: RowColors.normal;
                                opacity: 0.45;
                                HorizontalLayout {
                                    padding-left: Theme.padding-screen;
                                    padding-right: Theme.padding-screen;
                                    alignment: stretch;
                                    VerticalLayout {
                                        alignment: center;
                                        horizontal-stretch: 1;
                                        Text {
                                            text: @tr("Adaptive bitrate");
                                            color: Theme.text-primary;
                                            font-size: Theme.font-size-body;
                                        }
                                        Text {
                                            text: @tr("Available in v0.2.0");
                                            color: Theme.text-disabled;
                                            font-size: Theme.font-size-label;
                                        }
                                    }
                                    // Disabled → one-way `checked:` is safe (no self-mutation).
                                    Switch {
                                        checked: root.adaptive-bitrate;
                                        disabled: true;
                                    }
                                }
                            }
                        }
                    }
```

---

## Verification

Renders after 6B; full compile check in 6D.

Manual: the reconnect switch toggles and holds; the adaptive-bitrate row is
dimmed (`opacity: 0.45`) and its switch does not respond to taps.

---

## Next

→ [STEP-6D-status-and-registration.md](STEP-6D-status-and-registration.md)
