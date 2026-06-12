# Step 09 — H.264 camera source node (Rust)

**Master plan:** §7.3 · **Phase:** 2 · **Depends on:** step-08 · **Lang:** Rust

## Goal

Add a sibling builder to `CameraSourceNode` that ingests **already-encoded H.264** from
`H264_FRAME_PAIR` (step-08) instead of raw I420. The pipeline mode selects which builder
runs. The raw builder is left untouched (legacy fallback). Rotation/mirror now happen in
the StreamPack encoder/SurfaceProcessor, so the raw `videoconvert`/`videoflip`/
`videocrop` chain is **dropped** here.

## Files touched

- `crates/migration-runtime/src/nodes/camera_source.rs` — new builder variant + `wire_need_data` for H.264

## Current raw pipeline (for contrast — do not remove)

```
appsrc video/x-raw,I420 → videoconvert → videocrop → videoflip → aligncrop → appsink
```

## New H.264 pipeline

```
appsrc video/x-h264,byte-stream,au → queue(leaky) → h264parse → appsink
  (feeds the existing DestinationNode mux/sink — step-10)
```

## Code

```rust
// crates/migration-runtime/src/nodes/camera_source.rs  (new builder variant)
let appsrc = AppSrc::builder()
    .name(format!("camera-h264-appsrc-{}", self.id))
    .format(gst::Format::Time)
    .is_live(true)
    .do_timestamp(true)
    .stream_type(gst_app::AppStreamType::Stream)
    .caps(
        &gst::Caps::builder("video/x-h264")
            .field("stream-format", "byte-stream")
            .field("alignment", "au")
            .build(),
    )
    .build();

let queue = gst::ElementFactory::make("queue")
    .name(format!("camera-h264-queue-{}", self.id))
    .property("max-size-buffers", 8u32)
    .property_from_str("leaky", "downstream")
    .build()
    .map_err(|e| format!("queue: {}", e.message))?;

let parse = gst::ElementFactory::make("h264parse")
    .name(format!("camera-h264parse-{}", self.id))
    .build()
    .map_err(|e| format!("h264parse: {}", e.message))?;
if parse.has_property("config-interval") {
    parse.set_property("config-interval", -1i32); // repeat SPS/PPS in-band
}
// appsrc → queue → h264parse → appsink   (feeds the existing DestinationNode mux/sink)
```

`wire_need_data` pulls from `H264_FRAME_PAIR` and pushes each access unit; set buffer
flags for keyframes from the Kotlin `flags` arg (clear `DELTA_UNIT` on IDR).

## ⚠ Confirm the bitstream format before pinning caps

The caps above (`stream-format=byte-stream, alignment=au`) are correct **only if** the
StreamPack endpoint hands us **Annex B** access units (start-code `00 00 00 01`, SPS/PPS
in-band). Android `MediaCodec` can instead emit **AVCC** (length-prefixed NAL units) with
codec config (`csd-0`/`csd-1` = SPS/PPS) delivered separately via
`BUFFER_FLAG_CODEC_CONFIG`. If so, either:

- configure the encoder/endpoint for byte-stream output, **or**
- push `stream-format=avc` + set `codec_data` on the caps from the CSD buffers, and
  forward the config NAL on stream start (don't drop the `CODEC_CONFIG` frame).

Log the first AU bytes in step-08 to decide. The keyframe flag mapping (`flags` →
`gst::BufferFlags::DELTA_UNIT` cleared on IDR) depends on this too.

## How to verify

```
✅ With mode = StreamPackEncodedToGstreamer, the H.264 builder runs; the raw builder
   does not (legacy mode still builds the raw chain).
✅ h264parse negotiates (caps match the actual bitstream format).
✅ appsink receives parsed AUs; PTS is monotonic; keyframes flagged.
✅ No videoconvert/videoflip/videocrop in the H.264 pipeline.
```

## Risks

- Wrong `stream-format` → `not-negotiated` and a dead pipeline. This is the single most
  likely Phase 2 failure — confirm Annex B vs AVCC first.
- Rotation/mirror parity: the raw path baked these into `videoflip`; here they must be
  correct from `setTargetRotation` in step-02. Verify AUTO mode before retiring the raw
  path (step-11).
