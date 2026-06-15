# STEP-5C — Bridge properties + callbacks

> Add the read/write state and action callbacks to the `Bridge` global. This
> is the actual surface STEP-7's slintcn widgets bind to.

---

## Goal

Add to the `Bridge` global: the `srt-destination` state property, the separate
passphrase + encryption-index scalars, and the three action callbacks.

---

## Pre-flight

| Exists (reference pattern) | Location |
|---|---|
| `cam-rtmp-*` properties + callbacks (mirror this) | `ui/bridge.slint:410–424` |
| `Bridge` global block | `ui/bridge.slint:232` onward |
| `SrtDestination` struct | added in [STEP-5B](STEP-5B-srt-destination-struct.md) |

The `cam-rtmp-*` block being mirrored:

```slint
in-out property <string> cam-rtmp-url:             "";
in-out property <string> cam-rtmp-stream-key:      "";
in property      <MixerState> cam-rtmp-state:      MixerState.idle;
in property      <string>     cam-rtmp-error-text: "";
callback start-camera-rtmp-stream();
callback stop-camera-rtmp-stream();
```

---

## The change

**File:** `ui/bridge.slint`

Add inside the `Bridge` global, after the `cam-rtmp-*` block (around line 424):

```slint
// ── SRT destination (v0.1.0, STEP-5C) ────────────────────────────────────────
in-out property <SrtDestination> srt-destination: {
    uri:        "",
    latency-ms: 200,
    state:      MixerState.idle,
    last-error: "",
};
// Passphrase kept separate — never serialised into the main struct so the
// struct can be logged without leaking the secret.
in-out property <string> srt-destination-passphrase: "";
// 0 = None · 1 = AES-128 · 2 = AES-192 · 3 = AES-256
in-out property <int>    srt-destination-pbkeylen-idx: 0;

callback start-srt-destination();
callback stop-srt-destination();
callback save-srt-destination-config();
```

---

## Property direction & ownership

| Property | Dir | Written by | Read by |
|---|---|---|---|
| `srt-destination` (struct) | `in-out` | STEP-7 page (uri, latency-ms) + Rust (state, last-error) | STEP-7 status section |
| `srt-destination-passphrase` | `in-out` | STEP-7 passphrase `Input` | Rust `start-srt-destination` handler |
| `srt-destination-pbkeylen-idx` | `in-out` | STEP-7 encryption `CyclerRow` | Rust → maps to `pbkeylen` {None,16,24,32} |

`in-out` (not `in`) because both Slint (user edits) and Rust (status writeback)
mutate `srt-destination` — same as `cam-rtmp-*` precedent.

---

## Callback contract (Rust side, reference)

These are wired in the migration backend (e.g. `src/backend/migration_backend.rs`).
Unhandled callbacks are safe no-ops, so STEP-5 compiles even before the handlers
exist — but here is the intended mapping:

```rust
// pbkeylen-idx → pbkeylen bytes (the STEP-1C / STEP-7 contract)
let pbkeylen = match bridge.get_srt_destination_pbkeylen_idx() {
    1 => Some(16),  // AES-128
    2 => Some(24),  // AES-192
    3 => Some(32),  // AES-256
    _ => None,      // 0 = None → omit passphrase + pbkeylen
};
let dest       = bridge.get_srt_destination();        // SrtDestination
let passphrase = bridge.get_srt_destination_passphrase().to_string();
// → build {"createdestination":{"family":{"Srt":{ uri, latency, … }}}}
//   (see STEP-1C for the exact JSON shape)
```

| Callback | Rust action |
|---|---|
| `save-srt-destination-config` | persist `uri` + `latency-ms` + `pbkeylen-idx` to config store |
| `start-srt-destination` | build & send the `CreateDestination` graph command (STEP-1C) |
| `stop-srt-destination` | send `Remove`/stop for the SRT destination node |

---

## Verification

```bash
slint-lsp ui/main.slint 2>&1 | grep -c error
# → 0
```

---

## Next

→ [STEP-5D-verification-rust-bindings.md](STEP-5D-verification-rust-bindings.md)
