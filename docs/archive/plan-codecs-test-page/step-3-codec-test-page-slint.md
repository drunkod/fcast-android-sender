# Step 3 — `ui/pages/codec_test_page.slint` (full replacement, slintcn)

← [Step 2](step-2-bridge-slint.md) · [Index](README.md) · Next → [Step 4](step-4-codec-test-rs.md)

Differences from the current stub and from the raw research snippet:

- **slintcn `ScrollArea`** (not std `ScrollView`) with `content-height` bound to the
  log's `preferred-height` — same pattern the current stub already uses and that
  `test_functionality_page.slint` uses.
- slintcn-backed **`PrimaryButton`** wrappers (48px targets + a11y).
- slintcn **`Badge`** as a running/idle status chip, **`Separator`** under the buttons.
- **`FocusScope`** with Escape → `PanelBridge.pop()`, matching migrated pages.
- Log `Text` binds to `Bridge.codec-test-log`, with an empty-state placeholder.

```slint
// codec_test_page.slint — MediaCodec probe panel (slintcn).
//
// Reachable from FullSettingsPage's "H.264 encoder test" row, which sets
// `PanelBridge.push(Panel.codec-test)`. Buttons invoke Bridge callbacks wired
// in src/android_main.rs; the log streams back via Bridge.codec-test-log.
import { ScrollArea } from "../slintcn/components/scroll-area.slint";
import { Badge, BadgeVariant } from "../slintcn/components/badge.slint";
import { Separator } from "../slintcn/components/separator.slint";

import { Bridge, Panel } from "../bridge.slint";
import { PanelBridge } from "../state/panel_bridge.slint";
import { Theme } from "../theme.slint";
import { PrimaryButton, DestructiveButton } from "../components/buttons.slint";
import { PanelHeader } from "../components/panel_chrome.slint";

export component CodecTestPage inherits Rectangle {
    width: 100%;
    height: 100%;
    background: Theme.surface-primary;

    forward-focus: panel-scope;
    panel-scope := FocusScope {
        key-pressed(event) => {
            if event.text == Key.Escape { PanelBridge.pop(); return accept; }
            return reject;
        }

        VerticalLayout {
            // ── Header ─────────────────────────────────────────────────
            PanelHeader {
                title: @tr("Codec test");
                close-clicked => { PanelBridge.pop(); }
            }

            // ── Body ───────────────────────────────────────────────────
            VerticalLayout {
                padding: Theme.padding-screen;
                spacing: Theme.spacing-default;

                // Status chip — running / idle.
                HorizontalLayout {
                    spacing: Theme.spacing-default;
                    alignment: start;
                    Badge {
                        text: Bridge.codec-test-running ? @tr("running") : @tr("idle");
                        variant: Bridge.codec-test-running
                            ? BadgeVariant.default
                            : BadgeVariant.secondary;
                    }
                }

                // Primary actions row.
                HorizontalLayout {
                    spacing: Theme.spacing-default;

                    PrimaryButton {
                        label: Bridge.codec-test-running
                            ? @tr("Running…")
                            : @tr("Run full codec test");
                        enabled: !Bridge.codec-test-running;
                        clicked => { Bridge.run-codec-test(); }
                        horizontal-stretch: 1;
                    }
                    PrimaryButton {
                        label: @tr("Dump codecs only");
                        enabled: !Bridge.codec-test-running;
                        clicked => { Bridge.run-codec-dump-only(); }
                        horizontal-stretch: 1;
                    }
                }

                PrimaryButton {
                    label: @tr("Smoke test encoders");
                    enabled: !Bridge.codec-test-running;
                    clicked => { Bridge.run-codec-smoke-only(); }
                }

                Separator { }

                // ── Log output (slintcn ScrollArea) ────────────────────
                ScrollArea {
                    content-height: log.preferred-height;
                    vertical-stretch: 1;
                    log := Text {
                        width: parent.width;
                        text: Bridge.codec-test-log == ""
                            ? @tr("Press a button above to start.")
                            : Bridge.codec-test-log;
                        color: Theme.text-secondary;
                        font-size: Theme.font-size-label;
                        wrap: word-wrap;
                    }
                }
            }
        }
    }   // end FocusScope
}
```

> Note: `ScrollArea` needs the inner content's width pinned (`width: parent.width`)
> so `preferred-height` reflects wrapped text, otherwise the log won't scroll.

`DestructiveButton` is imported for forward-compat (e.g. a future "Cancel/Stop"
button); drop it from the import if you don't add one.

---

← [Step 2](step-2-bridge-slint.md) · [Index](README.md) · Next → [Step 4](step-4-codec-test-rs.md)
