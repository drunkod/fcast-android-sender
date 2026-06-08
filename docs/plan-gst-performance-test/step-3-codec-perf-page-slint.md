# Step 3 — `ui/pages/codec_perf_page.slint` (new, slintcn)

← [Step 2](step-2-bridge-props.md) · [Index](README.md) · Next → [Step 4](step-4-main-slint-route.md)

New file. Differs from the research snippet: **virtualised `ListView`** over
`perf-test-log-lines` (not a single `Text` in a std `ScrollView`), slintcn
`Badge` status chip + `Separator`, and a `FocusScope` Escape handler — the same
idiom as the fixed `codec_test_page.slint`.

```slint
// codec_perf_page.slint — GStreamer codec pipeline performance benchmark (slintcn).
//
// Runs real GStreamer encode/decode pipelines (androidmedia AMC elements),
// counts buffers at fakesink, reports throughput in FPS. Pure-Rust benchmark
// via the gst crate — no Kotlin/JNI. Reachable from FullSettingsPage's
// "Codec performance test" row → PanelBridge.push(Panel.codec-perf).
//
// Log uses a virtualised ListView (not a single Text in a ScrollView) so long
// factory/benchmark reports scroll smoothly — same lesson as codec_test_page.
import { ListView } from "std-widgets.slint";
import { Badge, BadgeVariant } from "../slintcn/components/badge.slint";
import { Separator } from "../slintcn/components/separator.slint";

import { Bridge, Panel } from "../bridge.slint";
import { PanelBridge } from "../state/panel_bridge.slint";
import { Theme } from "../theme.slint";
import { PrimaryButton } from "../components/buttons.slint";
import { PanelHeader } from "../components/panel_chrome.slint";

export component CodecPerfPage inherits Rectangle {
    width: 100%;
    height: 100%;
    background: Theme.surface-primary;

    forward-focus: panel-scope;
    panel-scope := FocusScope {
        key-pressed(event) => {
            if event.text == Key.Escape {
                PanelBridge.pop();
                return accept;
            }
            return reject;
        }

        VerticalLayout {
            // ── Header ─────────────────────────────────────────────────
            PanelHeader {
                title: @tr("Codec performance");
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
                        text: Bridge.perf-test-running ? @tr("running") : @tr("idle");
                        variant: Bridge.perf-test-running ? BadgeVariant.default : BadgeVariant.secondary;
                    }
                }

                // Buttons row 1.
                HorizontalLayout {
                    spacing: Theme.spacing-default;

                    PrimaryButton {
                        label: Bridge.perf-test-running ? @tr("Running…") : @tr("Full benchmark");
                        enabled: !Bridge.perf-test-running;
                        clicked => { Bridge.run-perf-test(); }
                        horizontal-stretch: 1;
                    }
                    PrimaryButton {
                        label: @tr("List factories");
                        enabled: !Bridge.perf-test-running;
                        clicked => { Bridge.run-perf-list-factories(); }
                        horizontal-stretch: 1;
                    }
                }

                // Buttons row 2.
                HorizontalLayout {
                    spacing: Theme.spacing-default;

                    PrimaryButton {
                        label: @tr("Encode only");
                        enabled: !Bridge.perf-test-running;
                        clicked => { Bridge.run-perf-encode-only(); }
                        horizontal-stretch: 1;
                    }
                    PrimaryButton {
                        label: @tr("Decode only");
                        enabled: !Bridge.perf-test-running;
                        clicked => { Bridge.run-perf-decode-only(); }
                        horizontal-stretch: 1;
                    }
                }

                Separator { }

                // ── Log output (virtualised line list) ─────────────────
                Rectangle {
                    vertical-stretch: 1;

                    // Empty state — shown before any run produces lines.
                    if Bridge.perf-test-log-lines.length == 0: Text {
                        width: 100%;
                        height: 100%;
                        text: @tr("Benchmarks GStreamer androidmedia encode/decode pipelines.\n\nMeasures real throughput (FPS) by counting buffers at fakesink.\n\nPress a button above to start.");
                        color: Theme.text-secondary;
                        font-size: Theme.font-size-label;
                        horizontal-alignment: center;
                        vertical-alignment: center;
                        wrap: word-wrap;
                    }

                    if Bridge.perf-test-log-lines.length > 0: ListView {
                        width: 100%;
                        height: 100%;
                        for line in Bridge.perf-test-log-lines: Rectangle {
                            height: row.preferred-height;
                            row := Text {
                                width: parent.width;
                                text: line;
                                color: Theme.text-secondary;
                                font-size: Theme.font-size-label;
                                font-family: "monospace";
                                wrap: word-wrap;
                            }
                        }
                    }
                }
            }
        }
    }   // end FocusScope
}
```

---

← [Step 2](step-2-bridge-props.md) · [Index](README.md) · Next → [Step 4](step-4-main-slint-route.md)
