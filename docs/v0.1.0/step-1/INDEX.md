# STEP-1 — `DestinationFamily::Srt` variant (sub-steps)

> Split of the original STEP-1 into four self-contained sub-steps.
> **All four land in the same commit** (together with STEP-2 + STEP-3) —
> the variant alone makes the `match` in `destination.rs` non-exhaustive.

---

## Sub-step map

| # | File | Scope | Net Δ |
|---|------|-------|-------|
| 1A | [STEP-1A-latency-helper.md](STEP-1A-latency-helper.md) | Add the `default_srt_latency()` serde helper | ~3 lines |
| 1B | [STEP-1B-enum-variant.md](STEP-1B-enum-variant.md) | Add the `Srt { uri, latency, passphrase, pbkeylen }` variant + field semantics | ~16 lines |
| 1C | [STEP-1C-serde-wire-contract.md](STEP-1C-serde-wire-contract.md) | Serde representation + JSON wire examples (plain / encrypted / listener) | docs only |
| 1D | [STEP-1D-verification-pitfalls.md](STEP-1D-verification-pitfalls.md) | `Eq`/`Hash`/exhaustiveness checks, compile/test verification, pitfalls | docs only |

Single file edited across 1A + 1B: `crates/migration-runtime/src/protocol.rs`.

---

## Is there any UI in STEP-1?

**No.** STEP-1 is a pure wire-protocol change to the migration-runtime Rust
crate — it adds an enum variant and a serde default. There is no Slint surface
to build here.

The **SRT settings UI** (slintcn `Input` + `Switch` + `Badge` + `Card` +
the dependency-free `CyclerRow`) is **STEP-7**
([../step-7/INDEX.md](../step-7/INDEX.md)). That page
is what ultimately drives the JSON command whose shape STEP-1 defines — see
the encryption-index → `pbkeylen` mapping table in STEP-7, which is the
contract between this variant and the UI.

```
STEP-1 (this)          STEP-5                STEP-7 (UI, slintcn)
DestinationFamily::Srt  Bridge.srt-destination  Input/Switch/Badge/Card/CyclerRow
  { uri, latency,   ◄── + srt-destination-   ◄── writes draft-uri, latency-ms,
    passphrase,           pbkeylen-idx            pbkeylen-idx, passphrase
    pbkeylen }
```

---

## Landing order

```
1A ─► 1B ─► 1C (verify shape) ─► 1D (verify build)
            │
            └─ must squash with STEP-2 + STEP-3 (exhaustive match)
```

→ Next top-level step after this folder: [../step-2/INDEX.md](../step-2/INDEX.md)
