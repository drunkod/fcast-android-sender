# STEP-6A — Page scaffold

> The shell of `protocol_rtmp_settings_page.slint`: imports, component
> declaration, local state, FocusScope (Esc-to-pop), PanelHeader, and the
> scrollable `body` container that 6B–6D fill.

---

## Goal

Stand up the page file with all imports and the empty `body` VerticalLayout,
so 6B–6D can append sections into it.

---

## Pre-flight

| Exists (do not re-create) | Location |
|---|---|
| `PanelHeader` (title + close) | `ui/components/panel_chrome.slint` |
| `PanelBridge.pop()` / `.push()` | `ui/state/panel_bridge.slint` |
| `SettingsValueRow`, `RowColors` | `ui/components/settings_rows.slint` |
| `Bridge.cam-rtmp-*`, `Panel`, `MixerState` | `ui/bridge.slint` |
| slintcn `Switch`/`Badge`/`Button`/`Card` | `ui/slintcn/components/` |

---

## The change — file header + shell

**Create:** `ui/pages/protocol_rtmp_settings_page.slint`

```slint
// ui/pages/protocol_rtmp_settings_page.slint
// Moblin analogue: View/Settings/Streams/Stream/Rtmp/StreamRtmpSettingsView.swift
import { Theme } from "../theme.slint";
import { Bridge, Panel, MixerState } from "../bridge.slint";
import { PanelBridge } from "../state/panel_bridge.slint";
import { PanelHeader } from "../components/panel_chrome.slint";
import {
    SettingsValueRow,
    RowColors,
} from "../components/settings_rows.slint";
import { Switch } from "../slintcn/components/switch.slint";
import { Badge, BadgeVariant } from "../slintcn/components/badge.slint";
import { Button, ButtonVariant, ButtonSize } from "../slintcn/components/button.slint";
import { Card, CardVariant, CardPadding } from "../slintcn/components/card.slint";

export component ProtocolRtmpSettingsPage inherits Rectangle {
    width: 100%;
    height: 100%;
    background: transparent;

    // ── Local state ──────────────────────────────────────────────────────────
    in-out property <bool> adaptive-bitrate:        false;
    in-out property <bool> reconnect-on-disconnect: true;
    in-out property <bool> show-stream-key:         false;

    forward-focus: scope;
    scope := FocusScope {
        key-pressed(event) => {
            if (event.text == Key.Escape) { PanelBridge.pop(); return accept; }
            return reject;
        }

        VerticalLayout {
            // ── Header ───────────────────────────────────────────────────────
            PanelHeader {
                title: @tr("RTMP");
                close-clicked => { PanelBridge.pop(); }
            }

            // ── Body (sections appended in 6B–6D) ─────────────────────────────
            Flickable {
                vertical-stretch: 1;
                viewport-height: body.preferred-height;

                body := VerticalLayout {
                    padding: Theme.padding-screen;
                    spacing: Theme.spacing-loose;
                    alignment: start;

                    // 6B → CONNECTION
                    // 6C → RELIABILITY + PERFORMANCE
                    // 6D → STREAM STATUS + Stop button
                }
            }
        }
    }
}
```

---

## Notes

- **`forward-focus: scope`** + a zero-logic `FocusScope` gives Esc-to-pop
  without stealing taps (the `key-pressed` only consumes Escape, rejects the
  rest). This mirrors `rtmp_wizard_page.slint`.
- **`Flickable` + `viewport-height: body.preferred-height`** is the standard
  scroll pattern in this codebase — the body grows, the Flickable scrolls.
- The three local `in-out` bools are page-local UI state (not Bridge), since
  reconnect/adaptive persistence is a v0.2.0 concern.

---

## Verification

At this point the page renders just the header (empty body). Full compile check
comes after 6D assembles the sections.

---

## Next

→ [STEP-6B-connection-section.md](STEP-6B-connection-section.md)
