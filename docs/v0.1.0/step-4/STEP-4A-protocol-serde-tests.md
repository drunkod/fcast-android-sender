# STEP-4A — `protocol.rs` serde tests

> Four host-runnable tests for the `Srt` variant's wire shape. Require
> [STEP-1](../step-1/INDEX.md) only.

---

## Goal

Assert the four serde invariants from
[STEP-1C](../step-1/STEP-1C-serde-wire-contract.md):
default latency, full encryption round-trip, `None` field omission, and
bracketed-IPv6 round-trip.

---

## Pre-flight

| Insert location | Detail |
|---|---|
| `protocol.rs` `#[cfg(test)] mod tests { … }` | after `whep_destination_info_bound_ports_skipped_when_none` (around line 371) |
| Imports in scope | `super::*` already brings `Command`, `DestinationFamily`, serde |

---

## The change

**File:** `crates/migration-runtime/src/protocol.rs`

```rust
// ── SRT ──────────────────────────────────────────────────────────────────────

#[test]
fn srt_destination_defaults_latency_when_omitted() {
    let cmd: Command = serde_json::from_str(
        r#"{"createdestination":{"id":"s1","family":{"Srt":{"uri":"srt://10.0.0.1:9000"}}}}"#,
    )
    .unwrap();
    match cmd {
        Command::CreateDestination {
            family: DestinationFamily::Srt { latency, passphrase, pbkeylen, .. },
            ..
        } => {
            assert_eq!(latency, 200, "default latency must be 200 ms");
            assert!(passphrase.is_none(), "passphrase should be absent");
            assert!(pbkeylen.is_none(),   "pbkeylen should be absent");
        }
        other => panic!("expected Srt destination, got {other:?}"),
    }
}

#[test]
fn srt_destination_with_encryption_roundtrip() {
    let original = DestinationFamily::Srt {
        uri:        "srt://10.0.0.1:9000".to_string(),
        latency:    500,
        passphrase: Some("supersecretphrase1".to_string()),
        pbkeylen:   Some(32),
    };
    let json    = serde_json::to_string(&original).unwrap();
    let decoded: DestinationFamily = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn srt_destination_passphrase_absent_omitted_from_wire() {
    let family = DestinationFamily::Srt {
        uri:        "srt://10.0.0.1:9000".to_string(),
        latency:    200,
        passphrase: None,
        pbkeylen:   None,
    };
    let json = serde_json::to_string(&family).unwrap();
    assert!(!json.contains("passphrase"), "passphrase key must be absent");
    assert!(!json.contains("pbkeylen"),   "pbkeylen key must be absent");
}

#[test]
fn srt_destination_ipv6_uri_roundtrip() {
    // IPv6 in SRT URIs must use brackets: srt://[fe80::1]:1234
    let family = DestinationFamily::Srt {
        uri:        "srt://[fe80::1]:1234".to_string(),
        latency:    200,
        passphrase: None,
        pbkeylen:   None,
    };
    let encoded = serde_json::to_string(&family).unwrap();
    let decoded: DestinationFamily = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, family);
}
```

---

## What each test pins

| Test | Invariant (from STEP-1C) |
|---|---|
| `…_defaults_latency_when_omitted` | `#[serde(default = "default_srt_latency")]` → 200 |
| `…_with_encryption_roundtrip` | full struct survives serialize → deserialize |
| `…_passphrase_absent_omitted_from_wire` | `skip_serializing_if = "Option::is_none"` emits no `null` |
| `…_ipv6_uri_roundtrip` | bracketed IPv6 URI is preserved verbatim |

---

## Verification

```bash
cargo test -p migration-runtime -- srt_destination
```

All four green (requires STEP-1's variant; does **not** require STEP-3).

---

## Next

→ [STEP-4B-profile-tests.md](STEP-4B-profile-tests.md)
