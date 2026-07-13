# STEP-7A — Widget wizard (`ui/pages/widget_wizard_page.slint`)

> `ToggleGroup` maps to `WidgetTypeChoice` by index (0 text, 1 image, 2 crop, 3 clock).

```slint
import { Theme } from "../theme.slint";
import { Bridge, Panel, WidgetTypeChoice } from "../bridge.slint";
import { PanelBridge } from "../state/panel_bridge.slint";
import { PanelHeader } from "../components/panel_chrome.slint";
import { RowColors } from "../components/settings_rows.slint";
import { Input } from "../slintcn/components/input.slint";
import { Slider } from "../slintcn/components/slider.slint";
import { Button, ButtonVariant, ButtonSize } from "../slintcn/components/button.slint";
import { Card, CardVariant, CardPadding } from "../slintcn/components/card.slint";
import { ToggleGroup, ToggleGroupItem } from "../slintcn/components/toggle-group.slint";

export component WidgetWizardPage inherits Rectangle {
    width: 100%; height: 100%;
    background: transparent;
    in-out property <int> type-idx: 0;

    function sync-type() {
        Bridge.draft-widget-type =
            root.type-idx == 1 ? WidgetTypeChoice.image :
            root.type-idx == 2 ? WidgetTypeChoice.crop  :
            root.type-idx == 3 ? WidgetTypeChoice.clock :
                                 WidgetTypeChoice.text;
    }

    forward-focus: scope;
    scope := FocusScope {
        key-pressed(event) => {
            if (event.text == Key.Escape) { PanelBridge.pop(); return accept; }
            return reject;
        }
        VerticalLayout {
            PanelHeader { title: @tr("New Widget"); close-clicked => { PanelBridge.pop(); } }
            Flickable {
                vertical-stretch: 1;
                viewport-height: body.preferred-height;
                body := VerticalLayout {
                    padding: Theme.padding-screen; spacing: Theme.spacing-loose;
                    alignment: start;

                    Text { text: @tr("TYPE"); color: Theme.text-secondary; font-size: Theme.font-size-label; font-weight: 600; }
                    ToggleGroup {
                        items: [{ label: @tr("Text") }, { label: @tr("Image") }, { label: @tr("Crop") }, { label: @tr("Clock") }];
                        selected <=> root.type-idx;
                        changed(i) => { root.sync-type(); }
                    }

                    Text { text: @tr("NAME"); color: Theme.text-secondary; font-size: Theme.font-size-label; font-weight: 600; }
                    Rectangle {
                        height: 56px; background: RowColors.normal; border-radius: Theme.radius-card;
                        VerticalLayout { padding: Theme.padding-screen;
                            Input { text <=> Bridge.draft-widget-name; placeholder: @tr("Widget name"); }
                        }
                    }

                    // ── Text config ──
                    if root.type-idx == 0: VerticalLayout {
                        spacing: Theme.spacing-default;
                        Text { text: @tr("TEXT"); color: Theme.text-secondary; font-size: Theme.font-size-label; font-weight: 600; }
                        Rectangle { height: 56px; background: RowColors.normal; border-radius: Theme.radius-card;
                            VerticalLayout { padding: Theme.padding-screen;
                                Input { text <=> Bridge.draft-widget-text-format; placeholder: "{time} · My Stream"; }
                            }
                        }
                        Text { text: @tr("Font size: ") + Bridge.draft-widget-font-size; color: Theme.text-secondary; font-size: Theme.font-size-label; }
                        Slider { value <=> Bridge.draft-widget-font-size; minimum: 12; maximum: 96; }
                    }

                    // ── Image config ──
                    if root.type-idx == 1: VerticalLayout {
                        spacing: Theme.spacing-default;
                        Text { text: @tr("IMAGE"); color: Theme.text-secondary; font-size: Theme.font-size-label; font-weight: 600; }
                        Button { text: Bridge.draft-widget-image-path != "" ? @tr("Change image") : @tr("Pick image");
                            variant: ButtonVariant.outline; size: ButtonSize.default;
                            clicked => { Bridge.pick-widget-image(); } }
                        if Bridge.draft-widget-image-path != "": Text {
                            text: Bridge.draft-widget-image-path; color: Theme.text-disabled;
                            font-size: Theme.font-size-label; wrap: word-wrap;
                        }
                    }

                    // ── Crop config ──
                    if root.type-idx == 2: VerticalLayout {
                        spacing: Theme.spacing-default;
                        Text { text: @tr("CROP (%)"); color: Theme.text-secondary; font-size: Theme.font-size-label; font-weight: 600; }
                        Text { text: @tr("Top: ") + Bridge.draft-crop-top; color: Theme.text-secondary; font-size: Theme.font-size-label; }
                        Slider { value <=> Bridge.draft-crop-top; minimum: 0; maximum: 50; }
                        Text { text: @tr("Bottom: ") + Bridge.draft-crop-bottom; color: Theme.text-secondary; font-size: Theme.font-size-label; }
                        Slider { value <=> Bridge.draft-crop-bottom; minimum: 0; maximum: 50; }
                        Text { text: @tr("Left: ") + Bridge.draft-crop-left; color: Theme.text-secondary; font-size: Theme.font-size-label; }
                        Slider { value <=> Bridge.draft-crop-left; minimum: 0; maximum: 50; }
                        Text { text: @tr("Right: ") + Bridge.draft-crop-right; color: Theme.text-secondary; font-size: Theme.font-size-label; }
                        Slider { value <=> Bridge.draft-crop-right; minimum: 0; maximum: 50; }
                    }

                    // ── Clock config ──
                    if root.type-idx == 3: VerticalLayout {
                        spacing: Theme.spacing-default;
                        Text { text: @tr("CLOCK FORMAT"); color: Theme.text-secondary; font-size: Theme.font-size-label; font-weight: 600; }
                        Rectangle { height: 56px; background: RowColors.normal; border-radius: Theme.radius-card;
                            VerticalLayout { padding: Theme.padding-screen;
                                Input { text <=> Bridge.draft-widget-clock-format; placeholder: "%H:%M:%S"; }
                            }
                        }
                    }

                    Button {
                        text: @tr("Create Widget");
                        variant: ButtonVariant.default; size: ButtonSize.lg;
                        enabled: Bridge.draft-widget-name != "";
                        clicked => {
                            root.sync-type();
                            Bridge.create-widget();            // reads draft-* (STEP-9)
                            PanelBridge.pop();
                        }
                    }
                }
            }
        }
    }
}
```

> Type-specific config gated by `if root.type-idx == N` — the Slint analog of
> Moblin's `switch createWidgetWizard.type`.

→ Next: [STEP-7B-layout-editor.md](STEP-7B-layout-editor.md)
