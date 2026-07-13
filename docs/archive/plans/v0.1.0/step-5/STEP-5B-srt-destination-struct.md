# STEP-5B — `SrtDestination` struct

> Add the state struct the SRT settings page (STEP-7) reads/writes and Rust
> updates with live status.

---

## Goal

Define `SrtDestination` — the per-destination state surfaced to the UI: the
URI, latency, current `MixerState`, and last error.

---

## Pre-flight

| Exists (reference pattern) | Location |
|---|---|
| `RtmpDestination` struct (mirror this shape) | `ui/bridge.slint:214–221` |
| `SrtSource` struct (existing SRT-adjacent struct) | `ui/bridge.slint:201–212` |
| `MixerState` enum (used as a field type) | `ui/bridge.slint:34–40` |

`RtmpDestination` for reference:

```slint
export struct RtmpDestination {
    node-id:    string,
    enabled:    bool,
    uri:        string,
    stream-key: string,
    state:      MixerState,
    last-error: string,
}
```

---

## The change

**File:** `ui/bridge.slint`

Add after the `RtmpDestination` struct (around line 221), before `MixerCanvas`:

```slint
// ── SRT destination (v0.1.0, STEP-5B) ────────────────────────────────────────
export struct SrtDestination {
    uri:        string,
    latency-ms: int,
    state:      MixerState,
    last-error: string,
}
```

---

## Field rationale

| Field | Type | Purpose |
|---|---|---|
| `uri` | `string` | full `srt://…` target (the only required SRT field) |
| `latency-ms` | `int` | resolved millisecond value from STEP-7's latency cycler |
| `state` | `MixerState` | drives the Live/Connecting badge + Stop button (STEP-7) |
| `last-error` | `string` | surfaced in the status section when non-empty |

> **Why not a `passphrase` field here?** Encryption secrets are kept out of the
> struct on purpose — they live in a separate scalar property (STEP-5C) so the
> struct can be logged/serialised without leaking the passphrase. The encryption
> *strength* is an index (`pbkeylen-idx`), also a separate property.

`MixerState` is reused (not a new SRT-specific enum) so the badge/stop-button
logic matches RTMP exactly — `MixerState.idle` / `.running` etc.

---

## Verification

```bash
slint-lsp ui/main.slint 2>&1 | grep -c error
# → 0
```

The struct is referenced by the property added in STEP-5C; on its own it just
defines a type (no error).

---

## Next

→ [STEP-5C-bridge-properties-callbacks.md](STEP-5C-bridge-properties-callbacks.md)
