# STEP-1B — Add the `Srt` enum variant

> The core of STEP-1: extend `DestinationFamily` with an SRT variant.
> **Requires [STEP-1A](STEP-1A-latency-helper.md)** (the `default_srt_latency`
> helper) and makes the `match` in `destination.rs` non-exhaustive until
> STEP-2 + STEP-3 land — squash all of them.

---

## Goal

Add `Srt { uri, latency, passphrase, pbkeylen }` to `DestinationFamily` so the
migration-runtime wire protocol accepts SRT destination commands.

---

## Pre-flight

| Exists (do not re-create) | Location |
|---|---|
| `DestinationFamily` enum (derives `Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize`) | `crates/migration-runtime/src/protocol.rs:190` |
| `Whep` variant (insert the new arm after this) | `protocol.rs:203–207` |
| `default_srt_latency` helper | added in [STEP-1A](STEP-1A-latency-helper.md) |

Current enum (for orientation):

```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DestinationFamily {
    Rtmp { uri: String },
    Udp { host: String },
    LocalFile { base_name: String, max_size_time: Option<u32> },
    LocalPlayback,
    Whep {
        #[serde(default)]
        server_port: u16,
    },
}
```

---

## The change

**File:** `crates/migration-runtime/src/protocol.rs`

Insert the `Srt` arm after `Whep`, before the closing `}`:

```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DestinationFamily {
    Rtmp {
        uri: String,
    },
    Udp {
        host: String,
    },
    LocalFile {
        base_name: String,
        max_size_time: Option<u32>,
    },
    LocalPlayback,
    Whep {
        #[serde(default)]
        server_port: u16,
    },
    // ── NEW (STEP-1B) ─────────────────────────────────────────────────────
    Srt {
        /// Full SRT URI, e.g. `srt://host:port` (caller) or
        /// `srt://0.0.0.0:port?mode=listener` (inbound). IPv6 needs brackets:
        /// `srt://[fe80::1]:1234`.
        uri: String,
        /// SRT end-to-end latency in milliseconds. Default 200 ms (STEP-1A).
        /// Both endpoints must agree within ±50 %; SRT upgrades both to the
        /// larger value.
        #[serde(default = "default_srt_latency")]
        latency: i32,
        /// AES passphrase (10–79 ASCII chars). Must be set together with
        /// `pbkeylen`; omitting either disables encryption silently.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        passphrase: Option<String>,
        /// AES key length in bytes: 16 (AES-128), 24 (AES-192), 32 (AES-256).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pbkeylen: Option<i32>,
    },
}
```

---

## Field semantics

| Field | Type | serde behaviour | Maps to (STEP-3) |
|---|---|---|---|
| `uri` | `String` | required | `srtsink` `"uri"` property |
| `latency` | `i32` | defaults to `200` when omitted | `srtsink` `"latency"` (ms, **i32 not i64**) |
| `passphrase` | `Option<String>` | absent → omitted from wire | `srtsink` `"passphrase"` (only if both Some) |
| `pbkeylen` | `Option<i32>` | absent → omitted from wire | `srtsink` `"pbkeylen"` (only if both Some) |

The `Option` + `skip_serializing_if` pairing means a plain (unencrypted) SRT
destination serialises to just `{"Srt":{"uri":…,"latency":…}}` — no nulls.

---

## Why these exact types (forward-looking)

- `latency: i32` — matches the `srtsink` GObject property type. A common bug is
  passing `i64` (treating it as nanoseconds); the GLib type check rejects it.
- `passphrase`/`pbkeylen` are `Option` because encryption is opt-in, and the
  pair must be set together (see [STEP-1D](STEP-1D-verification-pitfalls.md) P1).
- No floats anywhere — important for the `Eq, Hash` derive (see 1D).

---

## Verification

```bash
# Compiles only once STEP-2 + STEP-3 add their match arms.
cargo check -p migration-runtime
```

If you land 1B alone (without 2+3), expect:

```
error[E0004]: non-exhaustive patterns: `&DestinationFamily::Srt { .. }` not covered
  --> crates/migration-runtime/src/nodes/destination.rs
```

That is expected — it is why 1A+1B+2+3 squash into one commit.

---

## Next

→ [STEP-1C-serde-wire-contract.md](STEP-1C-serde-wire-contract.md)
