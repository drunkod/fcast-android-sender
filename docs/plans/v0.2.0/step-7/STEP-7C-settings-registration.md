# STEP-7C — Per-type settings (optional) + registration

## Optional dedicated settings pages

`widget_text_settings_page.slint` / `widget_image_settings_page.slint` /
`widget_crop_settings_page.slint` mirror the wizard's per-type sections for
**editing an existing widget** (vs creating). Same controls; `Save` calls
`Bridge.update-widget(...)`. Optional for v0.2.0 — the wizard covers creation;
edit can reuse it. Add only if you want post-creation editing without
delete+recreate.

## Registration (`main.slint`)

```slint
import { WidgetWizardPage } from "pages/widget_wizard_page.slint";
import { SceneWidgetLayoutPage } from "pages/scene_widget_layout_page.slint";
if PanelBridge.active == Panel.widget-wizard:        WidgetWizardPage { }
if PanelBridge.active == Panel.scene-widget-layout:  SceneWidgetLayoutPage { }
```

## Verify

```bash
slint-lsp ui/main.slint 2>&1 | grep error   # → (none)
```
Manual: wizard type toggle swaps the config section; create adds the widget to
the scene; layout editor drag moves the box and (live) the on-stream overlay.

## Done — STEP-7 complete

→ Next: [../step-8/INDEX.md](../step-8/INDEX.md)
