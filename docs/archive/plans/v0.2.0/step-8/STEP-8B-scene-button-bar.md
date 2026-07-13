# STEP-8B — Scene button bar (live overlay)

> `CastingView` is transparent; status overlays live in `main.slint`, gated on
> `PanelBridge.active == Panel.none && Bridge.app-state != Disconnected`. Add a
> bottom scene bar the same way.

## `ui/components/scene_button_bar.slint`

```slint
import { Theme } from "../theme.slint";
import { Bridge } from "../bridge.slint";
import { RowColors } from "settings_rows.slint";

export component SceneButtonBar inherits Rectangle {
    height: 56px;
    background: Theme.scrim-light;

    HorizontalLayout {
        padding-left: Theme.padding-screen;
        padding-right: Theme.padding-screen;
        spacing: Theme.spacing-default;
        alignment: center;

        for scene[idx] in Bridge.scenes: Rectangle {
            visible: scene.enabled;
            width: scene.enabled ? 96px : 0px;
            height: 40px;
            border-radius: Theme.radius-card;
            background: scene.active ? Theme.accent
                       : (ta.pressed ? RowColors.pressed : RowColors.normal);
            ta := TouchArea {
                clicked => { Bridge.set-scene(scene.id); }
                Text {
                    text: scene.name;
                    color: scene.active ? #ffffff : Theme.text-primary;
                    font-size: Theme.font-size-label;
                    font-weight: scene.active ? 700 : 500;
                    horizontal-alignment: center;
                    vertical-alignment: center;
                    overflow: elide;
                }
            }
        }
    }
}
```

## Place it in `main.slint`

```slint
import { SceneButtonBar } from "components/scene_button_bar.slint";

// after the StreamRightBadges block, inside MainWindow:
if PanelBridge.active == Panel.none
        && Bridge.app-state != AppState.Disconnected
        && Bridge.scenes.length > 1: SceneButtonBar {
    x: 0;
    y: parent.height - self.height - SafeArea.bottom - 8px;
    width: parent.width;
}
```

> Only shown with >1 scene. Tap → `Bridge.set-scene(id)` → STEP-3 reconfigures
> the live mixer. `scene.active` highlights the current scene (Rust keeps it in
> sync via the `scenes` model push, STEP-9).

→ Next: [STEP-8C-quickgroup-verify.md](STEP-8C-quickgroup-verify.md)
