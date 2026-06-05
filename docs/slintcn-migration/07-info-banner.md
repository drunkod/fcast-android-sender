# Step 7 — Migrate `ui/components/info_banner.slint`

← [Step 6: Panel chrome](06-panel-chrome.md) · [Index](README.md) · Next → [Step 8: Pages std-widgets](08-pages-std-widgets.md)

**Keep the current `states`-driven coloring.** slintcn `Alert` only has `AlertVariant.default` /
`AlertVariant.destructive` — it **cannot** express the app's 4-way `error / warning / success /
info` severity. Do **not** force `InfoBanner` into `Alert`. The only change is `Text` → `Label`;
the severity logic stays untouched.

## Full file (after)

```slint
import { Theme } from "../theme.slint";
import { Bridge, BannerSeverity } from "../bridge.slint";
import { Label } from "../slintcn/components/label.slint";

export component InfoBanner inherits Rectangle {
    in property <string>          message: Bridge.banner-message;
    in property <BannerSeverity>  severity: Bridge.banner-severity;
    in-out property <bool>        shown: Bridge.banner-visible;

    height: root.shown ? 40px : 0px;
    clip: true;
    accessible-role:  text;
    accessible-label: root.message;

    private property <color> banner-bg: Theme.accent-active.darker(20%);
    states [
        error   when root.severity == BannerSeverity.error   : { banner-bg: Theme.error;   }
        warning when root.severity == BannerSeverity.warning : { banner-bg: Theme.warning; }
        success when root.severity == BannerSeverity.success : { banner-bg: Theme.success; }
        info    when root.severity == BannerSeverity.info    : { banner-bg: Theme.accent-active.darker(20%); }
    ]
    background: root.banner-bg;
    animate height     { duration: 200ms; easing: ease-out;    }
    animate background { duration: 150ms; easing: ease-in-out; }

    HorizontalLayout {
        padding-left:  Theme.padding-screen;
        padding-right: Theme.padding-screen;
        Label {
            text: root.message;
            color: white;
            vertical-alignment: center;
            font-size: Theme.font-size-label;
        }
    }
}
```

## When `Alert` IS appropriate

For genuinely 2-state callouts (e.g. a destructive-confirm inline banner), use slintcn `Alert`
directly at that site — not as the `InfoBanner` replacement:

```slint
import { Alert, AlertVariant } from "../slintcn/components/alert.slint";
import { LucidePaths } from "../slintcn/components/lucide-paths.slint";

Alert {
    variant: AlertVariant.destructive;
    icon: LucidePaths.triangle-alert;     // confirm exact path name in lucide-paths.slint
    title: @tr("This cannot be undone");
    description: @tr("All presets will be deleted.");
}
```

← [Step 6: Panel chrome](06-panel-chrome.md) · [Index](README.md) · Next → [Step 8: Pages std-widgets](08-pages-std-widgets.md)
