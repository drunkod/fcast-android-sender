# STEP-2 — SRT pipeline profile arm (sub-steps)

> Split of the original STEP-2 into three self-contained sub-steps.
> **Lands in the same commit** as STEP-1 + STEP-3 — the `Srt` variant from
> STEP-1 makes the `match` in `from_family` non-exhaustive until 2A is present.

---

## Sub-step map

| # | File | Scope | Net Δ |
|---|------|-------|-------|
| 2A | [STEP-2A-from-family-arm.md](STEP-2A-from-family-arm.md) | Add the `Srt` arm (element list) to `DestinationPipelineProfile::from_family` | ~12 lines |
| 2B | [STEP-2B-element-roles-and-filter.md](STEP-2B-element-roles-and-filter.md) | Element roles + the `audio`/`video` `retain` filter semantics | docs only |
| 2C | [STEP-2C-full-function-verification.md](STEP-2C-full-function-verification.md) | Full assembled `from_family` + compile/test verification | docs only |

Single file edited (in 2A): `crates/migration-runtime/src/nodes/destination.rs`.

---

## What this step is (and what it is NOT)

`DestinationPipelineProfile::from_family` produces the **diagnostic element
inventory** returned by the `getinfo` command — a `Vec<String>` of element
names. It is **informational only**; the *real* GStreamer graph is built in
STEP-3 (`build_live_pipeline`). 2A mirrors the existing `Udp` arm, swapping
`udpsink` → `srtsink`.

### Is there any UI in STEP-2?

**No.** This is a backend Rust change. The closest UI reflection is the
**"Pipeline Reference"** rows in the SRT settings page
([../step-7/INDEX.md](../step-7/INDEX.md)), which show
`appsrc → videoconvert → h264enc → h264parse → mpegtsmux → srtsink` as a
read-only debug aid built with slintcn `Card` + `SettingsValueRow`. Those rows
are the human-facing echo of the element list defined here.

```
STEP-2 (this)                         STEP-7 (UI, slintcn)
from_family → ["mpegtsmux","srtsink",  Card { SettingsValueRow {
  "videoconvert","h264enc", …]    ───►   value: "appsrc → … → srtsink" } }
(getinfo element inventory)            (read-only "Pipeline Reference")
```

---

## Landing order

```
2A ─► 2B (understand filter) ─► 2C (verify)
 │
 └─ squash with STEP-1 + STEP-3 (exhaustive match)
```

→ Next top-level step: [../step-3/INDEX.md](../step-3/INDEX.md)
