# STEP-3C — Audio chain

> Third slice of the `Srt` arm: build the AAC audio path and link it into the
> muxer. Goes **inside** the arm, after the video chain from
> [STEP-3B](STEP-3B-video-chain.md).

---

## Goal

Wire `appsrc(audio) → audioconvert → audioresample → avenc_aac → mpegtsmux`.
Identical to the UDP branch's audio chain.

---

## Pre-flight

| Fact | Detail |
|---|---|
| `audio_appsrc` local | created earlier in `build_live_pipeline` when `self.audio_enabled` **and** family is not RTMP (`nodes/destination.rs:597`) |
| SRT uses external audio | unlike RTMP (which injects a silence/mic source), SRT mixes the connected audio slot via `audio_appsrc` — same as UDP |
| `avenc_aac` | the AAC encoder factory used by UDP/LocalFile |

> **Note on RTMP divergence:** the RTMP branch deliberately ignores
> `audio_appsrc` and builds its own embedded `openslessrc`/silence source.
> SRT does **not** do that — it behaves like UDP and encodes the real audio
> slot. So `audio_appsrc` is `Some(...)` for SRT whenever `audio_enabled`.

---

## The change — audio chain (inside the arm)

**File:** `crates/migration-runtime/src/nodes/destination.rs`

Add directly after the video chain block from 3B:

```rust
    // ── Audio chain ──────────────────────────────────────────────────────
    // appsrc(audio) → audioconvert → audioresample → avenc_aac → mpegtsmux
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
            [
                appsrc.upcast_ref::<gst::Element>(),
                &aconv,
                &aresample,
                &aenc,
                &mux,
            ]
            .as_slice(),
        )
        .map_err(|err| format!("Failed to link srt audio chain: {err:?}"))?;
    }
```

---

## Skip path

If `audio_appsrc` is `None` (audio disabled), this block is skipped and the
muxer has no audio pad — video-only SRT is valid (and the common case for the
STEP-3E smoke test, which uses `audio:false`).

---

## Pad request ordering

`mpegtsmux` uses **request pads** (`sink_%d`). Both the video chain (3B) and
this audio chain link into `&mux` via `link_many`, which requests a fresh sink
pad per call. Order doesn't matter for `mpegtsmux` — it negotiates PIDs
internally — so video-first / audio-first both work.

---

## Verification

```bash
cargo check -p migration-runtime
```

Still reports the open-match error until 3D adds `mux.link(&sink)` and closes
the arm.

---

## Next

→ [STEP-3D-assemble-and-link.md](STEP-3D-assemble-and-link.md)
