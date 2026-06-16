# Step 3 — Migrate `ui/theme.slint`

← [Step 2: build.rs](02-build-rs.md) · [Index](README.md) · Next → [Step 4: Buttons](04-buttons.md)

Keep `Theme` as the single global the whole app imports (changing its name touches every file).
Re-point **color** tokens at the slintcn palette; keep every **layout / typography / log** token
exactly as-is.

> Replace `Palette` and the import path below with the **real** values you recorded in
> [Step 1's verification gate](01-install.md#1c-verification-gate-do-this-before-any-code-change).

## Full file (after)

```slint
// theme.slint — colors now alias slintcn palette; layout tokens unchanged.
//
// NOTE: confirm the import path + global name against the generated
//       ui/slintcn/theme/ output (Step 1 verification gate).
import { Palette } from "slintcn/theme/tokens.slint";   // ← adjust to real path/name

export global Theme {
    // ── Surfaces (aliased to slintcn) ────────────────────────────────────
    out property <color> surface-primary:  transparent;          // app-specific, keep
    out property <color> surface-black:     #000000;              // app-specific, keep
    out property <color> surface-card:      Palette.card;
    out property <color> surface-bar:       Palette.background;
    out property <color> surface-overlay:   #1f2937cc;            // app-specific, keep
    out property <color> scrim-strong:      #00000080;
    out property <color> scrim-light:       #00000040;

    // ── Text ─────────────────────────────────────────────────────────────
    out property <color> text-primary:      Palette.foreground;
    out property <color> text-secondary:    Palette.muted-foreground;
    out property <color> text-disabled:     Palette.muted-foreground;
    out property <color> text-on-accent:    Palette.primary-foreground;
    out property <color> text-on-error:     Palette.destructive-foreground;

    // ── Accent / interactive ─────────────────────────────────────────────
    out property <color> accent:            Palette.primary;
    out property <color> accent-muted:      Palette.muted;
    out property <color> accent-active:     Palette.primary;
    out property <color> accent-pressed:    Palette.primary;     // or .darker(20%) at call site

    // ── Severity ─────────────────────────────────────────────────────────
    // slintcn has only one "destructive". Keep the brighter/darker pairs the
    // app relies on (StatusPill/Badge legibility) as literal tokens.
    out property <color> error:             Palette.destructive;
    out property <color> error-fg:          #ef4444;
    out property <color> warning:           #ed6c02;
    out property <color> warning-fg:        #ed6c02;
    out property <color> success:           #2e7d32;
    out property <color> recording-dot:     #cc0000;

    // ── Typography (UNCHANGED — copy verbatim from current theme.slint) ──
    out property <length> font-size-label:    9pt;
    out property <length> font-size-body:     12pt;
    out property <length> font-size-heading:  15pt;
    out property <length> font-size-icon:     11pt;
    out property <length> font-size-display:  36pt;
    out property <length> font-size-hero:     54pt;
    out property <length> font-size-cell:     15pt;

    // ── Spacing (UNCHANGED) ──────────────────────────────────────────────
    out property <length> padding-screen:   12px;
    out property <length> padding-card:      8px;
    out property <length> spacing-default:   8px;
    out property <length> spacing-tight:     4px;
    out property <length> spacing-loose:    16px;

    // ── Shape (UNCHANGED) ────────────────────────────────────────────────
    out property <length> radius-card:       8px;
    out property <length> radius-pill:        6px;
    out property <length> radius-circle:    9999px;
    out property <length> row-height:        48px;
    out property <length> row-height-comfortable: 56px;
    out property <length> row-height-compact:     40px;
    out property <length> control-bar-height: 72px;
    out property <length> header-height:    56px;
    out property <length> thumbnail-width:  200px;
    out property <length> qr-square-min:    240px;
    out property <length> qr-square-max:    360px;

    // ── Icon badge backgrounds (UNCHANGED) ───────────────────────────────
    out property <color> icon-bg-neutral:  #374151;

    // ── Debug log level colors (UNCHANGED) ───────────────────────────────
    out property <color> log-trace:   #888888;
    out property <color> log-debug:   #4080ff;
    out property <color> log-info:    #20a020;
    out property <color> log-warning: #f0a020;
    out property <color> log-error:   #e02020;

    // ── Elevation (UNCHANGED) ────────────────────────────────────────────
    out property <length> elevation-1-blur:    4px;
    out property <length> elevation-2-blur:    8px;
    out property <length> elevation-3-blur:   16px;
}
```

## Risk / tight loop

If a slintcn palette token doesn't exist under the guessed name, the compile fails fast with a
clear "unknown property" error. Migrate colors **one token at a time** and run `cargo check`
between changes if you want a fast feedback loop. Only the `Palette.*` lines are uncertain — every
literal-value token is a verbatim copy and is safe.

← [Step 2: build.rs](02-build-rs.md) · [Index](README.md) · Next → [Step 4: Buttons](04-buttons.md)
