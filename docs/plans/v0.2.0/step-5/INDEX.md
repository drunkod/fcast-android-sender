# STEP-5 — Bridge structs, properties, callbacks & Panels (sub-steps)

> The Slint ↔ Rust contract for scenes/widgets. UI-data-model only — safe to land
> independently (unhandled callbacks are no-ops; Rust `Panel` usage is `==`, never
> an exhaustive `match`). **Refs:** PHASE-40/41 §Bridge.

| # | File | Scope |
|---|------|-------|
| 5A | [STEP-5A-panels.md](STEP-5A-panels.md) | `Panel` variants |
| 5B | [STEP-5B-structs-enum.md](STEP-5B-structs-enum.md) | `SceneItem`/`WidgetItem`/`ScenePlacementItem`/`WidgetTypeChoice` |
| 5C | [STEP-5C-properties-callbacks.md](STEP-5C-properties-callbacks.md) | Bridge properties + callbacks |
| 5D | [STEP-5D-bindings-verify.md](STEP-5D-bindings-verify.md) | generated Rust accessor names + verify |

→ Next: [../step-6/INDEX.md](../step-6/INDEX.md)
