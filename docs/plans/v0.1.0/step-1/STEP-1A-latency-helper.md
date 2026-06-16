# STEP-1A — `default_srt_latency()` serde helper

> Smallest piece of STEP-1. A serde `default` provider so `latency` can be
> omitted from the wire and fall back to a sane value.

---

## Goal

Add a free function `default_srt_latency()` that returns the default SRT
end-to-end latency (200 ms). It is referenced by the
`#[serde(default = "default_srt_latency")]` attribute on the `latency` field
added in [STEP-1B](STEP-1B-enum-variant.md).

---

## Pre-flight

| Exists (do not re-create) | Location |
|---|---|
| Existing `default_*` helpers (`default_as_true`, `default_capture_*`, `default_camera_*`, `default_true`) | `crates/migration-runtime/src/protocol.rs:5–43` |

For reference, the existing helpers look like this (top of the file):

```rust
fn default_as_true() -> bool {
    true
}

fn default_capture_width() -> u32 {
    1280
}

// … default_capture_height / default_capture_fps / default_camera_* …

fn default_true() -> bool {
    true
}
```

---

## The change

**File:** `crates/migration-runtime/src/protocol.rs`

Add alongside the other `default_*` helpers (e.g. right after `default_true`
around line 43):

```rust
/// Default SRT end-to-end latency in milliseconds.
///
/// 200 ms matches the `gst-launch` `srtsink` default and is a safe starting
/// point for contribution feeds. Both SRT endpoints must agree within ±50 %;
/// SRT silently upgrades both sides to the larger configured value.
fn default_srt_latency() -> i32 {
    200
}
```

### Why a function and not a literal?

serde's `#[serde(default = "...")]` takes a **path to a function**, not a
literal. The other optional numeric fields in this file (`default_capture_width`,
`default_camera_fps`, …) follow the same pattern — matching it keeps the file
consistent.

---

## Verification

This helper is unused until [STEP-1B](STEP-1B-enum-variant.md) references it.
On its own it triggers a dead-code warning:

```bash
cargo check -p migration-runtime
# warning: function `default_srt_latency` is never used
```

That warning disappears once 1B wires it into the `latency` field. Land 1A
and 1B together.

---

## Next

→ [STEP-1B-enum-variant.md](STEP-1B-enum-variant.md)
