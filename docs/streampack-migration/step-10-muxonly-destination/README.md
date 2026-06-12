# Step 10 — Mux-only destination path (Rust)

**Master plan:** §7.4 · **Phase:** 2 · **Depends on:** step-09 · **Lang:** Rust

## Goal

Because the camera source is now already H.264 (step-09), the destination must **not**
re-encode. Add a "pre-encoded video" path so `DestinationFamily::Srt` muxes the incoming
H.264 directly. Gate it on the pipeline-mode flag so the legacy raw destinations (which
still run `select_video_encoder`) are untouched.

## Files touched

- `crates/migration-runtime/src/nodes/destination.rs` — pre-encoded video branch

## Current SRT destination (legacy raw — keep)

```
appsrc(raw) → videoconvert → amcvidenc(NV12) → h264parse → mpegtsmux → queue → srtsink
                              └─ select_video_encoder() builds amcvidenc-*/x264enc
```

## New SRT destination (pre-encoded — Phase 2)

```
appsrc(h264) → h264parse → mpegtsmux → queue → srtsink
   (NO videoconvert, NO select_video_encoder)
```

## Implementation notes

- Add a `video_pre_encoded: bool` (or reuse the Rust `AndroidCameraPipeline` mode) on the
  destination build path. When set:
  - **skip** `videoconvert` and `select_video_encoder` / `add_video_encoder_chain` /
    `link_video_encoder_chain`;
  - link `appsrc(h264) → h264parse → mpegtsmux`;
  - keep `config-interval = 1` on `h264parse` so SPS/PPS repeat in-band for late SRT
    joiners (matches the existing raw SRT arm);
  - keep the existing `mpegtsmux alignment=7 → queue → srtsink` tail unchanged
    (`srtsink` uri/latency/passphrase/pbkeylen handling is identical).
- The destination's video `appsrc` caps become `video/x-h264` instead of `video/x-raw`.
- **Audio:** unchanged unless step-08 decided StreamPack also emits AAC. If GStreamer
  keeps encoding mic audio, the existing `audioconvert → audioresample → avenc_aac → mux`
  arm is reused as-is. If StreamPack supplies AAC, add a second pre-encoded `appsrc(aac)
  → aacparse → mux` arm and drop the GStreamer audio encoder.

## How to verify

```
✅ Legacy mode: SRT destination still builds the encoder chain (amcvidenc/x264enc) — no
   behaviour change.
✅ Pre-encoded mode: no encoder element is created; appsrc(h264) → h264parse → mpegtsmux
   → queue → srtsink links cleanly.
✅ End-to-end: StreamPack camera (step-09) → this destination → SRT receiver shows A/V.
✅ Late-join: a receiver connecting mid-stream decodes within one GOP (config-interval=1).
```

## Risks

- Double-encode bug: if the mode flag isn't threaded correctly, a pre-encoded H.264
  appsrc could hit `videoconvert`+encoder → garbage/`not-negotiated`. Assert no encoder
  element exists on the pre-encoded path.
- Timestamp continuity: StreamPack PTS (step-08) must be sane for `mpegtsmux`; watch for
  DTS/PTS reordering warnings from `h264parse`/`mpegtsmux`.
- Keep all other `DestinationFamily` arms (Rtmp/Udp/Rist/LocalFile/LocalPlayback/Whep)
  on their existing encoder path for now — only `Srt` needs the pre-encoded branch for
  Phase 2 validation.
