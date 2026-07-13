# STEP-5C — Bridge properties + callbacks

Add inside `export global Bridge`:

```slint
    // ── Scenes (v0.2.0) ──────────────────────────────────────────────────────
    in property <[SceneItem]>   scenes: [];
    in-out property <string>    current-scene-id: "";
    in-out property <string>    editing-scene-id: "";          // scene-edit target
    in property  <[ScenePlacementItem]> editing-scene-widgets: []; // placements of editing scene

    callback set-scene(string);                 // scene_id (live switch)
    callback create-scene(string);              // name → new scene
    callback rename-scene(string, string);      // (scene_id, name)
    callback remove-scene(string);              // scene_id
    callback reorder-scenes(int, int);          // (from_idx, to_idx)
    callback set-scene-quick-group(string, int);// (scene_id, group 0–4)
    callback open-scene-edit(string);           // scene_id → push scene-edit

    // ── Widgets (v0.2.0) ─────────────────────────────────────────────────────
    in property <[WidgetItem]>  widgets: [];

    // Wizard draft state
    in-out property <WidgetTypeChoice> draft-widget-type: WidgetTypeChoice.text;
    in-out property <string> draft-widget-name: "";
    in-out property <string> draft-widget-text-format: "";
    in-out property <int>    draft-widget-font-size: 32;
    in-out property <string> draft-widget-image-path: "";
    in-out property <int>    draft-widget-scale-idx: 0;        // 0 fit,1 fill,2 stretch
    in-out property <string> draft-widget-clock-format: "%H:%M:%S";
    in-out property <float>  draft-crop-top: 0;
    in-out property <float>  draft-crop-bottom: 0;
    in-out property <float>  draft-crop-left: 0;
    in-out property <float>  draft-crop-right: 0;

    callback create-widget();                   // reads draft-* → new widget
    callback remove-widget(string);             // widget_id
    callback add-widget-to-scene(string, string);    // (scene_id, widget_id)
    callback remove-widget-from-scene(string, string); // (scene_id, widget_id)
    callback set-placement-enabled(string, string, bool); // (scene_id, widget_id, on)
    callback pick-widget-image();               // opens file picker → draft-widget-image-path

    // ── Widget layout editor ─────────────────────────────────────────────────
    in-out property <string> editing-widget-id: "";
    in-out property <float>  layout-x: 0;
    in-out property <float>  layout-y: 0;
    in-out property <float>  layout-width: 30;
    in-out property <float>  layout-height: 20;
    in-out property <float>  layout-opacity: 1.0;
    // (scene_id, widget_id, x, y, w, h, opacity) — mirrors apply-mixer-slot-config
    callback apply-widget-layout(string, string, float, float, float, float, float);
```

→ Next: [STEP-5D-bindings-verify.md](STEP-5D-bindings-verify.md)
