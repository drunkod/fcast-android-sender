# STEP-7D — Bitrate (disabled) + Save

> The disabled adaptive-bitrate placeholder `Switch` and the Save `Button`
> that commits the draft URI and fires the config callback.

---

## Goal

Add the v0.2.0 adaptive-bitrate placeholder (dimmed, disabled `Switch`) and a
full-width Save `Button`.

---

## The change — append to `body`

**File:** `ui/pages/protocol_srt_settings_page.slint` (inside `body`, after 7C)

```slint
                    // ── ADAPTIVE BITRATE (v0.2.0, disabled) ─────────────────
                    Text {
                        text: @tr("BITRATE");
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
                                            text: @tr("FastIRL algorithm — available in v0.2.0");
                                            color: Theme.text-disabled;
                                            font-size: Theme.font-size-label;
                                        }
                                    }
                                    // Disabled → no self-mutation, no bound state needed.
                                    Switch { disabled: true; }
                                }
                            }
                        }
                    }

                    // ── SAVE ─────────────────────────────────────────────────
                    Button {
                        text: @tr("Save");
                        variant: ButtonVariant.default;
                        size: ButtonSize.lg;
                        clicked => {
                            Bridge.srt-destination.uri = root.draft-uri;
                            Bridge.save-srt-destination-config();
                        }
                    }
```

---

## Save semantics

| What | When |
|---|---|
| `Bridge.srt-destination.uri = root.draft-uri` | committed on Save (the `Input` edits `draft-uri` live; the struct is only updated here) |
| `latency-ms` / `pbkeylen-idx` / `passphrase` | already written live by 7C's cyclers + passphrase Input |
| `Bridge.save-srt-destination-config()` | fires the Rust handler to persist + (optionally) (re)start |

> The URI is intentionally **draft-until-Save** so a half-typed `srt://…`
> doesn't churn the destination on every keystroke. The cyclers/passphrase
> write live because they are discrete, valid-by-construction values.

The disabled adaptive-bitrate `Switch` needs **no** bound property: with
`disabled: true` the component never mutates `checked`, so omitting `checked`
entirely is fine (defaults to `false`).

---

## Next

→ [STEP-7E-status-pipeline-registration.md](STEP-7E-status-pipeline-registration.md)
