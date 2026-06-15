# STEP-6D — Live status, Stop button, registration

> Final section + the `main.slint` wiring + verification. Adds the conditional
> status block (slintcn `Badge` + `Button`) and shows the complete assembled
> file.

---

## Goal

Append the live-status section (only visible while a stream is active), the
Stop `Button`, register the page in `main.slint`, and verify the build.

---

## The change — append to `body`

**File:** `ui/pages/protocol_rtmp_settings_page.slint` (inside `body`, after 6C)

```slint
                    // ── LIVE STATUS (only when stream is active) ─────────────
                    if Bridge.cam-rtmp-state != MixerState.idle: VerticalLayout {
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
                                        // slintcn Badge — green/default when live,
                                        // neutral/secondary while connecting.
                                        Badge {
                                            text: Bridge.cam-rtmp-state == MixerState.running
                                                ? @tr("Live")
                                                : @tr("Connecting…");
                                            variant: Bridge.cam-rtmp-state == MixerState.running
                                                ? BadgeVariant.default
                                                : BadgeVariant.secondary;
                                        }
                                    }
                                }
                                if Bridge.cam-rtmp-error-text != "": SettingsValueRow {
                                    title: @tr("Error");
                                    value: Bridge.cam-rtmp-error-text;
                                    show-chevron: false;
                                }
                            }
                        }

                        // slintcn Button — destructive Stop, only while running.
                        if Bridge.cam-rtmp-state == MixerState.running: Button {
                            text: @tr("Stop Stream");
                            variant: ButtonVariant.destructive;
                            size: ButtonSize.lg;
                            clicked => { Bridge.stop-camera-rtmp-stream(); }
                        }
                    }
```

This closes the `body`, the `Flickable`, the `VerticalLayout`, the `FocusScope`,
and the component — the file from 6A now ends with the matching `}` braces.

---

## Registration in `main.slint`

```slint
import { ProtocolRtmpSettingsPage } from "pages/protocol_rtmp_settings_page.slint";

// Inside the panel switch/if chain (next to the other page routes):
if Bridge.active-panel == Panel.protocol-rtmp-settings : ProtocolRtmpSettingsPage { }
```

To open it from elsewhere (e.g. a streams list row):

```slint
PanelBridge.push(Panel.protocol-rtmp-settings);
```

---

## Complete file (6A + 6B + 6C + 6D)

> The full page is the concatenation of the snippets in order: 6A's shell with
> 6B's CONNECTION card, 6C's RELIABILITY + PERFORMANCE cards, and 6D's STREAM
> STATUS block all inside `body`. No code is duplicated across sub-steps — each
> appends the next sibling into `body`.

---

## Verification

```bash
slint-lsp ui/main.slint 2>&1 | grep error
# → (none)

./gradlew assembleDebug 2>&1 | tail -20
# → BUILD SUCCESSFUL
```

Manual walkthrough:
1. Stream key masked (`●●●●●●●●`) by default; tapping the row reveals it and
   flips Show ⇄ Hide.
2. Server URL row shows `cam-rtmp-url` (or "Not set") and opens the
   camera-RTMP panel on tap.
3. Reconnect `Switch` toggles and holds; adaptive-bitrate row is dimmed and
   unresponsive.
4. With `cam-rtmp-state != idle`, the STREAM STATUS card appears; the `Badge`
   reads "Live" (running) or "Connecting…"; the Stop `Button` shows only when
   running and fires `Bridge.stop-camera-rtmp-stream()`.

---

## Done — STEP-6 complete

| Sub-step | Status |
|---|---|
| 6A page scaffold | ✅ |
| 6B connection section | ✅ |
| 6C reliability + performance | ✅ |
| 6D status + registration | ✅ |

→ Next top-level step: [../step-7/INDEX.md](../step-7/INDEX.md)
