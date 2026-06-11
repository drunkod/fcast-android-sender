# STEP-7C — Transport section

> The core SRT controls: latency `CyclerRow`, encryption `CyclerRow`,
> conditional passphrase `Input`, and the big-packets `Switch`.

---

## Goal

A `Card` holding four rows. The two cyclers write to the Bridge (latency → ms
value; encryption → `pbkeylen-idx`); the passphrase `Input` appears only when
encryption ≠ None; the big-packets `Switch` is interactive.

---

## The change — append to `body`

**File:** `ui/pages/protocol_srt_settings_page.slint` (inside `body`, after 7B)

```slint
                    // ── TRANSPORT ────────────────────────────────────────────
                    Text {
                        text: @tr("TRANSPORT");
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

                            // Latency cycler — writes the resolved ms value to the Bridge.
                            CyclerRow {
                                title: @tr("Latency");
                                options: root.latency-labels;
                                idx <=> root.latency-idx;
                                changed(i) => {
                                    Bridge.srt-destination.latency-ms =
                                        root.latency-ms-values[i];
                                }
                            }

                            // Encryption cycler — writes idx to the Bridge so Rust can
                            // map it to pbkeylen {None,16,24,32} (see 7A contract).
                            CyclerRow {
                                title: @tr("Encryption");
                                options: root.encryption-labels;
                                idx <=> root.encryption-idx;
                                changed(i) => {
                                    Bridge.srt-destination-pbkeylen-idx = i;
                                }
                            }

                            // Passphrase — only when encryption is active.
                            if root.encryption-idx > 0: Rectangle {
                                height: 80px;
                                background: RowColors.normal;
                                VerticalLayout {
                                    padding: Theme.padding-screen;
                                    spacing: 4px;
                                    alignment: center;
                                    HorizontalLayout {
                                        spacing: Theme.spacing-default;
                                        Text {
                                            text: @tr("Passphrase");
                                            color: Theme.text-secondary;
                                            font-size: Theme.font-size-label;
                                            vertical-alignment: center;
                                            horizontal-stretch: 1;
                                        }
                                        TouchArea {
                                            width: 40px;
                                            height: 20px;
                                            clicked => {
                                                root.show-passphrase = !root.show-passphrase;
                                            }
                                            Text {
                                                text: root.show-passphrase
                                                    ? @tr("Hide") : @tr("Show");
                                                color: Theme.accent;
                                                font-size: Theme.font-size-label;
                                                vertical-alignment: center;
                                            }
                                        }
                                    }
                                    // slintcn Input in password mode (toggled by Show/Hide).
                                    Input {
                                        text <=> root.draft-passphrase;
                                        placeholder: @tr("10–79 characters");
                                        password: !root.show-passphrase;
                                        edited(v) => {
                                            root.draft-passphrase = v;
                                            Bridge.srt-destination-passphrase = v;
                                        }
                                    }
                                }
                            }

                            // Big packets toggle.
                            // "7 MPEG-TS packets per SRT packet (6 otherwise).
                            //  Some Android hotspots fail with big packets." — Moblin docs.
                            Rectangle {
                                height: Theme.row-height;
                                background: RowColors.normal;
                                HorizontalLayout {
                                    padding-left: Theme.padding-screen;
                                    padding-right: Theme.padding-screen;
                                    alignment: stretch;
                                    VerticalLayout {
                                        alignment: center;
                                        horizontal-stretch: 1;
                                        Text {
                                            text: @tr("Big packets");
                                            color: Theme.text-primary;
                                            font-size: Theme.font-size-body;
                                        }
                                        Text {
                                            text: @tr("7 TS packets per SRT packet; disable if hotspot drops packets");
                                            color: Theme.text-disabled;
                                            font-size: Theme.font-size-label;
                                        }
                                    }
                                    // Two-way bind — slintcn Switch mutates its own `checked`.
                                    Switch {
                                        checked <=> root.big-packets;
                                    }
                                }
                            }
                        }
                    }
```

---

## Behaviour notes

| Control | Writes to | When |
|---|---|---|
| Latency cycler | `Bridge.srt-destination.latency-ms` | on each tap (via `latency-ms-values[i]`) |
| Encryption cycler | `Bridge.srt-destination-pbkeylen-idx` | on each tap (idx 0–3) |
| Passphrase `Input` | `Bridge.srt-destination-passphrase` | on each keystroke |
| Big-packets `Switch` | `root.big-packets` (page-local) | on toggle |

- **Passphrase visibility** is gated by `if root.encryption-idx > 0` — the whole
  row only exists when encryption is selected, so there is no orphan secret
  field when encryption is `None`.
- **`password: !root.show-passphrase`** flips the `Input` between masked and
  clear text via the Show/Hide affordance.
- **`big-packets`** is page-local in v0.1.0 (it maps to an MPEG-TS payload-size
  tuning that the Rust handler can read when wiring; not yet a Bridge property
  — promote it in v0.2.0 if persisted).

---

## Next

→ [STEP-7D-bitrate-and-save.md](STEP-7D-bitrate-and-save.md)
