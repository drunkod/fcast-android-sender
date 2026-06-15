# STEP-7A — `CyclerRow` + page scaffold

> The in-file `CyclerRow` component (Select replacement) + the page shell
> (imports, local state, FocusScope, PanelHeader, Flickable `body`).

---

## Goal

Define the dependency-free `CyclerRow` and stand up
`protocol_srt_settings_page.slint` with all imports and the empty `body` that
7B–7E fill.

---

## Encryption index → `pbkeylen` contract (carry-over from STEP-1C / STEP-5C)

The encryption cycler (7C) writes its index into
`Bridge.srt-destination-pbkeylen-idx`. The Rust `start-srt-destination` handler
maps it to the `pbkeylen` byte count the SRT pipeline (STEP-3) expects:

| idx | Label | `pbkeylen` sent to `srtsink` |
|---|---|---|
| 0 | None | `None` (omit `passphrase` + `pbkeylen`) |
| 1 | AES-128 | `Some(16)` |
| 2 | AES-192 | `Some(24)` |
| 3 | AES-256 | `Some(32)` |

```rust
// Rust start-srt-destination handler (reference):
let pbkeylen = match bridge.get_srt_destination_pbkeylen_idx() {
    1 => Some(16),
    2 => Some(24),
    3 => Some(32),
    _ => None, // 0 = None
};
```

---

## The change — `CyclerRow` + shell

**Create:** `ui/pages/protocol_srt_settings_page.slint`

```slint
// ui/pages/protocol_srt_settings_page.slint
// Moblin analogue: View/Settings/Streams/Stream/Srt/StreamSrtSettingsView.swift
import { Theme } from "../theme.slint";
import { Bridge, Panel, MixerState } from "../bridge.slint";
import { PanelBridge } from "../state/panel_bridge.slint";
import { PanelHeader } from "../components/panel_chrome.slint";
import {
    SettingsValueRow,
    RowColors,
} from "../components/settings_rows.slint";
import { Input } from "../slintcn/components/input.slint";
import { Switch } from "../slintcn/components/switch.slint";
import { Badge, BadgeVariant } from "../slintcn/components/badge.slint";
import { Button, ButtonVariant, ButtonSize } from "../slintcn/components/button.slint";
import { Card, CardVariant, CardPadding } from "../slintcn/components/card.slint";

// ── CyclerRow ────────────────────────────────────────────────────────────────
// Dependency-free Picker stand-in: tap the row to advance through `options`,
// wrapping at the end. Emits `changed(new-idx)`. No PopupWindow, no lucide.
component CyclerRow inherits Rectangle {
    in property <string> title;
    in property <[string]> options;
    in-out property <int> idx: 0;
    callback changed(int);

    height: Theme.row-height;
    background: ta.pressed ? RowColors.pressed : RowColors.normal;

    // Advance to the next option, wrapping at the end (matches the inline
    // cycler idiom in recording_page.slint).
    function advance() {
        root.idx = Math.mod(root.idx + 1, root.options.length);
        root.changed(root.idx);
    }

    accessible-role: button;
    accessible-label: root.title + ", " + root.options[root.idx];
    accessible-action-default => { root.advance(); }

    ta := TouchArea {
        clicked => { root.advance(); }
        HorizontalLayout {
            padding-left: Theme.padding-screen;
            padding-right: Theme.padding-screen;
            spacing: Theme.spacing-default;
            alignment: stretch;
            Text {
                text: root.title;
                color: Theme.text-primary;
                font-size: Theme.font-size-body;
                vertical-alignment: center;
                horizontal-stretch: 1;
            }
            Text {
                text: root.options[root.idx];
                color: Theme.text-secondary;
                font-size: Theme.font-size-body;
                vertical-alignment: center;
            }
            Text {
                text: "›";
                color: Theme.text-disabled;
                font-size: Theme.font-size-body;
                vertical-alignment: center;
            }
        }
    }
}

export component ProtocolSrtSettingsPage inherits Rectangle {
    width: 100%;
    height: 100%;
    background: transparent;

    // ── Local draft state ────────────────────────────────────────────────────
    in-out property <string> draft-uri:        Bridge.srt-destination.uri;
    in-out property <int>    latency-idx:       1;   // default → 200 ms
    in-out property <int>    encryption-idx:    0;   // default → None
    in-out property <bool>   show-passphrase:   false;
    in-out property <string> draft-passphrase:  Bridge.srt-destination-passphrase;
    in-out property <bool>   big-packets:       true;

    // Latency labels + parallel raw ms values (cycler index → real ms value).
    property <[string]> latency-labels:    ["120 ms", "200 ms", "500 ms", "1000 ms", "2000 ms", "4000 ms"];
    property <[int]>    latency-ms-values: [120, 200, 500, 1000, 2000, 4000];

    property <[string]> encryption-labels: ["None", "AES-128", "AES-192", "AES-256"];

    forward-focus: scope;
    scope := FocusScope {
        key-pressed(event) => {
            if (event.text == Key.Escape) { PanelBridge.pop(); return accept; }
            return reject;
        }

        VerticalLayout {
            PanelHeader {
                title: @tr("SRT");
                close-clicked => { PanelBridge.pop(); }
            }

            Flickable {
                vertical-stretch: 1;
                viewport-height: body.preferred-height;

                body := VerticalLayout {
                    padding: Theme.padding-screen;
                    spacing: Theme.spacing-loose;
                    alignment: start;

                    // 7B → CONNECTION
                    // 7C → TRANSPORT
                    // 7D → BITRATE + SAVE
                    // 7E → STREAM STATUS + PIPELINE
                }
            }
        }
    }
}
```

---

## CyclerRow design notes

- **`function advance()`** centralises the cycle so both the `TouchArea.clicked`
  and the `accessible-action-default` (screen-reader / keyboard) reuse it —
  no duplicated `Math.mod` and no fragile cross-element callback invocation.
- **Parallel label/value arrays** (7C's latency) keep the displayed label
  (`"200 ms"`) separate from the value pushed to the Bridge (`200`). The
  cycler reports an index; the page maps `index → latency-ms-values[index]`.
- **`Math.mod`** is the same wrap idiom used in `recording_page.slint:159`.

---

## Next

→ [STEP-7B-connection-section.md](STEP-7B-connection-section.md)
