# Step 0 — Corrections to the original research

← [Index](README.md) · Next → [Step 1: Install](01-install.md)

The original research doc (`docs/Migrate UI to slintcn design system.md`) was written from
assumptions. After querying the real registry via the slintcn MCP, these are **wrong** and are
fixed throughout this plan.

| Original claim | Reality (verified via MCP) |
|---|---|
| Install via `node slintcn.mjs add …` in `build.rs` | Install via **`npx slintcn@latest add …`** once, at the terminal. Files are **vendored** into `ui/slintcn/` (same pattern as the existing `ui/components/std/`). `build.rs` is **not** modified. |
| `Button { variant: "ghost"; }` (string) | `Button { variant: ButtonVariant.ghost; }` — an **enum** `ButtonVariant`. Same for `ButtonSize`. |
| `Label { variant: "muted"; }` (string) | `Label { variant: LabelVariant.muted; }` — enum `LabelVariant` (`default` / `muted` / `required`). |
| `Input { placeholder; value; }` | `Input { placeholder; text <=>; edited(t) => … }` — the text property is **`text`**, not `value`. |
| `Progress { indeterminate: true; }` for spinners | **Progress has no `indeterminate`** — it is a 0–100 value bar. The `Spinner` mapping must use the **vendored `std/spinner.slint`** or slintcn **`Skeleton`**. |
| `Alert` variants: `destructive` / `warning` / `success` / `default` | **Alert only has `AlertVariant.default` and `AlertVariant.destructive`.** Warning/success must keep the current custom coloring, not Alert variants. |
| `ScrollArea { mouse-drag-pan-enabled: true; }` | `ScrollArea { content-height: <px>; }` — you must supply the total scrollable height. No `mouse-drag-pan-enabled`. |
| `Card` padding overridden via `padding-left` etc. | `Card { variant: CardVariant.solid; }` exposes layout props `padding-l` / `gap-l`. Pad an inner `VerticalLayout`, don't set `padding-left` on the Card. |
| Theme global is `Palette` at `slintcn/theme/tokens.slint` | **Unverified** — the exact global name/path is only known after install. Inspect the generated `ui/slintcn/theme/`. |
| Pages import a vendored std-widgets | Pages import Slint's **built-in** `"std-widgets.slint"`. The vendored `ui/components/std/` is used **only** by `ui/components/mcore/common.slint`, per `VENDORING.md`. Leave `std/` alone. |
| `Switch.toggled` / `Slider.changed` callbacks exist | **Unverified.** The registry usage snippets show only `checked <=>` / `value <=>`. Confirm callback names in the generated source before wiring `toggled()` / `changed()`. |

## Verified component APIs (source of truth for every snippet)

```slint
// Button
import { Button, ButtonVariant, ButtonSize } from "slintcn/components/button.slint";
Button { variant: ButtonVariant.default; size: ButtonSize.lg; text: "Ship it"; clicked => {} }
//   ButtonVariant: default | destructive | ghost | … (8 variants)

// Card
import { Card, CardVariant } from "slintcn/components/card.slint";
Card { variant: CardVariant.solid; VerticalLayout { padding: parent.padding-l; spacing: parent.gap-l; } }

// Input
import { Input } from "slintcn/components/input.slint";
Input { placeholder: "you@example.com"; text <=> email; edited(t) => {} }

// Label
import { Label, LabelVariant } from "slintcn/components/label.slint";
Label { text: "Email"; variant: LabelVariant.required; }   // default | muted | required

// Switch
import { Switch } from "slintcn/components/switch.slint";
Switch { label: "Dark mode"; checked <=> dark; }

// Slider
import { Slider } from "slintcn/components/slider.slint";
Slider { value <=> volume; minimum: 0; maximum: 100; }

// Checkbox
import { Checkbox } from "slintcn/components/checkbox.slint";
Checkbox { label: "Accept terms"; checked <=> accepted; }

// Progress — 0..100, NOT indeterminate
import { Progress } from "slintcn/components/progress.slint";
Progress { value: 64; }

// Alert — only default | destructive
import { Alert, AlertVariant } from "slintcn/components/alert.slint";
import { LucidePaths } from "slintcn/components/lucide-paths.slint";
Alert { icon: LucidePaths.check; title: "Deployed"; description: "Your changes are live."; }

// Separator
import { Separator, SeparatorOrientation } from "slintcn/components/separator.slint";
Separator { orientation: SeparatorOrientation.horizontal; }

// ScrollArea — needs content-height
import { ScrollArea } from "slintcn/components/scroll-area.slint";
ScrollArea { content-height: 480px; VerticalLayout { height: 480px; } }
```

← [Index](README.md) · Next → [Step 1: Install](01-install.md)
