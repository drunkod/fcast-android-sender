# STEP-8C — Receiver list

> The populated list, shown when `Bridge.devices` has entries. A slintcn `Card`
> wrapping a `for` loop of tappable receiver rows, each with a `Badge` for the
> default receiver. Append after the empty state (8B).

---

## Goal

Render each `ReceiverItem` as a tappable row (icon · name+address · chevron)
that fires `Bridge.connect-receiver(device.id)`, with a "Default" `Badge` on the
default receiver.

---

## The change — append after the empty state

**File:** `ui/pages/connect_page.slint`

```slint
        // ── Receiver list ─────────────────────────────────────────────────────
        if Bridge.devices.length > 0: VerticalLayout {
            spacing: Theme.spacing-default;

            Text {
                text: @tr("AVAILABLE RECEIVERS");
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
                    for device[idx] in Bridge.devices: Rectangle {
                        height: 64px;
                        background: row-ta.pressed ? RowColors.pressed : RowColors.normal;

                        row-ta := TouchArea {
                            clicked => { Bridge.connect-receiver(device.id); }

                            HorizontalLayout {
                                padding-left: Theme.padding-screen;
                                padding-right: Theme.padding-screen;
                                spacing: Theme.spacing-default;
                                alignment: stretch;

                                // ── Icon ─────────────────────────────────────
                                Text {
                                    text: device.kind == "fcast" ? "📺" : "🖥";
                                    font-size: 20pt;
                                    vertical-alignment: center;
                                }

                                // ── Name + address ────────────────────────────
                                VerticalLayout {
                                    alignment: center;
                                    horizontal-stretch: 1;
                                    spacing: 2px;

                                    HorizontalLayout {
                                        spacing: Theme.spacing-tight;
                                        alignment: start;
                                        Text {
                                            text: device.name;
                                            color: Theme.text-primary;
                                            font-size: Theme.font-size-body;
                                            font-weight: 600;
                                            vertical-alignment: center;
                                        }
                                        // slintcn Badge — compact "Default" tag.
                                        if device.is-default: Badge {
                                            text: @tr("Default");
                                            variant: BadgeVariant.secondary;
                                            size: BadgeSize.sm;
                                        }
                                    }

                                    Text {
                                        // Slint has no implicit int→string coercion in `+`;
                                        // interpolate the port instead of concatenating it.
                                        text: device.address != ""
                                            ? device.address
                                            : "\{device.ip}:\{device.port}";
                                        color: Theme.text-secondary;
                                        font-size: Theme.font-size-label;
                                    }
                                }

                                // ── Chevron ───────────────────────────────────
                                Text {
                                    text: "›";
                                    color: Theme.text-disabled;
                                    font-size: 18pt;
                                    vertical-alignment: center;
                                }
                            }
                        }
                    }
                }
            }
        }
```

---

## Key details

| Detail | Why |
|---|---|
| `Card { card-padding: none } + VerticalLayout { spacing: 1px }` | rounded group frame with hairline dividers between rows (same idiom as STEP-6/7) |
| whole-row `TouchArea` → `Bridge.connect-receiver(device.id)` | tapping anywhere on the row connects |
| `RowColors.pressed / .normal` | shared press feedback hex, not hardcoded |
| `BadgeSize.sm` | compact tag that fits inline beside the name (requires the `BadgeSize` import from 8A) |
| `"\{device.ip}:\{device.port}"` | **interpolation, not `+`** — `port` is an `int`; Slint `+` does not coerce int→string |
| `device.kind == "fcast" ? "📺" : "🖥"` | simple per-kind glyph; extend as more receiver kinds appear |

> **`for device[idx] in Bridge.devices`** binds `device` (the item) and `idx`
> (position). `idx` is unused here but kept for parity with other list pages and
> in case row striping is added later.

---

## Next

→ [STEP-8D-manual-button-and-done.md](STEP-8D-manual-button-and-done.md)
