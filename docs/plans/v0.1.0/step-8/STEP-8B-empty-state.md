# STEP-8B — Empty state

> The "no receivers found" block, shown when `Bridge.devices` is empty. Append
> into the outer `VerticalLayout` from [STEP-8A](STEP-8A-scaffold-and-header.md),
> after the header.

---

## Goal

A centred empty state (icon + title + hint) gated on `Bridge.devices.length == 0`.

---

## The change — append after the header

**File:** `ui/pages/connect_page.slint`

```slint
        // ── Empty state ───────────────────────────────────────────────────────
        if Bridge.devices.length == 0: VerticalLayout {
            alignment: center;
            spacing: Theme.spacing-default;
            min-height: 200px;

            Text {
                text: "📡";
                font-size: 36pt;
                horizontal-alignment: center;
            }
            Text {
                text: @tr("No receivers found");
                color: Theme.text-primary;
                font-size: Theme.font-size-body;
                font-weight: 600;
                horizontal-alignment: center;
            }
            Text {
                text: @tr("Make sure your FCast receiver is on the same network and running.");
                color: Theme.text-secondary;
                font-size: Theme.font-size-body;
                horizontal-alignment: center;
                wrap: word-wrap;
            }
        }
```

---

## Notes

- **`if Bridge.devices.length == 0`** — Slint conditional element; it mounts only
  when the model is empty and is mutually exclusive with 8C's
  `if … length > 0`.
- **`min-height: 200px`** gives the empty state vertical presence so the centred
  content doesn't collapse against the header.
- No slintcn component here — plain `Text` is the right tool for static copy
  (slintcn `Empty` exists in the registry but isn't installed, and three Text
  nodes are lighter than pulling it in).

---

## Next

→ [STEP-8C-receiver-list.md](STEP-8C-receiver-list.md)
