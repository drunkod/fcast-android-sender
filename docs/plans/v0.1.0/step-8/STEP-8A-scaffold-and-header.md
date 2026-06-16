# STEP-8A — Scaffold & header

> Replace the 5-line placeholder with the `ConnectView` shell: imports, the
> outer `VerticalLayout`, and the "Cast to…" header. 8B–8D append into it.

---

## Goal

Stand up `connect_page.slint` with all imports and the header, ready for the
empty state (8B), list (8C), and manual button (8D).

---

## What this replaces

The current placeholder:

```slint
// connect_page.slint — Transparent placeholder
export component ConnectView inherits Rectangle {
    background: transparent;
}
```

> The component **name stays `ConnectView`** — it's already consumed by the app
> state machine, so no `main.slint` import change is needed (see 8D).

---

## The change — shell + header

**Replace** `ui/pages/connect_page.slint` with:

```slint
// ui/pages/connect_page.slint
// Replaces the 5-line transparent placeholder with a functional receiver list.
import { Theme } from "../theme.slint";
import { Bridge } from "../bridge.slint";
import { RowColors } from "../components/settings_rows.slint";
import { Button, ButtonVariant, ButtonSize } from "../slintcn/components/button.slint";
import { Badge, BadgeVariant, BadgeSize } from "../slintcn/components/badge.slint";
import { Card, CardVariant, CardPadding } from "../slintcn/components/card.slint";

export component ConnectView inherits Rectangle {
    background: transparent;

    VerticalLayout {
        padding: Theme.padding-screen;
        spacing: Theme.spacing-loose;
        alignment: start;

        // ── Header ───────────────────────────────────────────────────────────
        Text {
            text: @tr("Cast to…");
            color: Theme.text-primary;
            font-size: Theme.font-size-heading;
            font-weight: 700;
        }

        // 8B → empty state (if devices.length == 0)
        // 8C → receiver list (if devices.length > 0)
        // 8D → manual-connect Button
    }
}
```

---

## Import notes

- **`RowColors`** (from `settings_rows.slint`) — for the row press/normal hex in
  8C, instead of hardcoding `#1e2535`/`#2a3347`.
- **`BadgeSize`** is imported here (alongside `Badge`, `BadgeVariant`) because
  8C uses `BadgeSize.sm` for the compact "Default" tag — importing only
  `Badge, BadgeVariant` would fail to resolve `BadgeSize`.
- **`Button`/`ButtonVariant`/`ButtonSize`** — for 8D's manual-connect button.

---

## Next

→ [STEP-8B-empty-state.md](STEP-8B-empty-state.md)
