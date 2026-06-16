# Step 5 — Migrate `ui/components/settings_rows.slint`

← [Step 4: Buttons](04-buttons.md) · [Index](README.md) · Next → [Step 6: Panel chrome](06-panel-chrome.md)

Swap the two `std-widgets` primitives (`CheckBox`, `Slider`) for slintcn, wrap the section in a
slintcn `Card`, and replace `Text` → `Label`. Keep every component's property/callback surface
identical so call sites don't change.

## Two verification points (both flagged inline below)

The **callback names** on slintcn `Switch` and `Slider` are not documented in the registry usage
snippets. Open `ui/slintcn/components/switch.slint` / `slider.slint` and read the `callback`
declarations before wiring `toggled()` / `changed()`. (Recorded in
[Step 1](01-install.md#1c-verification-gate-do-this-before-any-code-change).)

## Full file (after)

```slint
// settings_rows.slint
import { Theme }     from "../theme.slint";
import { Switch }    from "../slintcn/components/switch.slint";
import { Slider }    from "../slintcn/components/slider.slint";
import { Label, LabelVariant } from "../slintcn/components/label.slint";
import { Card, CardVariant }   from "../slintcn/components/card.slint";

export global RowColors {
    out property <color> normal:  #1e2535;
    out property <color> pressed: #2a3347;
    out property <color> divider: #3a4154;
}

// ── SettingsSection — wrap rows in a slintcn Card ────────────────────────
export component SettingsSection inherits VerticalLayout {
    in property <string> title: "";
    spacing: 6px;
    vertical-stretch: 0;
    alignment: start;

    if root.title != "": HorizontalLayout {
        padding-left:  Theme.padding-screen;
        padding-right: Theme.padding-screen;
        padding-bottom: 2px;
        Label {
            text: root.title;
            variant: LabelVariant.muted;
            font-size: Theme.font-size-label;
            font-weight: 600;
        }
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

// ── SettingsValueRow — Text → Label, structure unchanged ─────────────────
export component SettingsValueRow inherits Rectangle {
    in property <string> icon: "";
    in property <color>  icon-bg: Theme.icon-bg-neutral;
    in property <string> title;
    in property <string> value: "";
    in property <bool>   enabled: true;
    in property <bool>   show-chevron: true;
    callback clicked();

    height: Theme.row-height;
    opacity: root.enabled ? 1.0 : 0.45;
    background: ta.pressed ? RowColors.pressed : RowColors.normal;

    accessible-role:    button;
    accessible-label:   root.title + (root.value != "" ? ", " + root.value : "");
    accessible-enabled: root.enabled;
    accessible-action-default => { if root.enabled { root.clicked(); } }

    ta := TouchArea {
        enabled: root.enabled;
        clicked => { root.clicked(); }
    }
    HorizontalLayout {
        padding-left:  Theme.padding-screen;
        padding-right: Theme.padding-screen;
        spacing: Theme.spacing-default;
        alignment: center;

        if root.icon != "": Rectangle {
            width: 28px; height: 28px;
            border-radius: Theme.radius-pill;
            background: root.icon-bg;
            Text {                              // emoji/symbol badge — plain Text is fine
                text: root.icon;
                font-size: Theme.font-size-icon;
                color: Theme.text-primary;
                horizontal-alignment: center;
                vertical-alignment: center;
            }
        }
        Label { text: root.title; horizontal-stretch: 1; vertical-alignment: center; }
        if root.value != "": Label {
            text: root.value; variant: LabelVariant.muted; vertical-alignment: center;
        }
        if root.show-chevron: Label {
            text: "›"; variant: LabelVariant.muted;
            font-size: Theme.font-size-heading; vertical-alignment: center;
        }
    }
}

// ── SettingsToggleRow — CheckBox → Switch ────────────────────────────────
export component SettingsToggleRow inherits Rectangle {
    in property <string> icon: "";
    in property <color>  icon-bg: Theme.icon-bg-neutral;
    in property <string> title;
    in-out property <bool> checked: false;
    in property <bool> enabled: true;
    callback toggled(bool);

    height: Theme.row-height;
    opacity: root.enabled ? 1.0 : 0.45;
    background: RowColors.normal;

    accessible-role:    switch;
    accessible-label:   root.title;
    accessible-enabled: root.enabled;
    accessible-checked: root.checked;
    accessible-action-default => {
        if root.enabled { root.checked = !root.checked; root.toggled(root.checked); }
    }

    HorizontalLayout {
        padding-left:  Theme.padding-screen;
        padding-right: Theme.padding-screen;
        spacing: Theme.spacing-default;
        alignment: center;

        if root.icon != "": Rectangle {
            width: 28px; height: 28px;
            border-radius: Theme.radius-pill;
            background: root.icon-bg;
            Text {
                text: root.icon; font-size: Theme.font-size-icon;
                color: Theme.text-primary;
                horizontal-alignment: center; vertical-alignment: center;
            }
        }
        Label { text: root.title; horizontal-stretch: 1; vertical-alignment: center; }
        Switch {
            checked <=> root.checked;
            // enabled: root.enabled;            // ← verify Switch exposes `enabled`
            // toggled(v) => { root.toggled(v); } // ← verify callback name; may be `changed`.
            //                                     //   Fallback: a `changed` handler reading
            //                                     //   self.checked.
        }
    }
}

// ── SettingsTextRow — Text → Label ───────────────────────────────────────
export component SettingsTextRow inherits Rectangle {
    in property <string> title;
    in property <string> subtitle: "";
    height: root.subtitle == ""
        ? Theme.row-height
        : Theme.row-height + Theme.font-size-label + Theme.spacing-default;
    background: RowColors.normal;

    VerticalLayout {
        padding-left:  Theme.padding-screen;
        padding-right: Theme.padding-screen;
        alignment: center;
        Label { text: root.title; }
        if root.subtitle != "": Label {
            text: root.subtitle; variant: LabelVariant.muted;
            font-size: Theme.font-size-label;
        }
    }
}

// ── SettingsSliderRow — std Slider → slintcn Slider ──────────────────────
export component SettingsSliderRow inherits Rectangle {
    in property <string> title;
    in property <string> unit: "";
    in property <float>  minimum: 0;
    in property <float>  maximum: 100;
    in property <bool>   show-fractional: false;
    in-out property <float> value: 50;
    callback changed(float);

    height: Theme.row-height * 1.5;
    background: RowColors.normal;

    VerticalLayout {
        padding-left:  Theme.padding-screen;
        padding-right: Theme.padding-screen;
        padding-top:   Theme.spacing-tight;
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
            // changed(v) => { root.changed(v); }  // ← verify callback name in generated source
        }
    }
}
```

## Callback fallback (if names differ)

If slintcn `Switch`/`Slider` don't expose `toggled`/`changed`, drive the wrapper callback from a
`changed` property handler instead:

```slint
Switch {
    checked <=> root.checked;
    changed => { root.toggled(self.checked); }   // whatever the generated callback is
}
```

← [Step 4: Buttons](04-buttons.md) · [Index](README.md) · Next → [Step 6: Panel chrome](06-panel-chrome.md)
