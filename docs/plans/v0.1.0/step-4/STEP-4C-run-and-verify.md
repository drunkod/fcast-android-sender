# STEP-4C — Run & verify

> No code edit. How to run the six SRT tests and what to expect.

---

## Run just the SRT tests

```bash
# Serde (4A) + profile (4B) tests — host, no GStreamer
cargo test -p migration-runtime -- srt
```

Filtering on `srt` catches both groups:
- `srt_destination_*` (4 serde tests, 4A)
- `srt_profile_*` (2 profile tests, 4B)

## Run the full crate suite (catch regressions)

```bash
cargo test -p migration-runtime
```

---

## Expected output

```
running 6 tests
test tests::srt_destination_defaults_latency_when_omitted ... ok
test tests::srt_destination_with_encryption_roundtrip ... ok
test tests::srt_destination_passphrase_absent_omitted_from_wire ... ok
test tests::srt_destination_ipv6_uri_roundtrip ... ok
test tests::srt_profile_lists_srtsink_and_mpegtsmux ... ok
test tests::srt_profile_audio_disabled_removes_audio_elements ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured
```

(`protocol.rs` and `nodes/destination.rs` live in the same crate, so a single
`-p migration-runtime` run executes both modules' `tests`.)

---

## Dependency matrix

| Test group | Needs | Runs without |
|---|---|---|
| 4A serde tests | STEP-1 variant | STEP-3 pipeline, GStreamer init |
| 4B profile tests | STEP-2 `from_family` arm | STEP-3 pipeline, GStreamer init |

> Neither group constructs a live pipeline, so they pass on a plain `cargo test`
> host run — no `adb`, no device, no GStreamer plugins required. The live-path
> smoke test lives in [../step-3/STEP-3E-smoke-and-pitfalls.md](../step-3/STEP-3E-smoke-and-pitfalls.md).

---

## CI hook (optional)

If the crate is in a workspace CI matrix, these run automatically under the
existing `cargo test --workspace`. No new CI config needed — the tests are
plain `#[test]` functions in already-compiled modules.

---

## Done — STEP-4 complete

| Sub-step | Status |
|---|---|
| 4A protocol serde tests | ✅ |
| 4B profile tests | ✅ |
| 4C run & verify | ✅ |

→ Next top-level step: [../step-5/INDEX.md](../step-5/INDEX.md)
