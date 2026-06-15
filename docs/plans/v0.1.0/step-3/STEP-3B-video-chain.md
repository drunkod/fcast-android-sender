# STEP-3B — Video chain

> Second slice of the `Srt` arm: build the video encode path and link it into
> the muxer. Goes **inside** the arm opened in [STEP-3A](STEP-3A-mux-sink-setup.md),
> after the `srtsink` property block.

---

## Goal

Wire `appsrc(video) → videoconvert → [encoder chain] → h264parse → mpegtsmux`,
reusing the shared encoder-selection helpers so SRT gets the same hardware/
software H.264 fallback as RTMP/UDP.

---

## Pre-flight

| Helper (do not re-create) | Location |
|---|---|
| `select_video_encoder` (Android hw / host sw fallback) | `nodes/destination.rs:434` (Android) / `:525` (host) |
| `add_video_encoder_chain` (adds encoder + optional NV12 capsfilter to pipeline) | `nodes/destination.rs:546` |
| `link_video_encoder_chain` (links upstream → [capsfilter] → encoder → downstream) | `nodes/destination.rs:562` |
| `video_appsrc` local | top of `build_live_pipeline`, in scope |

These are the exact three helpers the UDP branch uses — SRT calls them
identically.

---

## The change — video chain (inside the arm)

**File:** `crates/migration-runtime/src/nodes/destination.rs`

Add directly after the `srtsink` property block from 3A:

```rust
    // ── Video chain ──────────────────────────────────────────────────────
    // appsrc(video) → videoconvert → [NV12 capsfilter] → amcvidenc/x264enc
    //               → h264parse → mpegtsmux
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
            &vconv,
            &venc_chain,
            &vparse,
            "srt video encoder chain",
        )?;

        gst::Element::link_many([&vparse, &mux].as_slice())
            .map_err(|err| format!("Failed to link srt video output: {err:?}"))?;
    }
```

---

## Why the encoder is built via a helper, not inline

`select_video_encoder` returns a `VideoEncoderChain { encoder, capsfilter }`:

- On **Android** it probes `amcvidenc-*` hardware H.264 factories first, falling
  back to `x264enc`/`openh264enc`, and attaches an `NV12` capsfilter so the
  hardware encoder gets the pixel format it expects.
- On **host** it tries `nvh264enc → x264enc → openh264enc`.

`add_video_encoder_chain` / `link_video_encoder_chain` handle the optional
capsfilter transparently, so the SRT arm doesn't need to know whether a
capsfilter exists. This is identical to UDP/RTMP — no SRT-specific encoder
logic.

---

## Skip path

If `video_appsrc` is `None` (video disabled), the whole block is skipped and
the muxer simply has no video pad — audio-only SRT is valid. The
`video_appsrc` local is created earlier in `build_live_pipeline` only when
`self.video_enabled`.

---

## Verification

```bash
cargo check -p migration-runtime
```

The `unused variable: mux` warning from 3A disappears once this block links to
`&mux`. Audio chain → next.

---

## Next

→ [STEP-3C-audio-chain.md](STEP-3C-audio-chain.md)
