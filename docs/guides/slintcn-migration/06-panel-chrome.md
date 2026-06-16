# Step 6 — Migrate `ui/components/panel_chrome.slint`

← [Step 5: Settings rows](05-settings-rows.md) · [Index](README.md) · Next → [Step 7: Info banner](07-info-banner.md)

Read the current file first (it exports `PanelHeader`, `PanelHeaderActions`, `Card`, `FormRow`).
Apply these swaps, keeping the public property/callback surface:

- `TextButton` (the "Done" action) → `Button { variant: ButtonVariant.ghost; }`
- Inner custom `Card` → slintcn `Card { variant: CardVariant.solid; }` — move padding to an inner
  `VerticalLayout` (slintcn Card pads via `padding-l`, not `padding-left`).
- Title `Text` → `Label`.

## Imports

```slint
import { Theme }  from "../theme.slint";
import { Button, ButtonVariant } from "../slintcn/components/button.slint";
import { Label }  from "../slintcn/components/label.slint";
import { Card, CardVariant } from "../slintcn/components/card.slint";
```

## `PanelHeader`

```slint
export component PanelHeader inherits Rectangle {
    in property <string> title;
    callback close-clicked();
    height: Theme.header-height;
    background: Theme.surface-card;

    HorizontalLayout {
        padding: Theme.padding-screen;
        spacing: Theme.spacing-default;
        Label {
            text: root.title;
            font-size: Theme.font-size-heading;
            vertical-alignment: center;
            horizontal-stretch: 1;
        }
        Button {
            text: @tr("close-panel-button" => "Done");
            variant: ButtonVariant.ghost;
            clicked => { root.close-clicked(); }
        }
    }
}
```

## `PanelHeaderActions` (title left, free-form `@children` right)

```slint
export component PanelHeaderActions inherits Rectangle {
    in property <string> title;
    height: Theme.header-height;
    background: Theme.surface-card;

    HorizontalLayout {
        padding: Theme.padding-screen;
        spacing: Theme.spacing-default;
        Label {
            text: root.title;
            font-size: Theme.font-size-heading;
            vertical-alignment: center;
            horizontal-stretch: 1;
        }
        @children
    }
}
```

## `Card` re-export (note the padding move)

```slint
// slintcn Card pads via padding-l / gap-l — pad an INNER layout, not the Card.
export component Card inherits Card {            // slintcn Card
    variant: CardVariant.solid;
    // App-standard padding lives on the inner layout that wraps @children:
    VerticalLayout {
        padding-left:   Theme.padding-screen;
        padding-right:  Theme.padding-screen;
        padding-top:    12px;
        padding-bottom: 12px;
        @children
    }
}
```

> If `inherits Card` + same name causes a clash, rename to e.g. `PanelCard` and update the few
> call sites, or use composition (`inherits Rectangle` holding a slintcn `Card`).

## `FormRow` (vertical label + control pair)

```slint
export component FormRow inherits VerticalLayout {
    in property <string> label;
    spacing: Theme.spacing-tight;
    Label { text: root.label; variant: LabelVariant.muted; font-size: Theme.font-size-label; }
    @children
}
```

> `FormRow` needs `LabelVariant` — add it to the import:
> `import { Label, LabelVariant } from "../slintcn/components/label.slint";`

← [Step 5: Settings rows](05-settings-rows.md) · [Index](README.md) · Next → [Step 7: Info banner](07-info-banner.md)
