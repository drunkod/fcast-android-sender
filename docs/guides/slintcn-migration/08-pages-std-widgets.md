# Step 8 — Migrate raw `std-widgets` usage in pages

← [Step 7: Info banner](07-info-banner.md) · [Index](README.md) · Next → [Step 9: Page order](09-page-order.md)

These are the renames a wrapper can't hide — apply them per page. The buttons/rows wrappers
(Steps 4–6) already cover `PrimaryButton`/`SettingsToggleRow`/etc., so for most pages only the raw
`std-widgets` lines change.

## Verified mapping

| std-widgets | slintcn | import (from `ui/pages/…`) | property change |
|---|---|---|---|
| `LineEdit` | `Input` | `../slintcn/components/input.slint` | `placeholder-text` → `placeholder`; text stays **`text`**; `edited(t)` callback |
| `CheckBox` | `Checkbox` | `../slintcn/components/checkbox.slint` | `checked` same; `label` available |
| `Slider` | `Slider` | `../slintcn/components/slider.slint` | `value`/`minimum`/`maximum` same; **verify callback** |
| `ScrollView` | `ScrollArea` | `../slintcn/components/scroll-area.slint` | needs **`content-height`**; no `mouse-drag-pan-enabled` |
| `Spinner` | *(keep)* or `Skeleton` | — / `../slintcn/components/skeleton.slint` | Progress is NOT a spinner |
| `VerticalBox` | `VerticalLayout` | *(built-in, no import)* | add `padding: 8px; spacing: 8px;` |
| `ListView` | *(keep)* | — | no slintcn equivalent installed |
| `ComboBox` | *(keep)* or `Select`/`Combobox` | `../slintcn/components/select.slint` | only if you add it in Step 1 |

## `LineEdit` → `Input` (28 sites)

```slint
// before
import { LineEdit } from "std-widgets.slint";
ip-field := LineEdit {
    placeholder-text: @tr("Receiver IP address");
    text <=> root.ip;
    edited => { root.on-edit(self.text); }
}

// after
import { Input } from "../slintcn/components/input.slint";
ip-field := Input {
    placeholder: @tr("Receiver IP address");
    text <=> root.ip;
    edited(t) => { root.on-edit(t); }       // verify arg shape in generated input.slint
}
```

## `CheckBox` → `Checkbox` (4 sites)

```slint
// before
import { CheckBox } from "std-widgets.slint";
CheckBox { text: @tr("Enable"); checked <=> root.on; toggled => { … } }

// after
import { Checkbox } from "../slintcn/components/checkbox.slint";
Checkbox { label: @tr("Enable"); checked <=> root.on; /* verify callback name */ }
```

## `Slider` → slintcn `Slider` (4 sites)

```slint
// before
import { Slider } from "std-widgets.slint";
Slider { minimum: 0; maximum: 100; value <=> root.v; changed(x) => { … } }

// after
import { Slider } from "../slintcn/components/slider.slint";
Slider { minimum: 0; maximum: 100; value <=> root.v; /* verify callback name */ }
```

## `ScrollView` → `ScrollArea` (52 sites — riskiest)

`ScrollView` auto-measures its child; `ScrollArea` requires you to declare `content-height`.

```slint
// before
import { ScrollView } from "std-widgets.slint";
ScrollView {
    VerticalLayout { /* content of unknown height */ }
}

// after — ScrollArea needs the total content height
import { ScrollArea } from "../slintcn/components/scroll-area.slint";
ScrollArea {
    content-height: content.preferred-height;   // or a fixed value
    content := VerticalLayout { /* content */ }
}
```

> ⚠️ For dynamic lists, bind `content-height` to the child's `preferred-height` (above) or compute
> it. **Migrate ScrollView last**, page-by-page, and visually check each. If a page uses `ListView`
> (std-widgets), leave it — there is no slintcn ListView in the installed set.

## `Spinner` → keep, or `Skeleton` (6 sites)

```slint
// keep std Spinner (simplest — it stays an intentional dependency)
import { Spinner } from "std-widgets.slint";
Spinner { indeterminate: true; width: 24px; height: 24px; }

// OR slintcn Skeleton for placeholder-style loading
import { Skeleton } from "../slintcn/components/skeleton.slint";
Skeleton { width: 100%; height: 24px; }
```

## `VerticalBox` → `VerticalLayout` (7 sites)

```slint
// before
import { VerticalBox } from "std-widgets.slint";
VerticalBox { /* children */ }

// after — VerticalBox is just VerticalLayout with default padding/spacing
VerticalLayout { padding: 8px; spacing: 8px; /* children */ }
```

← [Step 7: Info banner](07-info-banner.md) · [Index](README.md) · Next → [Step 9: Page order](09-page-order.md)
