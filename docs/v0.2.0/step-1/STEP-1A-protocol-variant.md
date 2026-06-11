# STEP-1A — `DestinationFamily::Rist` variant

> Squash with 1B + 1C.

## Goal

Add the RIST destination variant + serde defaults to
`crates/migration-runtime/src/protocol.rs`.

## The change

Default helpers (near the other `default_*` fns):

```rust
fn default_rist_port() -> u32 {
    5004
}

fn default_rist_sender_buffer_ms() -> u32 {
    1000
}
```

Variant (after `Srt` in `DestinationFamily`):

```rust
    Rist {
        /// Receiver address (RIST is point-to-point over UDP+ARQ).
        address: String,
        #[serde(default = "default_rist_port")]
        port: u32,
        /// Sender-side retransmit buffer in milliseconds.
        #[serde(default = "default_rist_sender_buffer_ms")]
        sender_buffer_ms: u32,
    },
```

> `DestinationFamily` derives `Eq, Hash` — `String`/`u32` are fine (no floats).

## Verification

`cargo check -p migration-runtime` (clean only after 1B + 1C land too).

→ Next: [STEP-1B-from-family-arm.md](STEP-1B-from-family-arm.md)
