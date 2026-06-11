# STEP-8A — Navigation entry (settings → Scenes)

Add a row to the **STREAM** section of `ui/pages/settings_page.slint` (next to
the v0.1.0 "RTMP Settings" / "SRT Stream" rows):

```slint
                    SettingsValueRow {
                        icon: "🎬";
                        icon-bg: Theme.icon-bg-neutral;
                        title: @tr("Scenes");
                        value: @tr("open-panel-action" => "Open");
                        clicked => { PanelBridge.push(Panel.scene-list); }
                    }
```

That makes the whole scene/widget tree reachable (scene-list → scene-edit →
widget-wizard → layout).

→ Next: [STEP-8B-scene-button-bar.md](STEP-8B-scene-button-bar.md)
