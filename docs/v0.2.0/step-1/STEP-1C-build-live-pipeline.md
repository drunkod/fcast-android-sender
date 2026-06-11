# STEP-1C — `build_live_pipeline` RIST arm

> Squash with 1A + 1B. **File:** `crates/migration-runtime/src/nodes/destination.rs`.
> Mirror of the SRT arm; `ristsink` takes `address` + `port` (not a `uri`).

Insert after the `Srt` arm, before `LocalFile`:

```rust
DestinationFamily::Rist {
    address,
    port,
    sender_buffer_ms,
} => {
    let mux  = Self::make_element("mpegtsmux", None)?;
    let sink = Self::make_element("ristsink",  None)?;

    pipeline.add(&mux).map_err(|err| {
        format!("Failed to add mpegtsmux to rist pipeline: {err:?}")
    })?;
    pipeline.add(&sink).map_err(|err| {
        format!("Failed to add ristsink to rist pipeline: {err:?}")
    })?;

    // 188-byte MPEG-TS packet alignment (same reason as UDP/SRT).
    if mux.has_property("alignment") {
        mux.set_property("alignment", 7i32);
    }

    sink.set_property("address", address.clone());
    sink.set_property("port", *port);
    if sink.has_property("sender-buffer") {
        sink.set_property("sender-buffer", *sender_buffer_ms);
    }

    // ── Video chain (identical to SRT/UDP) ──
    if let Some(appsrc) = video_appsrc.as_ref() {
        let vconv      = Self::make_element("videoconvert", None)?;
        let venc_chain = Self::select_video_encoder(&self.id)?;
        let vparse     = Self::make_element("h264parse",    None)?;

        pipeline.add(&vconv).map_err(|err| {
            format!("Failed to add videoconvert to rist pipeline: {err:?}")
        })?;
        Self::add_video_encoder_chain(&pipeline, &venc_chain, "rist pipeline")?;
        pipeline.add(&vparse).map_err(|err| {
            format!("Failed to add h264parse to rist pipeline: {err:?}")
        })?;

        gst::Element::link_many([appsrc.upcast_ref::<gst::Element>(), &vconv].as_slice())
            .map_err(|err| format!("Failed to link rist video pre-processing: {err:?}"))?;
        Self::link_video_encoder_chain(&vconv, &venc_chain, &vparse, "rist video encoder chain")?;
        gst::Element::link_many([&vparse, &mux].as_slice())
            .map_err(|err| format!("Failed to link rist video output: {err:?}"))?;
    }

    // ── Audio chain (identical to SRT/UDP) ──
    if let Some(appsrc) = audio_appsrc.as_ref() {
        let aconv     = Self::make_element("audioconvert",  None)?;
        let aresample = Self::make_element("audioresample", None)?;
        let aenc      = Self::make_element("avenc_aac",     None)?;
        pipeline.add(&aconv).map_err(|err| format!("Failed to add audioconvert to rist pipeline: {err:?}"))?;
        pipeline.add(&aresample).map_err(|err| format!("Failed to add audioresample to rist pipeline: {err:?}"))?;
        pipeline.add(&aenc).map_err(|err| format!("Failed to add avenc_aac to rist pipeline: {err:?}"))?;
        gst::Element::link_many(
            [appsrc.upcast_ref::<gst::Element>(), &aconv, &aresample, &aenc, &mux].as_slice(),
        )
        .map_err(|err| format!("Failed to link rist audio chain: {err:?}"))?;
    }

    mux.link(&sink)
        .map_err(|err| format!("Failed to link mpegtsmux to ristsink: {err:?}"))?;
}
```

→ Next: [STEP-1D-android-plugin.md](STEP-1D-android-plugin.md)
