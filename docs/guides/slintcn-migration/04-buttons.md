# Step 4 — Migrate `ui/components/buttons.slint`

← [Step 3: Theme](03-theme.md) · [Index](README.md) · Next → [Step 5: Settings rows](05-settings-rows.md)

**97 call sites** use these (`PrimaryButton` 34, `TextButton` 38, `DestructiveButton` 25).
Keep the public `label` / `clicked` / `enabled` surface so **no call site changes**.

## Key mapping

| Old | slintcn | Note |
|---|---|---|
| `PrimaryButton` | `Button` + `ButtonVariant.default` | `label` → wrapper sets `text` |
| `TextButton` | `Button` + `ButtonVariant.ghost` | |
| `DestructiveButton` | `Button` + `ButtonVariant.destructive` | |
| `LoadingView`'s `Spinner` | **keep std Spinner** | Progress is NOT indeterminate |

## Full file (after)

```slint
// buttons.slint — now thin wrappers over slintcn Button.
import { Theme }   from "../theme.slint";
import { Button, ButtonVariant } from "../slintcn/components/button.slint";
import { Label }   from "../slintcn/components/label.slint";
// Spinner stays from std-widgets — slintcn Progress can't spin.
import { Spinner } from "std-widgets.slint";

export component PrimaryButton inherits Button {
    in property <string> label;           // keep call-site API
    text: root.label;
    variant: ButtonVariant.default;
    // `enabled` and `clicked` are already on slintcn Button — inherited.
}

export component TextButton inherits Button {
    in property <string> label;
    text: root.label;
    variant: ButtonVariant.ghost;
}

export component DestructiveButton inherits Button {
    in property <string> label;
    text: root.label;
    variant: ButtonVariant.destructive;
}

// LoadingView: keep std Spinner, swap Text → Label.
export component LoadingView inherits VerticalLayout {
    in property <string> label: @tr("Loading");
    alignment: center;
    spacing: Theme.spacing-default;

    Spinner {
        indeterminate: true;
        width: 48px;
        height: 48px;
    }
    Label {
        text: root.label;
        horizontal-alignment: center;
    }
}
```

## Verify before merging

- slintcn `Button` exposes `enabled` and `clicked` with those exact names (registry shows
  `clicked => {}`). If `enabled` differs, add an `in property <bool> enabled` passthrough.
- If inheriting `Button` **and** redeclaring `label` causes a name clash, switch to composition:

```slint
// composition fallback if inheritance clashes
export component PrimaryButton inherits Rectangle {
    in property <string> label;
    in property <bool> enabled: true;
    callback clicked();
    height: Theme.row-height;
    Button {
        text: root.label;
        variant: ButtonVariant.default;
        enabled: root.enabled;
        clicked => { root.clicked(); }
    }
}
```

## Call-site reference (unchanged — shown for confidence)

```slint
// These keep working as-is after the wrapper swap:
PrimaryButton     { label: @tr("Connect");  clicked => { … } }
TextButton        { label: @tr("Cancel");   clicked => { … } }
DestructiveButton { label: @tr("Reset");    enabled: root.can-reset; clicked => { … } }
```

← [Step 3: Theme](03-theme.md) · [Index](README.md) · Next → [Step 5: Settings rows](05-settings-rows.md)
