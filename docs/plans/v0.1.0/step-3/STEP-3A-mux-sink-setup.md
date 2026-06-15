# STEP-3A — Mux + sink setup & `srtsink` properties

> First slice of the `Srt` arm: create the muxer and sink, add them to the
> pipeline, and configure every `srtsink` property. **Requires STEP-1's
> variant** (for the field bindings); squash with STEP-1 + STEP-2.

---

## Goal

Open the `DestinationFamily::Srt` match arm, create `mpegtsmux` + `srtsink`,
add them to the pipeline, and set the muxer alignment + all SRT properties
(`uri`, `latency`, `wait-for-connection`, optional `passphrase`/`pbkeylen`).

---

## Pre-flight

| Template / helper to mirror | Location |
|---|---|
| `Udp` branch in `build_live_pipeline` (closest template) | `crates/migration-runtime/src/nodes/destination.rs:793–865` |
| `make_element` helper | `nodes/destination.rs:240` |
| `video_appsrc` / `audio_appsrc` locals (already in scope) | top of `build_live_pipeline`, `nodes/destination.rs:584–607` |
| `mpegtsmux alignment=7` precedent | UDP branch, `nodes/destination.rs` |

The UDP sink config being mirrored (note: UDP uses `host`+`port`, SRT uses a
full `uri`):

```rust
// UDP (existing):
sink.set_property("host", host.clone());
sink.set_property("port", 5005i32);
```

---

## The change — open the arm + setup

**File:** `crates/migration-runtime/src/nodes/destination.rs`

Insert after the `DestinationFamily::Udp` arm closing `}` (around line 865),
before `DestinationFamily::LocalFile`:

```rust
DestinationFamily::Srt {
    uri,
    latency,
    passphrase,
    pbkeylen,
} => {
    let mux  = Self::make_element("mpegtsmux", None)?;
    let sink = Self::make_element("srtsink",   None)?;

    pipeline.add(&mux).map_err(|err| {
        format!("Failed to add mpegtsmux to srt pipeline: {err:?}")
    })?;
    pipeline.add(&sink).map_err(|err| {
        format!("Failed to add srtsink to srt pipeline: {err:?}")
    })?;

    // ── mpegtsmux ───────────────────────────────────────────────────────
    // alignment=7 aligns output to 188-byte MPEG-TS packet boundaries.
    // Without it, FFmpeg and many hardware receivers report continuity
    // counter errors on every packet.
    if mux.has_property("alignment") {
        mux.set_property("alignment", 7i32);
    }

    // ── srtsink ─────────────────────────────────────────────────────────
    // latency is i32 milliseconds — NOT i64 nanoseconds.
    // Passing i64 triggers a GLib type warning and the property is ignored.
    sink.set_property("uri", uri.clone());
    sink.set_property("latency", *latency);

    // In caller mode (the default), srtsink's `wait-for-connection` is true,
    // so PAUSED → PLAYING blocks for up to `connect-timeout` (~3 s) when no
    // receiver is listening. `DestinationNode::sync_live_pipeline` drives
    // `set_state` synchronously on the runtime tick, so a blocking transition
    // stalls the whole node. Setting it false lets the pipeline reach PLAYING
    // immediately and connect in the background (udpsink never had this issue,
    // which is why the UDP template doesn't set it).
    if sink.has_property("wait-for-connection") {
        sink.set_property("wait-for-connection", false);
    }

    // Encryption: both passphrase AND pbkeylen must be set together.
    // Setting only one leaves the stream unencrypted without any warning.
    if let (Some(pass), Some(keylen)) = (passphrase.as_deref(), pbkeylen) {
        sink.set_property("passphrase", pass);
        sink.set_property("pbkeylen", *keylen);
    }

    // (video chain → STEP-3B, audio chain → STEP-3C, mux.link(sink) → STEP-3D)
}
```

> The arm is **not closed yet** — 3B and 3C add the media chains inside it,
> and 3D adds the final `mux.link(&sink)` and the closing `}`.

---

## Property reference

| Property | Type | Value | Notes |
|---|---|---|---|
| `mpegtsmux` `alignment` | `i32` | `7` | 188-byte TS packet alignment |
| `srtsink` `uri` | `String` | full `srt://…` | includes `?mode=` query if any |
| `srtsink` `latency` | `i32` | ms (default 200) | **not** `i64` |
| `srtsink` `wait-for-connection` | `bool` | `false` | non-blocking PLAYING |
| `srtsink` `passphrase` | `String` | only if both Some | 10–79 ASCII |
| `srtsink` `pbkeylen` | `i32` | only if both Some | 16 / 24 / 32 |

---

## Verification

```bash
cargo check -p migration-runtime
```

Will report `unused variable` warnings for the chain locals until 3B/3C add
them, and the match stays open — that's expected mid-step. Clean compile after
3D + STEP-1 + STEP-2.

---

## Next

→ [STEP-3B-video-chain.md](STEP-3B-video-chain.md)
