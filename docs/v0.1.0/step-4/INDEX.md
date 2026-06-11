# STEP-4 — Unit tests for SRT (sub-steps)

> Split of the original STEP-4 into three self-contained sub-steps, one per
> test site plus a run/verify step. **Independent of the STEP-1–3 squash
> ordering** — tests can land in a follow-up commit; they only need the `Srt`
> variant (STEP-1) and the profile arm (STEP-2) to compile.

---

## Sub-step map

| # | File | Scope | Net Δ |
|---|------|-------|-------|
| 4A | [STEP-4A-protocol-serde-tests.md](STEP-4A-protocol-serde-tests.md) | 4 serde tests in `protocol.rs` (defaults / encryption / omit / IPv6) | ~60 lines |
| 4B | [STEP-4B-profile-tests.md](STEP-4B-profile-tests.md) | 2 profile tests in `nodes/destination.rs` (element list / audio-disabled) | ~35 lines |
| 4C | [STEP-4C-run-and-verify.md](STEP-4C-run-and-verify.md) | How to run them + expected output | docs only |

Files edited: `crates/migration-runtime/src/protocol.rs` (4A),
`crates/migration-runtime/src/nodes/destination.rs` (4B).

---

## What this step covers

All six tests are **host-runnable with no GStreamer init** — they exercise the
serde wire shape (STEP-1) and the static element inventory (STEP-2), neither of
which instantiates real GStreamer elements. They are the cheap regression net
for the SRT wire contract.

### Is there any UI in STEP-4?

**No.** These are pure Rust unit tests for the migration-runtime crate. There
is no Slint involved.

> For UI-side testing of the SRT page (STEP-7), the project uses the snapshot
> harness at `tests/ui_snapshots.rs` — out of scope for this step, which only
> covers the wire-protocol logic. If you want SRT-page snapshot coverage, that
> would be a separate test step against the rendered slintcn page.

---

## Landing order

```
(STEP-1 + STEP-2 applied) ─► 4A ─► 4B ─► 4C (run)
```

→ Next top-level step: [../step-5/INDEX.md](../step-5/INDEX.md)
