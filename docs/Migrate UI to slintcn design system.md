> **Correction note:** This original research was written from assumptions. The split migration plan in `docs/slintcn-migration/` is the source of truth, and Step 0 corrections have been applied here for the known incorrect slintcn API claims.

Based on my analysis of the repository, here is the complete mapping and migration plan.

## Current Design System Inventory

The project uses a fully custom design system:

| Current File | Exports | slintcn Equivalent |
|---|---|---|
| `ui/theme.slint` | `Theme` global (colors, spacing, radii) | slintcn theme tokens (`ui/slintcn/theme/`) |
| `ui/components/buttons.slint` | `PrimaryButton`, `TextButton`, `DestructiveButton`, `LoadingView` | `Button` (variants), std `Spinner` or `Skeleton` |
| `ui/components/settings_rows.slint` | `SettingsSection`, `SettingsValueRow`, `SettingsToggleRow`, `SettingsTextRow`, `SettingsSliderRow` | `Card`, `Switch`, `Slider`, `Separator`, `Label` |
| `ui/components/panel_chrome.slint` | `PanelHeader`, `PanelHeaderActions`, `Card`, `FormRow` | `Sheet`, `Card` |
| `ui/components/info_banner.slint` | `InfoBanner` | `Alert` or `Toast` |
| built-in `std-widgets.slint` | `LineEdit`, `CheckBox`, `Slider`, `ScrollView`, `Spinner` | `Input`, `Checkbox`, `Slider`, `ScrollArea`, std `Spinner` or `Skeleton` |

---

## Component Mapping with slintcn Snippets

### Step 1 — Install slintcn CLI and scaffold config

Create `ui/slintcn.json` at the project root (next to `Cargo.toml`):

```json
{
  "$schema": "https://slintcn.dev/schema/slintcn.json",
  "style": "default",
  "baseColor": "neutral",
  "outDir": "ui/slintcn",
  "themeDir": "ui/slintcn/theme",
  "componentsDir": "ui/slintcn/components",
  "blocksDir": "ui/slintcn/blocks",
  "registries": {}
}
```

Then run the CLI once at the terminal to vendor all needed components into `ui/slintcn/`:

```bash
npx slintcn@latest add \
  button card input badge separator label \
  dialog alert-dialog sheet tooltip toast \
  checkbox switch icon slider \
  progress skeleton alert scroll-area \
  select tabs toggle
```

### Step 2 — Leave `build.rs` unchanged

`slintcn` files are vendored into `ui/slintcn/` the same way this repository already vendors `ui/components/std/`. Do **not** modify `build.rs` to run `node slintcn.mjs` or regenerate files during Cargo builds.

---

### Step 3 — Migrate `ui/theme.slint` → slintcn theme tokens

After install, inspect the generated `ui/slintcn/theme/` directory and use the real global name/path. You **keep** `ui/theme.slint` only for app-specific tokens (control-bar-height, safe-area helpers, log colors) and re-map color tokens to slintcn's palette.

```slint
// ui/theme.slint — App-specific tokens only. Colors now come from slintcn.
// TODO: replace Palette/import with the real generated global name/path.
import { Palette } from "slintcn/theme/tokens.slint";

export global Theme {
    // ── App-specific layout tokens (no slintcn equivalent) ────────────────
    out property <length> control-bar-height: 72px;
    out property <length> header-height:      56px;
    out property <length> row-height:         48px;
    out property <length> row-height-compact: 40px;
    out property <length> padding-screen:     12px;
    out property <length> thumbnail-width:    200px;
    out property <length> qr-square-min:      240px;
    out property <length> qr-square-max:      360px;

    // ── Re-export slintcn palette aliases ─────────────────────────────────
    out property <color> surface-card:    Palette.card;
    out property <color> surface-bar:     Palette.background;
    out property <color> text-primary:    Palette.foreground;
    out property <color> text-secondary:  Palette.muted-foreground;
    out property <color> text-disabled:   Palette.muted-foreground;
    out property <color> accent:          Palette.primary;
    out property <color> accent-active:   Palette.primary;
    out property <color> error:           Palette.destructive;
    out property <color> error-fg:        Palette.destructive-foreground;

    // ── Debug log level colors (no slintcn equivalent) ────────────────────
    out property <color> log-trace:   #888888;
    out property <color> log-debug:   #4080ff;
    out property <color> log-info:    #20a020;
    out property <color> log-warning: #f0a020;
    out property <color> log-error:   #e02020;
}
```

---

### Step 4 — Migrate `ui/components/buttons.slint`

Replace all three custom buttons with slintcn `Button` variants. The `LoadingView` keeps a std `Spinner`; slintcn `Progress` is a 0-100 value bar, not an indeterminate spinner.

```slint
// ui/components/buttons.slint
import { Button, ButtonVariant } from "../slintcn/components/button.slint";
import { Spinner } from "std-widgets.slint";
import { Label } from "../slintcn/components/label.slint";

// PrimaryButton → Button (default variant)
export component PrimaryButton inherits Button {
    // Button already has: label (text), enabled, clicked()
    // variant defaults to "default" (filled primary color)
}

// TextButton → Button ghost variant
export component TextButton inherits Button {
    variant: ButtonVariant.ghost;
}

// DestructiveButton → Button destructive variant
export component DestructiveButton inherits Button {
    variant: ButtonVariant.destructive;
}

// LoadingView → std Spinner + Label (Progress is a 0–100 bar, not indeterminate)
export component LoadingView inherits VerticalLayout {
    in property <string> label: @tr("Loading");
    alignment: center;
    spacing: 8px;

    Spinner {
        width: 48px;
        height: 48px;
    }
    Label {
        text: root.label;
        horizontal-alignment: center;
    }
}
```

---

### Step 5 — Migrate `ui/components/settings_rows.slint`

```slint
// ui/components/settings_rows.slint
import { Card, CardVariant } from "../slintcn/components/card.slint";
import { Switch } from "../slintcn/components/switch.slint";
import { Slider } from "../slintcn/components/slider.slint";
import { Separator } from "../slintcn/components/separator.slint";
import { Label, LabelVariant } from "../slintcn/components/label.slint";
import { Theme }     from "../theme.slint";

// ── SettingsSection — Card wrapping rows with a section title ─────────────
export component SettingsSection inherits VerticalLayout {
    in property <string> title: "";
    spacing: 6px;
    vertical-stretch: 0;
    alignment: start;

    if root.title != "": Label {
        text: root.title;
        variant: LabelVariant.muted;   // slintcn Label muted variant = secondary color
        font-size: 9pt;
        font-weight: 600;
        padding-left: Theme.padding-screen;
    }

    Card {
        variant: CardVariant.solid;
        clip: true;
        VerticalLayout {
            alignment: start;
            spacing: 1px;
            @children
        }
    }
}

// ── SettingsValueRow — navigation row with optional icon + chevron ────────
export component SettingsValueRow inherits Rectangle {
    in property <string> icon: "";
    in property <color>  icon-bg: #374151;
    in property <string> title;
    in property <string> value: "";
    in property <bool>   enabled: true;
    in property <bool>   show-chevron: true;
    callback clicked();

    height: Theme.row-height;
    opacity: root.enabled ? 1.0 : 0.45;
    background: ta.pressed ? #2a3347 : #1e2535;

    accessible-role:    button;
    accessible-label:   root.title + (root.value != "" ? ", " + root.value : "");
    accessible-enabled: root.enabled;
    accessible-action-default => { if root.enabled { root.clicked(); } }

    ta := TouchArea {
        enabled: root.enabled;
        clicked => { root.clicked(); }
    }
    HorizontalLayout {
        padding-left: Theme.padding-screen;
        padding-right: Theme.padding-screen;
        spacing: 8px;
        alignment: center;

        if root.icon != "": Rectangle {
            width: 28px; height: 28px;
            border-radius: 6px;
            background: root.icon-bg;
            Text {
                text: root.icon;
                font-size: 11pt;
                color: white;
                horizontal-alignment: center;
                vertical-alignment: center;
            }
        }
        Label { text: root.title; horizontal-stretch: 1; vertical-alignment: center; }
        if root.value != "": Label { text: root.value; variant: LabelVariant.muted; vertical-alignment: center; }
        if root.show-chevron: Label { text: "›"; variant: LabelVariant.muted; font-size: 15pt; vertical-alignment: center; }
    }
}

// ── SettingsToggleRow — row with slintcn Switch ───────────────────────────
export component SettingsToggleRow inherits Rectangle {
    in property <string> icon: "";
    in property <color>  icon-bg: #374151;
    in property <string> title;
    in-out property <bool> checked: false;
    in property <bool>     enabled: true;
    callback toggled(bool);

    height: Theme.row-height;
    opacity: root.enabled ? 1.0 : 0.45;
    background: #1e2535;

    accessible-role:    switch;
    accessible-label:   root.title;
    accessible-enabled: root.enabled;
    accessible-checked: root.checked;
    accessible-action-default => {
        if root.enabled { root.checked = !root.checked; root.toggled(root.checked); }
    }

    HorizontalLayout {
        padding-left: Theme.padding-screen;
        padding-right: Theme.padding-screen;
        spacing: 8px;
        alignment: center;

        if root.icon != "": Rectangle {
            width: 28px; height: 28px;
            border-radius: 6px;
            background: root.icon-bg;
            Text {
                text: root.icon; font-size: 11pt; color: white;
                horizontal-alignment: center; vertical-alignment: center;
            }
        }
        Label { text: root.title; horizontal-stretch: 1; vertical-alignment: center; }
        Switch {
            checked <=> root.checked;
            // Confirm callback availability in generated source before wiring;
            // registry usage only verifies checked <=> binding.
        }
    }
}

// ── SettingsTextRow — read-only info row ──────────────────────────────────
export component SettingsTextRow inherits Rectangle {
    in property <string> title;
    in property <string> subtitle: "";
    height: root.subtitle == "" ? Theme.row-height : Theme.row-height + 9pt + 8px;
    background: #1e2535;

    VerticalLayout {
        padding-left: Theme.padding-screen;
        padding-right: Theme.padding-screen;
        alignment: center;
        Label { text: root.title; }
        if root.subtitle != "": Label { text: root.subtitle; variant: LabelVariant.muted; font-size: 9pt; }
    }
}

// ── SettingsSliderRow — row with slintcn Slider ───────────────────────────
export component SettingsSliderRow inherits Rectangle {
    in property <string> title;
    in property <string> unit: "";
    in property <float>  minimum: 0;
    in property <float>  maximum: 100;
    in property <bool>   show-fractional: false;
    in-out property <float> value: 50;
    callback changed(float);

    height: Theme.row-height * 1.5;
    background: #1e2535;

    VerticalLayout {
        padding-left: Theme.padding-screen;
        padding-right: Theme.padding-screen;
        padding-top: 4px;
        HorizontalLayout {
            Label { text: root.title; horizontal-stretch: 1; vertical-alignment: center; }
            Label {
                text: root.show-fractional
                    ? "\{Math.round(root.value * 10) / 10}\{root.unit}"
                    : "\{Math.round(root.value)}\{root.unit}";
                variant: LabelVariant.muted;
                vertical-alignment: center;
            }
        }
        Slider {
            minimum: root.minimum;
            maximum: root.maximum;
            value <=> root.value;
            // Confirm callback availability in generated source before wiring;
            // registry usage only verifies value <=> binding.
        }
    }
}
```

---

### Step 6 — Migrate `ui/components/panel_chrome.slint`

```slint
// ui/components/panel_chrome.slint
import { Card, CardVariant } from "../slintcn/components/card.slint";
import { Button, ButtonVariant } from "../slintcn/components/button.slint";
import { Label, LabelVariant } from "../slintcn/components/label.slint";
import { Theme } from "../theme.slint";

// PanelHeader — title left, "Done" ghost button right
export component PanelHeader inherits Rectangle {
    in property <string> title;
    callback close-clicked();

    height: Theme.header-height;
    background: Theme.surface-card;

    HorizontalLayout {
        padding: Theme.padding-screen;
        spacing: 8px;
        Label {
            text: root.title;
            font-size: 15pt;
            vertical-alignment: center;
            horizontal-stretch: 1;
            accessible-role: text;
        }
        Button {
            text: @tr("close-panel-button" => "Done");
            variant: ButtonVariant.ghost;
            clicked => { root.close-clicked(); }
        }
    }
}

// PanelHeaderActions — title left, free-form @children right
export component PanelHeaderActions inherits Rectangle {
    in property <string> title;
    height: Theme.header-height;
    background: Theme.surface-card;

    HorizontalLayout {
        padding: Theme.padding-screen;
        spacing: 8px;
        Label {
            text: root.title;
            font-size: 15pt;
            vertical-alignment: center;
            horizontal-stretch: 1;
        }
        @children
    }
}

// AppCard — thin wrapper around slintcn Card with inner padding
export component AppCard inherits Card {  // slintcn Card
    variant: CardVariant.solid;
    VerticalLayout {
        padding: parent.padding-l;
        spacing: parent.gap-l;
        @children
    }
}

// FormRow — vertical label + control pair
export component FormRow inherits VerticalLayout {
    in property <string> label;
    spacing: 4px;
    Label { text: root.label; variant: LabelVariant.muted; font-size: 9pt; }
    @children
}
```

---

### Step 7 — Migrate `ui/components/info_banner.slint`

Keep the current custom severity coloring. slintcn `Alert` only has `AlertVariant.default` and `AlertVariant.destructive`, so it cannot represent this app's warning and success banner states.

```slint
// ui/components/info_banner.slint
import { Label } from "../slintcn/components/label.slint";
import { Bridge, BannerSeverity } from "../bridge.slint";
import { Theme } from "../theme.slint";

export component InfoBanner inherits Rectangle {
    in property <string>         message:  Bridge.banner-message;
    in property <BannerSeverity> severity: Bridge.banner-severity;
    in-out property <bool>       shown:    Bridge.banner-visible;

    height: root.shown ? 40px : 0px;
    clip: true;
    animate height { duration: 200ms; easing: ease-out; }

    accessible-role:  text;
    accessible-label: root.message;

    states [
        error when root.severity == BannerSeverity.error : { background: Theme.error; }
        warning when root.severity == BannerSeverity.warning : { background: Theme.warning; }
        success when root.severity == BannerSeverity.success : { background: Theme.success; }
    ]

    HorizontalLayout {
        padding-left: Theme.padding-screen;
        padding-right: Theme.padding-screen;
        Label {
            text: root.message;
            vertical-alignment: center;
        }
    }
}
```

---

### Step 8 — Migrate `std-widgets` usages in pages

Every page that imports from `std-widgets.slint` needs updating:

| `std-widgets` | slintcn replacement | Import path |
|---|---|---|
| `LineEdit` | `Input` | `slintcn/components/input.slint` |
| `CheckBox` | `Checkbox` | `slintcn/components/checkbox.slint` |
| `Slider` | `Slider` | `slintcn/components/slider.slint` |
| `ScrollView` | `ScrollArea` | `slintcn/components/scroll-area.slint` |
| `Spinner` | keep std `Spinner` or use `Skeleton` | built-in `std-widgets.slint` / `slintcn/components/skeleton.slint` |
| `VerticalBox` | plain `VerticalLayout` | (no import needed) |

Example — `ui/pages/settings_page.slint` receiver section:

```slint
// Before (std-widgets):
import { LineEdit, Spinner, ScrollView } from "std-widgets.slint";

// After (slintcn):
import { Input }    from "../slintcn/components/input.slint";
import { ScrollArea } from "../slintcn/components/scroll-area.slint";

// Usage — LineEdit → Input
ip-field := Input {
    placeholder: @tr("Receiver IP address");
    text <=> receiver-ip;
}

// Usage — Spinner stays Spinner
Spinner {
    width: 20px;
    height: 20px;
}

// Usage — ScrollView → ScrollArea
ScrollArea {
    content-height: content.preferred-height;
    content := VerticalLayout {
        // content goes here
    }
}
```

---

### Step 9 — Page-by-page migration checklist

For each page in `ui/pages/`, apply the following substitutions:

```
PrimaryButton    → Button (default)
TextButton       → Button (ButtonVariant.ghost)
DestructiveButton→ Button (ButtonVariant.destructive)
LoadingView      → std Spinner + Label
SettingsSection  → SettingsSection (updated in step 5)
SettingsValueRow → SettingsValueRow (updated in step 5)
SettingsToggleRow→ SettingsToggleRow (updated in step 5)
SettingsSliderRow→ SettingsSliderRow (updated in step 5)
PanelHeader      → PanelHeader (updated in step 6)
Card             → Card (updated in step 6)
LineEdit         → Input
CheckBox         → Checkbox
Slider           → Slider (slintcn)
ScrollView       → ScrollArea
Spinner          → keep std Spinner or use Skeleton
VerticalBox      → VerticalLayout
```

Pages to update (all 26 in `ui/pages/`): [0-cite-0](#0-cite-0) [0-cite-1](#0-cite-1) [0-cite-2](#0-cite-2) 

---

### Step 10 — Audit remaining `std-widgets.slint` usage

After all pages are migrated, audit for any remaining raw page-level `std-widgets.slint` imports:

```bash
grep -r "std-widgets" ui/
```

Replace raw page imports where slintcn has a verified equivalent. Leave the repository's vendored `ui/components/std/` chain intact; it is used only by `ui/components/mcore/common.slint`.

---

### Step 11 — Validate the build

```bash
cargo build --target aarch64-linux-android
```

The slintcn files are already vendored before validation; `build.rs` does not regenerate them. Fix any type mismatches surfaced by Slint.

---

## Key Property Name Changes to Watch

| Old component | Old property | slintcn property |
|---|---|---|
| `PrimaryButton` | `label` | `text` (Button) |
| `TextButton` | `label` | `text` (Button) |
| `DestructiveButton` | `label` | `text` (Button) |
| `CheckBox` | `checked` | `checked` (same) |
| `Slider` | `value` | `value` (same) |
| `LineEdit` | `text`, `placeholder-text` | `text`, `placeholder` (Input) |
| `ScrollView` | `mouse-drag-pan-enabled` | `content-height` on ScrollArea |

Repository: `kodyka/fcast-android-sender` (ref: slintcn)

## Goal
Replace the custom design system (ui/theme.slint, ui/components/buttons.slint, ui/components/settings_rows.slint, ui/components/panel_chrome.slint, ui/components/info_banner.slint, and all std-widgets usages) with slintcn components from https://github.com/zero-sq/slintcn.

## Steps

### 1. Add slintcn config file
Create `slintcn.json` at the repo root (next to `Cargo.toml`):
```json
{
  "$schema": "https://slintcn.dev/schema/slintcn.json",
  "style": "default",
  "baseColor": "neutral",
  "outDir": "ui/slintcn",
  "themeDir": "ui/slintcn/theme",
  "componentsDir": "ui/slintcn/components",
  "blocksDir": "ui/slintcn/blocks",
  "registries": {}
}
```

### 2. Update build.rs
Do not modify `build.rs` for slintcn. Run `npx slintcn@latest add button card input badge separator label dialog alert-dialog sheet tooltip toast checkbox switch icon slider progress skeleton alert scroll-area select tabs toggle` once at the terminal and commit the vendored `ui/slintcn/` output.

### 3. Update ui/theme.slint
After install, inspect the generated `ui/slintcn/theme/` files and use the real theme global name/path. Keep only app-specific layout tokens (control-bar-height, header-height, row-height, padding-screen, thumbnail-width, qr-square-min, qr-square-max, log level colors). Replace color token values with aliases to the verified slintcn palette properties.

### 4. Migrate ui/components/buttons.slint
- `PrimaryButton`: inherit slintcn `Button` from `slintcn/components/button.slint`. Change `label` property to `text` (slintcn Button uses `text`). Default variant is filled/primary.
- `TextButton`: inherit slintcn `Button` with `variant: ButtonVariant.ghost`.
- `DestructiveButton`: inherit slintcn `Button` with `variant: ButtonVariant.destructive`.
- `LoadingView`: keep std `Spinner` or use slintcn `Skeleton`; do not use `Progress` as a spinner.

### 5. Migrate ui/components/settings_rows.slint
- Remove `import { CheckBox, Slider } from "std-widgets.slint"`.
- Import `Card`, `Switch`, `Slider`, `Separator`, `Label` from slintcn.
- `SettingsSection`: wrap children in slintcn `Card` instead of a plain `Rectangle`. Use slintcn `Label` with muted variant for the section title.
- `SettingsToggleRow`: replace `CheckBox` with slintcn `Switch`. Keep the `checked <=>` binding; confirm callback names in the generated source before wiring `toggled`.
- `SettingsSliderRow`: replace std-widgets `Slider` with slintcn `Slider`. The registry verifies `minimum`, `maximum`, and `value <=>`; confirm callback names in the generated source before wiring `changed`.
- `SettingsValueRow` and `SettingsTextRow`: replace `Text` with slintcn `Label` where appropriate.

### 6. Migrate ui/components/panel_chrome.slint
- Replace `TextButton` import with slintcn `Button` (`ButtonVariant.ghost`) for the "Done" button in `PanelHeader`.
- Replace the inner `Card` component with slintcn `Card` from `slintcn/components/card.slint`.
- Replace `Text` with slintcn `Label` for titles.

### 7. Migrate ui/components/info_banner.slint
- Keep the current `BannerSeverity` state coloring.
- Replace inner `Text` usage with slintcn `Label` where appropriate.
- Do not map warning or success to slintcn `Alert` variants; `Alert` only supports default/destructive.

### 8. Migrate all pages in ui/pages/
For each of the 26 page files, apply these substitutions:
- `import { LineEdit, ... } from "std-widgets.slint"` → `import { Input } from "../slintcn/components/input.slint"` (and similarly for other components)
- `LineEdit { placeholder-text: ... }` → `Input { placeholder: ... }` (note: property renamed from `placeholder-text` to `placeholder` in slintcn Input)
- `Spinner { ... }` → keep std `Spinner` or use `Skeleton`; do not use `Progress` as a spinner.
- `ScrollView { mouse-drag-pan-enabled: true; ... }` → `ScrollArea { content-height: ...; ... }`
- `VerticalBox { ... }` → `VerticalLayout { padding: 8px; spacing: 8px; ... }` (VerticalBox is just VerticalLayout with default padding)
- `PrimaryButton { label: "..." }` → `Button { text: "..."; }` (property renamed from `label` to `text`)
- `TextButton { label: "..." }` → `Button { text: "..."; variant: ButtonVariant.ghost; }`
- `DestructiveButton { label: "..." }` → `Button { text: "..."; variant: ButtonVariant.destructive; }`

Priority pages to migrate first (most component usage):
1. `ui/pages/settings_page.slint` — uses LineEdit, Spinner, ScrollView, PrimaryButton, DestructiveButton, SettingsSection, SettingsValueRow, SettingsToggleRow
2. `ui/pages/audio_page.slint`, `ui/pages/camera_page.slint`, `ui/pages/mixer_page.slint` — heavy slider/toggle usage
3. `ui/pages/bitrate_preset_edit_page.slint`, `ui/pages/macro_edit_page.slint` — LineEdit/Input usage
4. All remaining pages

### 9. Audit and remove std-widgets dependency
After all pages are migrated, run `grep -r "std-widgets" ui/` and remove raw page imports where slintcn has a verified equivalent. The vendored `ui/components/std/` chain remains intact for `ui/components/mcore/common.slint`.

### 10. Build and fix type errors
Run `cargo build` (or `cargo check`). Fix any property name mismatches surfaced by the Slint compiler. Common issues:
- `label` vs `text` on Button
- `placeholder-text` vs `placeholder` on Input
- ScrollArea API differences vs ScrollView
- slintcn Slider may use `step` instead of no step (verify against slintcn docs)
