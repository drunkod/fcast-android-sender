# STEP-3D — Link mux→sink, full assembled arm

> Final code slice: link the muxer to the sink, close the arm, and confirm the
> whole thing assembles. This is the line that completes the squash commit
> (STEP-1 + STEP-2 + 3A–3D) and makes the `match` exhaustive again.

---

## Goal

Add `mux.link(&sink)` and the closing `}`, then verify the complete `Srt` arm
reads correctly end-to-end.

---

## The change — final link

**File:** `crates/migration-runtime/src/nodes/destination.rs`

Add after the audio chain block from [STEP-3C](STEP-3C-audio-chain.md), then
close the arm:

```rust
    // ── Mux → sink ───────────────────────────────────────────────────────
    mux.link(&sink)
        .map_err(|err| format!("Failed to link mpegtsmux to srtsink: {err:?}"))?;
}
```

`mpegtsmux`'s `src` pad is static, and `srtsink` has a static `sink` pad, so a
plain `mux.link(&sink)` (no pad names) is correct — same as UDP's
`mux.link(&sink)`.

---

## Full assembled `Srt` arm (3A + 3B + 3C + 3D)

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

    // mpegtsmux: 188-byte TS alignment (FFmpeg/HW receivers need it).
    if mux.has_property("alignment") {
        mux.set_property("alignment", 7i32);
    }

    // srtsink: latency is i32 ms (not i64 ns).
    sink.set_property("uri", uri.clone());
    sink.set_property("latency", *latency);

    // Non-blocking PLAYING in caller mode (see 3A rationale).
    if sink.has_property("wait-for-connection") {
        sink.set_property("wait-for-connection", false);
    }

    // Encryption only when BOTH passphrase + pbkeylen are present.
    if let (Some(pass), Some(keylen)) = (passphrase.as_deref(), pbkeylen) {
        sink.set_property("passphrase", pass);
        sink.set_property("pbkeylen", *keylen);
    }

    // Video chain (STEP-3B).
    if let Some(appsrc) = video_appsrc.as_ref() {
        let vconv      = Self::make_element("videoconvert", None)?;
        let venc_chain = Self::select_video_encoder(&self.id)?;
        let vparse     = Self::make_element("h264parse",    None)?;

        pipeline.add(&vconv).map_err(|err| {
            format!("Failed to add videoconvert to srt pipeline: {err:?}")
        })?;
        Self::add_video_encoder_chain(&pipeline, &venc_chain, "srt pipeline")?;
        pipeline.add(&vparse).map_err(|err| {
            format!("Failed to add h264parse to srt pipeline: {err:?}")
        })?;

        gst::Element::link_many(
            [appsrc.upcast_ref::<gst::Element>(), &vconv].as_slice(),
        )
        .map_err(|err| {
            format!("Failed to link srt video pre-processing: {err:?}")
        })?;

        Self::link_video_encoder_chain(
            &vconv, &venc_chain, &vparse, "srt video encoder chain",
        )?;

        gst::Element::link_many([&vparse, &mux].as_slice())
            .map_err(|err| format!("Failed to link srt video output: {err:?}"))?;
    }

    // Audio chain (STEP-3C).
    if let Some(appsrc) = audio_appsrc.as_ref() {
        let aconv     = Self::make_element("audioconvert",  None)?;
        let aresample = Self::make_element("audioresample", None)?;
        let aenc      = Self::make_element("avenc_aac",     None)?;

        pipeline.add(&aconv).map_err(|err| {
            format!("Failed to add audioconvert to srt pipeline: {err:?}")
        })?;
        pipeline.add(&aresample).map_err(|err| {
            format!("Failed to add audioresample to srt pipeline: {err:?}")
        })?;
        pipeline.add(&aenc).map_err(|err| {
            format!("Failed to add avenc_aac to srt pipeline: {err:?}")
        })?;

        gst::Element::link_many(
            [appsrc.upcast_ref::<gst::Element>(), &aconv, &aresample, &aenc, &mux].as_slice(),
        )
        .map_err(|err| format!("Failed to link srt audio chain: {err:?}"))?;
    }

    // Mux → sink (STEP-3D).
    mux.link(&sink)
        .map_err(|err| format!("Failed to link mpegtsmux to srtsink: {err:?}"))?;
}
```

---

## Pipeline diagram

```
appsrc(video) ──► videoconvert ──► [NV12 capsfilter] ──► amcvidenc-* / x264enc
                                                                │
                                                           h264parse
                                                                │
                                                          mpegtsmux ──► srtsink ──► network
                                                                │
appsrc(audio) ──► audioconvert ──► audioresample ──► avenc_aac ┘
```

`[NV12 capsfilter]` is only present on Android (added by `select_video_encoder`
when an AMC hardware encoder is chosen).

---

## SRT vs UDP arm — difference table

| Property | UDP arm | SRT arm |
|---|---|---|
| Muxer | `mpegtsmux` | `mpegtsmux` (identical) |
| Sink element | `udpsink` | `srtsink` |
| Sink location property | `"host"` + `"port"` | `"uri"` (full `srt://…`) |
| Latency property | — | `"latency"` (`i32` ms) |
| Encryption properties | — | `"passphrase"` + `"pbkeylen"` |
| `wait-for-connection` | — | set `false` (non-blocking PLAYING) |
| `alignment=7` | ✅ | ✅ (same reason) |
| Video chain | identical | identical |
| Audio chain | identical | identical |

---

## Verification

```bash
cargo check -p migration-runtime
```

**Now clean** — the match is exhaustive (STEP-1 variant + STEP-2 profile arm +
this arm). Smoke test + pitfalls → [STEP-3E](STEP-3E-smoke-and-pitfalls.md).

---

## Next

→ [STEP-3E-smoke-and-pitfalls.md](STEP-3E-smoke-and-pitfalls.md)
