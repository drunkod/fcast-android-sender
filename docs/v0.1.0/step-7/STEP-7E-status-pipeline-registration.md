# STEP-7E — Live status, pipeline reference, registration

> Final sections + `main.slint` wiring + verification. Adds the conditional
> status block (`Badge` + Stop `Button`), the debug Pipeline-reference card,
> and closes the file.

---

## The change — append to `body`

**File:** `ui/pages/protocol_srt_settings_page.slint` (inside `body`, after 7D)

```slint
                    // ── LIVE STATUS ──────────────────────────────────────────
                    if Bridge.srt-destination.state != MixerState.idle: VerticalLayout {
                        spacing: Theme.spacing-default;

                        Text {
                            text: @tr("STREAM STATUS");
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
                                            text: @tr("State");
                                            color: Theme.text-primary;
                                            font-size: Theme.font-size-body;
                                            vertical-alignment: center;
                                            horizontal-stretch: 1;
                                        }
                                        Badge {
                                            text: Bridge.srt-destination.state == MixerState.running
                                                ? @tr("Live")
                                                : @tr("Connecting…");
                                            variant: Bridge.srt-destination.state == MixerState.running
                                                ? BadgeVariant.default
                                                : BadgeVariant.secondary;
                                        }
                                    }
                                }
                                if Bridge.srt-destination.last-error != "": SettingsValueRow {
                                    title: @tr("Error");
                                    value: Bridge.srt-destination.last-error;
                                    show-chevron: false;
                                }
                            }
                        }

                        if Bridge.srt-destination.state == MixerState.running: Button {
                            text: @tr("Stop Stream");
                            variant: ButtonVariant.destructive;
                            size: ButtonSize.lg;
                            clicked => { Bridge.stop-srt-destination(); }
                        }
                    }

                    // ── PIPELINE REFERENCE (debug aid) ───────────────────────
                    Text {
                        text: @tr("PIPELINE");
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
                            SettingsValueRow {
                                title: @tr("Video");
                                value: "appsrc → videoconvert → h264enc → h264parse → mpegtsmux → srtsink";
                                show-chevron: false;
                            }
                            SettingsValueRow {
                                title: @tr("Audio");
                                value: "appsrc → audioconvert → audioresample → avenc_aac → mpegtsmux";
                                show-chevron: false;
                            }
                        }
                    }
```

After this block the `body`, `Flickable`, `VerticalLayout`, `FocusScope`, and
component all close with their matching `}` braces (opened in 7A).

> The Pipeline-reference card is the read-only UI echo of STEP-2's element
> inventory — see [../step-2/INDEX.md](../step-2/INDEX.md). It is a static
> debug aid (hardcoded strings), not driven by live `getinfo`.

---

## Registration in `main.slint`

```slint
import { ProtocolSrtSettingsPage } from "pages/protocol_srt_settings_page.slint";

// Inside the panel switch/if chain:
if Bridge.active-panel == Panel.protocol-srt-settings : ProtocolSrtSettingsPage { }
```

Open from elsewhere:

```slint
PanelBridge.push(Panel.protocol-srt-settings);
```

---

## Complete file

The full page = 7A's `CyclerRow` + shell, with 7B (CONNECTION), 7C (TRANSPORT),
7D (BITRATE + SAVE), and 7E (STREAM STATUS + PIPELINE) appended into `body` in
order. No snippet is duplicated across sub-steps.

---

## Verification

```bash
slint-lsp ui/main.slint 2>&1 | grep error
# → (none)
```

Manual checks:
1. Passphrase row hidden when encryption is `None`, visible for AES-*.
2. Latency cycler wraps through all 6 options; each tap sets
   `Bridge.srt-destination.latency-ms` to the matching ms value.
3. Encryption cycler wraps through 4 options and writes
   `Bridge.srt-destination-pbkeylen-idx` (Rust maps it to {None,16,24,32}).
4. Save commits `draft-uri` into `Bridge.srt-destination.uri` and fires
   `Bridge.save-srt-destination-config()`.
5. `Big packets` toggles and holds.
6. With `srt-destination.state == running`, the Live `Badge` shows and the Stop
   `Button` fires `Bridge.stop-srt-destination()`.

---

## Done — STEP-7 complete

| Sub-step | Status |
|---|---|
| 7A CyclerRow + scaffold | ✅ |
| 7B connection (Input) | ✅ |
| 7C transport (cyclers + Input + Switch) | ✅ |
| 7D bitrate + save | ✅ |
| 7E status + pipeline + registration | ✅ |

→ Next top-level step: [../step-8/INDEX.md](../step-8/INDEX.md)
