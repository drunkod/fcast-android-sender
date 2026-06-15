# Step 11 — Build + verification checklist

← [Step 10: Audit](10-audit.md) · [Index](README.md)

## Build

```bash
cargo check                                   # host build, fastest loop
cargo build --target aarch64-linux-android    # the real target
```

Run the UI snapshot tests if present (host build has Slint debug info enabled for this — see
`build.rs`):

```bash
cargo test --test ui_snapshots
```

## Expected compiler-surfaced fixes

| Symptom | Cause | Fix |
|---|---|---|
| `unknown property 'card'` in theme | wrong palette token name | read `ui/slintcn/theme/*.slint`, use real names ([Step 3](03-theme.md)) |
| `unknown element 'Palette'` / bad import | wrong theme import path/global | adjust the import to the recorded name ([Step 1](01-install.md)) |
| `unknown property 'variant'` expects enum | passed a string | use `ButtonVariant.*` / `LabelVariant.*` / `CardVariant.*` |
| `unknown callback 'toggled'`/`'changed'` | guessed callback name | read generated `switch.slint`/`slider.slint` ([Step 5](05-settings-rows.md)) |
| `unknown property 'indeterminate'` on Progress | tried to spin Progress | keep `Spinner` / use `Skeleton` ([Step 8](08-pages-std-widgets.md)) |
| ScrollArea shows nothing / clipped | missing `content-height` | bind to child `preferred-height` ([Step 8](08-pages-std-widgets.md)) |
| name clash on `inherits Button`+`label` | redeclaring inherited prop | use composition fallback ([Step 4](04-buttons.md)) |

## Open verification checklist (resolve during Step 1)

- [ ] Real theme global **name** and **import path** (`ui/slintcn/theme/`).
- [ ] slintcn `Button`: does it expose `enabled`? exact `clicked` signature?
- [ ] slintcn `Switch`: callback name (`toggled` vs `changed`?), `enabled` support.
- [ ] slintcn `Slider`: callback name, `step` support.
- [ ] slintcn `Input`: `edited` argument shape.
- [ ] slintcn `Card`: padding props (`padding-l`/`gap-l`?) and `CardVariant` values.
- [ ] Whether inheriting slintcn components while redeclaring `label` clashes (else use composition).

## What this migration deliberately does NOT do

- Does **not** modify `build.rs` (no node codegen — files are vendored).
- Does **not** touch `ui/components/std/` or `ui/components/mcore/` (separate vendored set).
- Does **not** replace `InfoBanner` with `Alert` (severity model mismatch).
- Does **not** rewrite the 132 button/row **call sites** — wrappers preserve their APIs.
- Does **not** map `Spinner` → `Progress` (Progress is not indeterminate).

← [Step 10: Audit](10-audit.md) · [Index](README.md)
