# STEP-1D — Verification, derive checks & pitfalls

> Final sub-step of STEP-1. No code edit — confirms the variant compiles,
> the derives hold, and catches the SRT-specific traps before STEP-3 wires
> the real pipeline.

---

## Derive-compatibility check (`Eq` + `Hash`)

`DestinationFamily` derives `Eq` and `Hash`:

```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
```

Every field of the new `Srt` variant must therefore implement `Eq` **and**
`Hash`:

| Field | Type | `Eq`? | `Hash`? |
|---|---|---|---|
| `uri` | `String` | ✅ | ✅ |
| `latency` | `i32` | ✅ | ✅ |
| `passphrase` | `Option<String>` | ✅ | ✅ |
| `pbkeylen` | `Option<i32>` | ✅ | ✅ |

✅ All clear. **Do not** introduce an `f32`/`f64` field (e.g. a fractional
bitrate multiplier) into this enum — floats are not `Eq`/`Hash` and would break
the derive for the whole type. If a float is ever needed, it belongs in a
separate non-`Hash` config struct, not in `DestinationFamily`.

---

## Exhaustiveness check

Adding the variant makes any `match family { … }` without an `Srt` arm fail to
compile. There are two such matches, both fixed in later steps:

| Match site | Fixed by |
|---|---|
| `DestinationPipelineProfile::from_family` | STEP-2 |
| `DestinationNode::build_live_pipeline` | STEP-3 |

This is why **STEP-1A + 1B + STEP-2 + STEP-3 squash into one commit**.

---

## Compile verification

```bash
cargo check -p migration-runtime
```

Expected: clean once STEP-2 + STEP-3 are applied. STEP-1B alone yields the
`non-exhaustive patterns` error (intended).

---

## Serde unit tests (host, no GStreamer)

These live in STEP-4, but here are the four that exercise STEP-1's surface:

```bash
cargo test -p migration-runtime -- srt_destination
```

- `srt_destination_defaults_latency_when_omitted` — confirms `latency` → 200.
- `srt_destination_with_encryption_roundtrip` — full struct round-trips.
- `srt_destination_passphrase_absent_omitted_from_wire` — `None` omits keys.
- `srt_destination_ipv6_uri_roundtrip` — bracketed IPv6 survives.

---

## Pitfalls (defined here, enforced in STEP-3)

| # | Risk | Where it bites | Mitigation |
|---|---|---|---|
| P1 | `passphrase` set without `pbkeylen` (or vice-versa) | `srtsink` silently leaves the stream unencrypted; peer rejects | STEP-3 only sets both when **both** are `Some` (`if let (Some, Some)`) |
| P2 | `latency` treated as `i64` nanoseconds | GLib type warning, property ignored | Field is `i32` milliseconds by design (STEP-1B) |
| P3 | Passphrase < 10 or > 79 chars | SRT handshake fails silently | Validate in the UI/handler; document in STEP-7 placeholder ("10–79 characters") |
| P4 | `f32`/`f64` field added later | breaks `Eq`/`Hash` derive for the whole enum | keep floats out of `DestinationFamily` (see above) |

---

## Done — STEP-1 complete

| Sub-step | Status |
|---|---|
| 1A latency helper | ✅ |
| 1B enum variant | ✅ |
| 1C serde wire contract | ✅ |
| 1D verification & pitfalls | ✅ |

→ Next top-level step: [../step-2/INDEX.md](../step-2/INDEX.md)
