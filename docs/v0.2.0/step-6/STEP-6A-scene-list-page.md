# STEP-6A — `scene_list_page.slint`

```slint
import { Theme } from "../theme.slint";
import { Bridge, Panel } from "../bridge.slint";
import { PanelBridge } from "../state/panel_bridge.slint";
import { PanelHeader } from "../components/panel_chrome.slint";
import { SettingsValueRow, RowColors } from "../components/settings_rows.slint";
import { Switch } from "../slintcn/components/switch.slint";
import { Badge, BadgeVariant } from "../slintcn/components/badge.slint";
import { Button, ButtonVariant, ButtonSize } from "../slintcn/components/button.slint";
import { Card, CardVariant, CardPadding } from "../slintcn/components/card.slint";
import { Input } from "../slintcn/components/input.slint";

export component SceneListPage inherits Rectangle {
    width: 100%; height: 100%;
    background: transparent;
    in-out property <string> draft-name: "";

    forward-focus: scope;
    scope := FocusScope {
        key-pressed(event) => {
            if (event.text == Key.Escape) { PanelBridge.pop(); return accept; }
            return reject;
        }
        VerticalLayout {
            PanelHeader { title: @tr("Scenes"); close-clicked => { PanelBridge.pop(); } }

            Flickable {
                vertical-stretch: 1;
                viewport-height: body.preferred-height;
                body := VerticalLayout {
                    padding: Theme.padding-screen;
                    spacing: Theme.spacing-loose;
                    alignment: start;

                    if Bridge.scenes.length == 0: Text {
                        text: @tr("No scenes yet. Create one to start composing overlays.");
                        color: Theme.text-secondary;
                        font-size: Theme.font-size-body;
                        wrap: word-wrap;
                    }

                    Card {
                        variant: CardVariant.solid;
                        card-padding: CardPadding.none;
                        clip: true;
                        VerticalLayout {
                            spacing: 1px;
                            for scene[idx] in Bridge.scenes: Rectangle {
                                height: 64px;
                                background: row.pressed ? RowColors.pressed : RowColors.normal;
                                row := TouchArea {
                                    clicked => { Bridge.open-scene-edit(scene.id); }
                                    HorizontalLayout {
                                        padding-left: Theme.padding-screen;
                                        padding-right: Theme.padding-screen;
                                        spacing: Theme.spacing-default;
                                        alignment: stretch;
                                        VerticalLayout {
                                            alignment: center;
                                            horizontal-stretch: 1;
                                            spacing: 2px;
                                            HorizontalLayout {
                                                spacing: Theme.spacing-tight;
                                                Text {
                                                    text: scene.name;
                                                    color: Theme.text-primary;
                                                    font-size: Theme.font-size-body;
                                                    font-weight: 600;
                                                    vertical-alignment: center;
                                                }
                                                if scene.active: Badge {
                                                    text: @tr("Live"); variant: BadgeVariant.default;
                                                }
                                                if scene.quick-switch-group > 0: Badge {
                                                    text: "Q\{scene.quick-switch-group}";
                                                    variant: BadgeVariant.secondary;
                                                }
                                            }
                                            Text {
                                                text: "\{scene.widget-count} " + @tr("widgets");
                                                color: Theme.text-secondary;
                                                font-size: Theme.font-size-label;
                                            }
                                        }
                                        Switch {
                                            checked: scene.enabled;
                                            toggled(on) => { /* Rust: persist enabled via update-scene */ }
                                        }
                                        Text {
                                            text: "›"; color: Theme.text-disabled;
                                            font-size: 18pt; vertical-alignment: center;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    Rectangle {
                        height: 56px; background: RowColors.normal;
                        border-radius: Theme.radius-card;
                        VerticalLayout {
                            padding: Theme.padding-screen; spacing: 4px;
                            Input {
                                text <=> root.draft-name;
                                placeholder: @tr("New scene name");
                            }
                        }
                    }
                    Button {
                        text: @tr("Create Scene");
                        variant: ButtonVariant.default;
                        size: ButtonSize.lg;
                        enabled: root.draft-name != "";
                        clicked => {
                            Bridge.create-scene(root.draft-name);
                            root.draft-name = "";
                        }
                    }
                }
            }
        }
    }
}
```

> **Reorder:** Slint has no built-in drag-reorder for `ListView`; mirror the
> v0.1.0 macro pattern (up/down arrow buttons → `Bridge.reorder-scenes(from,to)`)
> if ordering matters (PHASE-40 §40-D).

→ Next: [STEP-6B-scene-edit-page.md](STEP-6B-scene-edit-page.md)
