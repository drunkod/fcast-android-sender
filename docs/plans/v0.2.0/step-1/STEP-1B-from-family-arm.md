# STEP-1B — `from_family` arm

> Squash with 1A + 1C. **File:** `crates/migration-runtime/src/nodes/destination.rs`.

After the `Srt` arm in `DestinationPipelineProfile::from_family`:

```rust
DestinationFamily::Rist { .. } => {
    elements.extend([
        "mpegtsmux",
        "ristsink",
        "videoconvert",
        "h264enc",
        "h264parse",
        "audioconvert",
        "audioresample",
        "avenc_aac",
    ]);
}
```

Identical to the `Srt`/`Udp` arms with `ristsink` as the network sink. The shared
`retain` audio/video filter then prunes by flag (see v0.1.0 STEP-2B).

→ Next: [STEP-1C-build-live-pipeline.md](STEP-1C-build-live-pipeline.md)
