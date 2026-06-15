# STEP-5B — Structs + enum

**File:** `ui/bridge.slint`

```slint
export struct SceneItem {
    id: string,
    name: string,
    enabled: bool,
    active: bool,        // currently live scene
    widget-count: int,
    quick-switch-group: int,   // 0 = none, 1–4
}

export struct WidgetItem {
    id: string,
    name: string,
    widget-type: string,       // "text" | "image" | "crop" | "clock"
    enabled: bool,
}

// A widget's placement within the scene being edited.
export struct ScenePlacementItem {
    widget-id: string,
    name: string,
    widget-type: string,
    enabled: bool,
    x: float, y: float, width: float, height: float,
    opacity: float, zorder: int,
}

export enum WidgetTypeChoice { text, image, crop, clock }
```

→ Next: [STEP-5C-properties-callbacks.md](STEP-5C-properties-callbacks.md)
