# Step 10 — Audit + remove leftover std-widgets

← [Step 9: Page order](09-page-order.md) · [Index](README.md) · Next → [Step 11: Validate](11-validate.md)

## Audit command

```bash
grep -rn "std-widgets" ui/pages ui/components/buttons.slint \
  ui/components/settings_rows.slint ui/components/control_bar.slint
```

## Goal state

Only **intentional** keeps remain:

- `Spinner` imports — no slintcn spinner equivalent (Progress is a value bar, not indeterminate).
- `ListView` imports — no slintcn ListView in the installed set.
- `ComboBox` imports — unless you added slintcn `Select`/`Combobox` in Step 1.

## Do NOT touch

```
ui/components/std/      ← separately vendored Slint std-widgets (pinned upstream commit)
ui/components/mcore/    ← separately vendored mirroring_core helpers
ui/components/VENDORING.md
```

These are governed by `VENDORING.md` with their own pinned source commit
(`63980e6736e65adbd15588d21903d0c02223c15c`). `mcore/common.slint` imports from
`../std/std-widgets.slint`; leave that chain intact.

## Sanity check: no accidental edits to vendored sets

```bash
git status --short ui/components/std ui/components/mcore
# should print nothing
```

← [Step 9: Page order](09-page-order.md) · [Index](README.md) · Next → [Step 11: Validate](11-validate.md)
