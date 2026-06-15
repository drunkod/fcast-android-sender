# Step 1 — Install the slintcn CLI config + components

← [Step 0: Corrections](00-corrections.md) · [Index](README.md) · Next → [Step 2: build.rs](02-build-rs.md)

## 1a. Create `slintcn.json` at the repo root

Next to `Cargo.toml`. Exact keys depend on CLI version — run `npx slintcn@latest init` first if
available and let it scaffold; otherwise:

```json
{
  "style": "default",
  "baseColor": "neutral",
  "outDir": "ui/slintcn",
  "componentsDir": "ui/slintcn/components",
  "themeDir": "ui/slintcn/theme"
}
```

## 1b. Vendor the components

This is the **real** command (from the MCP `install_command` tool):

```bash
npx slintcn@latest add \
  button card input label separator \
  switch slider checkbox progress alert scroll-area badge
```

This writes `.slint` files into `ui/slintcn/components/` and a theme into `ui/slintcn/theme/`.
**Commit these files** — they are vendored, exactly like `ui/components/std/`.

This repository was installed with slintcn `0.35.0` (recorded in `slintcn.lock.json`). The local
machine did not have `npx` on `PATH`, so the command was run through one-off Node tooling:

```bash
nix-shell -p nodejs --run 'npx slintcn@latest add button card input label separator switch slider checkbox progress alert scroll-area badge'
```

## 1c. Verification gate (do this before any code change)

```bash
ls ui/slintcn/components/      # button.slint, card.slint, … present?
ls ui/slintcn/theme/           # note the exact theme filename
grep -rn "export global" ui/slintcn/theme/      # ← record the real palette global name
grep -rn "callback" ui/slintcn/components/switch.slint ui/slintcn/components/slider.slint
grep -rn "in property\|in-out property" ui/slintcn/components/button.slint
```

Fill these in — every later step depends on them:

| Question | Answer |
|---|---|
| Theme global name | `Tokens` for component tokens; `Theme` for mode; `Palette` for raw colors |
| Theme import path  | `../theme/tokens.slint` from components; app-level import path is `slintcn/theme/tokens.slint` |
| `Switch` callback name (`toggled`? `changed`?) | `toggled(bool)` |
| `Switch` has `enabled`? | No. It exposes `disabled: false`. |
| `Slider` callback name | `changed(float)` |
| `Button` exposes `enabled`? | No. It exposes `disabled: false`. |
| `Input` `edited` arg shape | `edited(string)` with current `text`; also exposes `accepted(string)` |
| `Card` padding props (`padding-l`/`gap-l`?) | Yes. `padding-l` and `gap-l` are read-only outputs; set `card-padding` / `size`. |

## Notes

- `build.rs` stays unchanged (see [Step 2](02-build-rs.md)).
- As long as files live under `ui/` and are imported with correct **relative** paths, the Slint
  compiler picks them up via the existing `slint_build::compile_with_config("ui/main.slint", …)`.
- There is **no** node/codegen build step. The `node slintcn.mjs` approach in the original
  research doc does not apply to this repo.

← [Step 0: Corrections](00-corrections.md) · [Index](README.md) · Next → [Step 2: build.rs](02-build-rs.md)
