# Step 08 — Custom StreamPack endpoint + Rust encoded-frame ingest

**Master plan:** §7.1, §7.2 · **Phase:** 2 · **Depends on:** step-07 green · **Lang:** Kotlin + Rust

## Goal

Stop using StreamPack's own SRT transport and instead route **encoded H.264 access
units** from the StreamPack `MediaCodec` over JNI into Rust, where a new H.264
`FramePair`-style channel feeds GStreamer (steps 09/10). Audio handling decided per
§15 (mux StreamPack AAC, or keep GStreamer mic AAC).

## Files touched

- **New:** `app/.../stream/RustGStreamerEndpoint.kt`
- `src/jni_bridge/main_activity.rs` — `native_write_encoded_frame` (+ pipeline lifecycle symbols)
- `src/jni_bridge/helpers.rs` — `push_encoded_frame` + `H264_FRAME_PAIR`
- `src/lib.rs` — re-export the new JNI symbols

## Code

### `app/.../stream/RustGStreamerEndpoint.kt`

```kotlin
package org.fcast.android.sender.stream

import java.nio.ByteBuffer
// import io.github.thibaultbee.streampack.core.elements.endpoints.IEndpointInternal
// import io.github.thibaultbee.streampack.core.elements.encoders.CodecConfig
// … (resolve exact 3.1.1 endpoint package; the boilerplate only uses built-in SRT/RTMP)

/** Routes StreamPack-encoded frames to Rust → GStreamer instead of StreamPack SRT. */
class RustGStreamerEndpoint /* : IEndpointInternal */ {
    private var nextPid = 256

    fun openPipeline()  = nativeOpenEncodedGStreamerPipeline()
    fun closePipeline() = nativeCloseEncodedGStreamerPipeline()
    fun startPipeline() = nativeStartEncodedGStreamerPipeline()
    fun stopPipeline()  = nativeStopEncodedGStreamerPipeline()

    fun addStream(mimeType: String): Int {
        val pid = nextPid++
        nativeAddEncodedStream(pid, mimeType)
        return pid
    }

    /** Called per encoded access unit. Forward then release the StreamPack buffer. */
    fun write(streamPid: Int, timestampNs: Long, flags: Int, buffer: ByteBuffer) {
        nativeWriteEncodedFrame(streamPid, timestampNs, flags, buffer, buffer.remaining())
    }

    private external fun nativeOpenEncodedGStreamerPipeline()
    private external fun nativeCloseEncodedGStreamerPipeline()
    private external fun nativeStartEncodedGStreamerPipeline()
    private external fun nativeStopEncodedGStreamerPipeline()
    private external fun nativeAddEncodedStream(pid: Int, mimeType: String)
    private external fun nativeWriteEncodedFrame(
        streamPid: Int, timestampNs: Long, flags: Int, buffer: ByteBuffer, size: Int,
    )
}
```

> The `IEndpointInternal`/`Frame` field names (`timestampInUs`, `flags`, `buffer`) are
> **version-sensitive** — read them from the 3.1.1 sources before wiring `write(...)`.

### `src/jni_bridge/main_activity.rs` — encoded-frame ingest (parallels `nativeProcessFrame`)

```rust
#[cfg(target_os = "android")]
pub fn native_write_encoded_frame<'local>(
    env: JNIEnv<'local>,
    _class: JClass<'local>,
    stream_pid: jni::sys::jint,
    timestamp_ns: jni::sys::jlong,
    flags: jni::sys::jint,
    buffer: JByteBuffer<'local>,
    size: jni::sys::jint,
) {
    if let Err(err) = crate::jni_bridge::helpers::push_encoded_frame(
        env, stream_pid, timestamp_ns, flags, buffer, size,
    ) {
        error!(?err, "push_encoded_frame failed");
    }
}
```

### `src/jni_bridge/helpers.rs` — `push_encoded_frame` + `H264_FRAME_PAIR`

`push_encoded_frame` mirrors `process_frame` but copies the raw access unit into a new
H.264 channel (a `Vec<u8>` + PTS, not a `VideoFrame`):

- Keep the existing `FRAME_PAIR` (raw I420) for the **legacy** path untouched.
- Add `H264_FRAME_PAIR` (or an analogous `Arc<…>` channel) carrying `{ bytes, pts_ns,
  is_keyframe }`. Map the Kotlin `flags` to a keyframe bool here.
- The lifecycle symbols (`nativeOpen/Close/Start/StopEncodedGStreamerPipeline`,
  `nativeAddEncodedStream`) drive the new H.264 camera source node (step-09) and the
  mux-only destination (step-10).

Re-export all new `Java_org_fcast_android_sender_…` symbols in `src/lib.rs`.

## How to verify

```
✅ Encoded frames arrive in Rust: log AU size + PTS for the first 10 and every 30th.
✅ Keyframe flag is set on IDR access units (sizes spike periodically).
✅ Direct-SRT path (step-07) still works when the endpoint is NOT selected.
✅ No raw nativeProcessFrame calls while the encoded endpoint is active.
```

## Risks (carried from master §15)

- **Endpoint API surface** is not exercised by the boilerplate — confirm
  `IEndpointInternal`/`Frame` from the 3.1.1 artifact.
- **H.264 bitstream format** (Annex B vs AVCC) is confirmed/handled in **step-09** where
  the GStreamer caps are pinned — log the first AU's first bytes here to decide.
- **Audio:** decide now whether StreamPack also emits AAC (mux both pre-encoded) or
  GStreamer keeps encoding mic audio.
