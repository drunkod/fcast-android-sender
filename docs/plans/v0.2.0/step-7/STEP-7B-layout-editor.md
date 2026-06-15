# STEP-7B — Layout editor (`ui/pages/scene_widget_layout_page.slint`)

> Visual canvas + `Slider`s; drag via `TouchArea`; live-applies through
> `apply-widget-layout`.

```slint
import { Theme } from "../theme.slint";
import { Bridge, Panel } from "../bridge.slint";
import { PanelBridge } from "../state/panel_bridge.slint";
import { PanelHeader } from "../components/panel_chrome.slint";
import { Slider } from "../slintcn/components/slider.slint";
import { Button, ButtonVariant, ButtonSize } from "../slintcn/components/button.slint";

export component SceneWidgetLayoutPage inherits Rectangle {
    width: 100%; height: 100%;
    background: transparent;

    function apply() {
        Bridge.apply-widget-layout(
            Bridge.editing-scene-id, Bridge.editing-widget-id,
            Bridge.layout-x, Bridge.layout-y,
            Bridge.layout-width, Bridge.layout-height,
            Bridge.layout-opacity);
    }

    forward-focus: scope;
    scope := FocusScope {
        key-pressed(event) => {
            if (event.text == Key.Escape) { PanelBridge.pop(); return accept; }
            return reject;
        }
        VerticalLayout {
            PanelHeader { title: @tr("Layout"); close-clicked => { PanelBridge.pop(); } }
            VerticalLayout {
                padding: Theme.padding-screen; spacing: Theme.spacing-loose;

                // ── 16:9 preview canvas with a draggable widget rectangle ──
                canvas := Rectangle {
                    width: parent.width - 2 * Theme.padding-screen;
                    height: self.width * 9.0 / 16.0;
                    background: #000000;
                    border-width: 1px; border-color: Theme.text-disabled;

                    box := Rectangle {
                        x: parent.width  * Bridge.layout-x / 100.0;
                        y: parent.height * Bridge.layout-y / 100.0;
                        width:  parent.width  * Bridge.layout-width  / 100.0;
                        height: parent.height * Bridge.layout-height / 100.0;
                        background: Theme.accent.with-alpha(0.35 * Bridge.layout-opacity);
                        border-width: 1px; border-color: Theme.accent;
                        drag := TouchArea {
                            moved => {
                                Bridge.layout-x = Math.clamp(
                                    Bridge.layout-x + (self.mouse-x - self.pressed-x) / canvas.width * 100.0,
                                    0, 100 - Bridge.layout-width);
                                Bridge.layout-y = Math.clamp(
                                    Bridge.layout-y + (self.mouse-y - self.pressed-y) / canvas.height * 100.0,
                                    0, 100 - Bridge.layout-height);
                                root.apply();
                            }
                        }
                    }
                }

                Text { text: @tr("Width: ") + Math.round(Bridge.layout-width) + "%"; color: Theme.text-secondary; font-size: Theme.font-size-label; }
                Slider { value <=> Bridge.layout-width;   minimum: 1; maximum: 100; changed(v) => { root.apply(); } }
                Text { text: @tr("Height: ") + Math.round(Bridge.layout-height) + "%"; color: Theme.text-secondary; font-size: Theme.font-size-label; }
                Slider { value <=> Bridge.layout-height;  minimum: 1; maximum: 100; changed(v) => { root.apply(); } }
                Text { text: @tr("Opacity: ") + Math.round(Bridge.layout-opacity * 100) + "%"; color: Theme.text-secondary; font-size: Theme.font-size-label; }
                Slider { value <=> Bridge.layout-opacity; minimum: 0; maximum: 1;   changed(v) => { root.apply(); } }

                Button { text: @tr("Done"); variant: ButtonVariant.default; size: ButtonSize.lg;
                    clicked => { root.apply(); PanelBridge.pop(); } }
            }
        }
    }
}
```

> Drag uses `mouse-x/pressed-x` deltas. `apply()` → `apply-widget-layout` →
> STEP-9 maps it to `Command::UpdateWidgetLayout` → STEP-3 turns it into live
> control points on the running compositor slot, so the overlay moves on the
> actual stream as you drag.

→ Next: [STEP-7C-settings-registration.md](STEP-7C-settings-registration.md)
