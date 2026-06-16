# STEP-5 — Bridge Panel enum + SRT destination state (sub-steps)

> Split of the original STEP-5 into four self-contained sub-steps.
> **Independent step** — can land before or after Steps 1–4. UI-data-model
> change only; adding callbacks without Rust handlers is safe (unhandled Slint
> callbacks are no-ops), and the Rust `Panel` usages are all `==` comparisons,
> never an exhaustive `match`, so new variants don't break the build.

---

## Sub-step map

| # | File | Scope | Net Δ |
|---|------|-------|-------|
| 5A | [STEP-5A-panel-enum.md](STEP-5A-panel-enum.md) | Add `protocol-rtmp-settings` + `protocol-srt-settings` to the `Panel` enum | ~4 lines |
| 5B | [STEP-5B-srt-destination-struct.md](STEP-5B-srt-destination-struct.md) | Add the `SrtDestination` struct | ~6 lines |
| 5C | [STEP-5C-bridge-properties-callbacks.md](STEP-5C-bridge-properties-callbacks.md) | Add Bridge properties + callbacks (`srt-destination`, passphrase, pbkeylen-idx, start/stop/save) | ~15 lines |
| 5D | [STEP-5D-verification-rust-bindings.md](STEP-5D-verification-rust-bindings.md) | Slint compile check + generated Rust binding names | docs only |

Single file edited (5A–5C): `ui/bridge.slint`.

---

## What this step is

The **data contract** between the slintcn SRT UI (STEP-7) and the Rust backend:
a panel route, a state struct, and the read/write properties + action
callbacks. No visible widgets here — those live in STEP-6/7/8.

### Is there slintcn UI in STEP-5?

**No — this is the hand-written data layer.** slintcn ships *presentational
widgets* (Button, Card, Input, Switch, Badge…), not data models. The structs,
enums, and `Bridge` properties defined here are plain Slint that the slintcn
components in STEP-6/7/8 then bind to.

```
STEP-5 (this — plain Slint)        STEP-7 (slintcn widgets)
Panel.protocol-srt-settings   ◄──  rendered when active-panel matches
SrtDestination { uri, … }     ◄──  Input { text <=> …uri }
srt-destination-pbkeylen-idx  ◄──  CyclerRow { changed => set idx }
start/stop/save callbacks     ◄──  Button { clicked => Bridge.save-… }
```

---

## Landing order

```
5A ─► 5B ─► 5C ─► 5D (verify)   — independent of Steps 1–4
```

→ Next top-level step: [../step-6/INDEX.md](../step-6/INDEX.md)
