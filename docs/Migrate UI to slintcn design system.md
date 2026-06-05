Based on my analysis of the repository, here is the complete mapping and migration plan.

## Current Design System Inventory

The project uses a fully custom design system:

| Current File | Exports | slintcn Equivalent |
|---|---|---|
| `ui/theme.slint` | `Theme` global (colors, spacing, radii) | slintcn theme tokens (`ui/slintcn/theme/`) |
| `ui/components/buttons.slint` | `PrimaryButton`, `TextButton`, `DestructiveButton`, `LoadingView` | `Button` (variants), `Progress`/`Skeleton` |
| `ui/components/settings_rows.slint` | `SettingsSection`, `SettingsValueRow`, `SettingsToggleRow`, `SettingsTextRow`, `SettingsSliderRow` | `Card`, `Switch`, `Slider`, `Separator`, `Label` |
| `ui/components/panel_chrome.slint` | `PanelHeader`, `PanelHeaderActions`, `Card`, `FormRow` | `Sheet`, `Card` |
| `ui/components/info_banner.slint` | `InfoBanner` | `Alert` or `Toast` |
| `std-widgets.slint` | `LineEdit`, `CheckBox`, `Slider`, `ScrollView`, `Spinner` | `Input`, `Checkbox`, `Slider`, `ScrollArea`, `Progress` |

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

Then run the CLI to add all needed components:

```bash
node slintcn.mjs add \
  button card input badge separator label \
  dialog alert-dialog sheet tooltip toast \
  checkbox switch icon slider \
  progress skeleton alert scroll-area \
  select tabs toggle
```

### Step 2 — Wire slintcn into `build.rs`

```rust
// build.rs
use std::fs;
use std::process::Command;

fn main() {
    let _ = fs::remove_dir_all("ui/slintcn");

    let status = Command::new("node")
        .arg("slintcn.mjs")
        .args([
            "add",
            "button", "card", "input", "badge", "separator", "label",
            "dialog", "alert-dialog", "sheet", "tooltip", "toast",
            "checkbox", "switch", "icon", "slider",
            "progress", "skeleton", "alert", "scroll-area",
            "select", "tabs", "toggle",
        ])
        .status()
        .expect("node slintcn.mjs failed — need Node 20+ on PATH");
    assert!(status.success(), "slintcn add failed");

    println!("cargo:rerun-if-changed=ui/slintcn.json");
    slint_build::compile("ui/main.slint").expect("Slint compile failed");
}
```

---

### Step 3 — Migrate `ui/theme.slint` → slintcn theme tokens

slintcn generates `ui/slintcn/theme/tokens.slint`. You **keep** `ui/theme.slint` only for app-specific tokens (control-bar-height, safe-area helpers, log colors) and re-map color tokens to slintcn's palette.

```slint
// ui/theme.slint — App-specific tokens only. Colors now come from slintcn.
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

Replace all three custom buttons with slintcn `Button` variants. The `LoadingView` uses slintcn `Progress`.

```slint
// ui/components/buttons.slint
import { Button }   from "../slintcn/components/button.slint";
import { Progress } from "../slintcn/components/progress.slint";
import { Label }    from "../slintcn/components/label.slint";

// PrimaryButton → Button (default variant)
export component PrimaryButton inherits Button {
    // Button already has: label (text), enabled, clicked()
    // variant defaults to "default" (filled primary color)
}

// TextButton → Button ghost variant
export component TextButton inherits Button {
    variant: "ghost";
}

// DestructiveButton → Button destructive variant
export component DestructiveButton inherits Button {
    variant: "destructive";
}

// LoadingView → indeterminate Progress + Label
export component LoadingView inherits VerticalLayout {
    in property <string> label: @tr("Loading");
    alignment: center;
    spacing: 8px;

    Progress {
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

---

### Step 5 — Migrate `ui/components/settings_rows.slint`

```slint
// ui/components/settings_rows.slint
import { Card }      from "../slintcn/components/card.slint";
import { Switch }    from "../slintcn/components/switch.slint";
import { Slider }    from "../slintcn/components/slider.slint";
import { Separator } from "../slintcn/components/separator.slint";
import { Label }     from "../slintcn/components/label.slint";
import { Theme }     from "../theme.slint";

// ── SettingsSection — Card wrapping rows with a section title ─────────────
export component SettingsSection inherits VerticalLayout {
    in property <string> title: "";
    spacing: 6px;
    vertical-stretch: 0;
    alignment: start;

    if root.title != "": Label {
        text: root.title;
        variant: "muted";   // slintcn Label muted variant = secondary color
        font-size: 9pt;
        font-weight: 600;
        padding-left: Theme.padding-screen;
    }

    Card {
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
        if root.value != "": Label { text: root.value; variant: "muted"; vertical-alignment: center; }
        if root.show-chevron: Label { text: "›"; variant: "muted"; font-size: 15pt; vertical-alignment: center; }
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
            enabled: root.enabled;
            toggled(v) => { root.toggled(v); }
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
        if root.subtitle != "": Label { text: root.subtitle; variant: "muted"; font-size: 9pt; }
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
                variant: "muted";
                vertical-alignment: center;
            }
        }
        Slider {
            minimum: root.minimum;
            maximum: root.maximum;
            value <=> root.value;
            changed(v) => { root.changed(v); }
        }
    }
}
```

---

### Step 6 — Migrate `ui/components/panel_chrome.slint`

```slint
// ui/components/panel_chrome.slint
import { Card }   from "../slintcn/components/card.slint";
import { Button } from "../slintcn/components/button.slint";
import { Label }  from "../slintcn/components/label.slint";
import { Theme }  from "../theme.slint";

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
            variant: "ghost";
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

// Card — re-export slintcn Card with standard padding
export component Card inherits Card {  // slintcn Card
    // slintcn Card already provides background, border-radius, clip
    // Override padding to match app conventions
    padding-left:   Theme.padding-screen;
    padding-right:  Theme.padding-screen;
    padding-top:    12px;
    padding-bottom: 12px;
}

// FormRow — vertical label + control pair
export component FormRow inherits VerticalLayout {
    in property <string> label;
    spacing: 4px;
    Label { text: root.label; variant: "muted"; font-size: 9pt; }
    @children
}
```

---

### Step 7 — Migrate `ui/components/info_banner.slint`

Replace with slintcn `Alert` for inline banners, or `Toast` for transient notifications.

```slint
// ui/components/info_banner.slint
import { Alert }  from "../slintcn/components/alert.slint";
import { Bridge, BannerSeverity } from "../bridge.slint";

export component InfoBanner inherits Rectangle {
    in property <string>         message:  Bridge.banner-message;
    in property <BannerSeverity> severity: Bridge.banner-severity;
    in-out property <bool>       shown:    Bridge.banner-visible;

    height: root.shown ? 40px : 0px;
    clip: true;
    animate height { duration: 200ms; easing: ease-out; }

    accessible-role:  text;
    accessible-label: root.message;

    // Map BannerSeverity → slintcn Alert variant
    Alert {
        width: parent.width;
        height: parent.height;
        variant: root.severity == BannerSeverity.error   ? "destructive"
               : root.severity == BannerSeverity.warning ? "warning"
               : root.severity == BannerSeverity.success ? "success"
               :                                           "default";
        description: root.message;
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
| `Spinner` | `Progress` (indeterminate) | `slintcn/components/progress.slint` |
| `VerticalBox` | plain `VerticalLayout` | (no import needed) |

Example — `ui/pages/settings_page.slint` receiver section:

```slint
// Before (std-widgets):
import { LineEdit, Spinner, ScrollView } from "std-widgets.slint";

// After (slintcn):
import { Input }    from "../slintcn/components/input.slint";
import { Progress } from "../slintcn/components/progress.slint";
import { ScrollArea } from "../slintcn/components/scroll-area.slint";

// Usage — LineEdit → Input
ip-field := Input {
    placeholder-text: @tr("Receiver IP address");
}

// Usage — Spinner → Progress (indeterminate)
Progress {
    indeterminate: true;
    width: 20px;
    height: 20px;
}

// Usage — ScrollView → ScrollArea
ScrollArea {
    mouse-drag-pan-enabled: true;
    // content goes here
}
```

---

### Step 9 — Page-by-page migration checklist

For each page in `ui/pages/`, apply the following substitutions:

```
PrimaryButton    → Button (default)
TextButton       → Button (variant: "ghost")
DestructiveButton→ Button (variant: "destructive")
LoadingView      → Progress (indeterminate) + Label
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
Spinner          → Progress (indeterminate)
VerticalBox      → VerticalLayout
```

Pages to update (all 26 in `ui/pages/`): [0-cite-0](#0-cite-0) [0-cite-1](#0-cite-1) [0-cite-2](#0-cite-2) 

---

### Step 10 — Remove `std-widgets.slint` dependency entirely

After all pages are migrated, audit for any remaining `std-widgets.slint` imports:

```bash
grep -r "std-widgets" ui/
```

Remove each remaining import and replace with the slintcn equivalent.

---

### Step 11 — Validate the build

```bash
cargo build --target aarch64-linux-android
```

The slintcn CLI regenerates `ui/slintcn/` on every `cargo build` via `build.rs`. Fix any type mismatches (e.g., slintcn `Button.text` vs old `PrimaryButton.label` — the property name changes).

---

## Key Property Name Changes to Watch

| Old component | Old property | slintcn property |
|---|---|---|
| `PrimaryButton` | `label` | `text` (Button) |
| `TextButton` | `label` | `text` (Button) |
| `DestructiveButton` | `label` | `text` (Button) |
| `CheckBox` | `checked` | `checked` (same) |
| `Slider` | `value` | `value` (same) |
| `LineEdit` | `text`, `placeholder-text` | `value`, `placeholder` (Input) |
| `ScrollView` | `mouse-drag-pan-enabled` | check slintcn ScrollArea API |

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
Modify `build.rs` to invoke the slintcn CLI before `slint_build::compile`. Add a `fs::remove_dir_all("ui/slintcn")` step first, then run `node slintcn.mjs add button card input badge separator label dialog alert-dialog sheet tooltip toast checkbox switch icon slider progress skeleton alert scroll-area select tabs toggle`. Add `println!("cargo:rerun-if-changed=slintcn.json")`.

### 3. Update ui/theme.slint
Import `Palette` from `slintcn/theme/tokens.slint`. Keep only app-specific layout tokens (control-bar-height, header-height, row-height, padding-screen, thumbnail-width, qr-square-min, qr-square-max, log level colors). Replace color token values with aliases to `Palette.*` properties (surface-card → Palette.card, text-primary → Palette.foreground, text-secondary → Palette.muted-foreground, accent → Palette.primary, error → Palette.destructive, etc.).

### 4. Migrate ui/components/buttons.slint
- `PrimaryButton`: inherit slintcn `Button` from `slintcn/components/button.slint`. Change `label` property to `text` (slintcn Button uses `text`). Default variant is filled/primary.
- `TextButton`: inherit slintcn `Button` with `variant: "ghost"`.
- `DestructiveButton`: inherit slintcn `Button` with `variant: "destructive"`.
- `LoadingView`: replace `Spinner` (std-widgets) with slintcn `Progress` (indeterminate: true), replace `Text` label with slintcn `Label`.

### 5. Migrate ui/components/settings_rows.slint
- Remove `import { CheckBox, Slider } from "std-widgets.slint"`.
- Import `Card`, `Switch`, `Slider`, `Separator`, `Label` from slintcn.
- `SettingsSection`: wrap children in slintcn `Card` instead of a plain `Rectangle`. Use slintcn `Label` with muted variant for the section title.
- `SettingsToggleRow`: replace `CheckBox` with slintcn `Switch`. Keep the same `checked <=>` binding and `toggled` callback.
- `SettingsSliderRow`: replace std-widgets `Slider` with slintcn `Slider`. API is identical (minimum, maximum, value, changed callback).
- `SettingsValueRow` and `SettingsTextRow`: replace `Text` with slintcn `Label` where appropriate.

### 6. Migrate ui/components/panel_chrome.slint
- Replace `TextButton` import with slintcn `Button` (variant: "ghost") for the "Done" button in `PanelHeader`.
- Replace the inner `Card` component with slintcn `Card` from `slintcn/components/card.slint`.
- Replace `Text` with slintcn `Label` for titles.

### 7. Migrate ui/components/info_banner.slint
- Import slintcn `Alert` from `slintcn/components/alert.slint`.
- Replace the inner `HorizontalLayout + Text` with an `Alert` component.
- Map `BannerSeverity.error` → `variant: "destructive"`, `warning` → `"warning"`, `success` → `"success"`, `info` → `"default"`.

### 8. Migrate all pages in ui/pages/
For each of the 26 page files, apply these substitutions:
- `import { LineEdit, ... } from "std-widgets.slint"` → `import { Input } from "../slintcn/components/input.slint"` (and similarly for other components)
- `LineEdit { placeholder-text: ... }` → `Input { placeholder: ... }` (note: property renamed from `placeholder-text` to `placeholder` in slintcn Input)
- `Spinner { indeterminate: true; }` → `Progress { indeterminate: true; }`
- `ScrollView { mouse-drag-pan-enabled: true; ... }` → `ScrollArea { ... }`
- `VerticalBox { ... }` → `VerticalLayout { padding: 8px; spacing: 8px; ... }` (VerticalBox is just VerticalLayout with default padding)
- `PrimaryButton { label: "..." }` → `Button { text: "..."; }` (property renamed from `label` to `text`)
- `TextButton { label: "..." }` → `Button { text: "..."; variant: "ghost"; }`
- `DestructiveButton { label: "..." }` → `Button { text: "..."; variant: "destructive"; }`

Priority pages to migrate first (most component usage):
1. `ui/pages/settings_page.slint` — uses LineEdit, Spinner, ScrollView, PrimaryButton, DestructiveButton, SettingsSection, SettingsValueRow, SettingsToggleRow
2. `ui/pages/audio_page.slint`, `ui/pages/camera_page.slint`, `ui/pages/mixer_page.slint` — heavy slider/toggle usage
3. `ui/pages/bitrate_preset_edit_page.slint`, `ui/pages/macro_edit_page.slint` — LineEdit/Input usage
4. All remaining pages

### 9. Audit and remove std-widgets dependency
After all pages are migrated, run `grep -r "std-widgets" ui/` and remove any remaining imports. The goal is zero `std-widgets.slint` imports in the final state.

### 10. Build and fix type errors
Run `cargo build` (or `cargo check`). Fix any property name mismatches surfaced by the Slint compiler. Common issues:
- `label` vs `text` on Button
- `placeholder-text` vs `placeholder` on Input
- ScrollArea API differences vs ScrollView
- slintcn Slider may use `step` instead of no step (verify against slintcn docs)

