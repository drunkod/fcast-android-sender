# STEP-2C — Full `from_family` + verification

> Final sub-step of STEP-2. No new edit — shows the complete function with the
> SRT arm in place and the verification commands.

---

## Complete `from_family` (all arms together)

```rust
impl DestinationPipelineProfile {
    fn from_family(family: &DestinationFamily, audio: bool, video: bool) -> Self {
        let mut elements = Vec::new();

        match family {
            DestinationFamily::Rtmp { .. } => {
                elements.extend([
                    "flvmux", "queue", "rtmp2sink",
                    "videoconvert", "timecodestamper", "timeoverlay",
                    "h264enc", "h264parse",
                    "audioconvert", "audioresample", "avenc_aac",
                ]);
            }
            DestinationFamily::Udp { .. } => {
                elements.extend([
                    "mpegtsmux", "udpsink",
                    "videoconvert", "h264enc", "h264parse",
                    "audioconvert", "audioresample", "avenc_aac",
                ]);
            }
            DestinationFamily::LocalFile { .. } => {
                elements.extend([
                    "splitmuxsink", "multiqueue",
                    "videoconvert", "h264enc", "h264parse",
                    "audioconvert", "audioresample", "avenc_aac",
                ]);
            }
            DestinationFamily::LocalPlayback => {
                elements.extend([
                    "autovideosink", "autoaudiosink",
                    "videoconvert", "audioconvert", "audioresample", "queue",
                ]);
            }
            DestinationFamily::Whep { .. } => {
                elements.extend(["videoconvert", "basewebrtcsink"]);
                let _ = audio;
            }
            // ── NEW (STEP-2A) ──────────────────────────────────────────
            DestinationFamily::Srt { .. } => {
                elements.extend([
                    "mpegtsmux",
                    "srtsink",
                    "videoconvert",
                    "h264enc",
                    "h264parse",
                    "audioconvert",
                    "audioresample",
                    "avenc_aac",
                ]);
            }
        }

        if !audio {
            elements.retain(|el| !el.contains("audio"));
        }
        if !video {
            elements.retain(|el| !el.contains("video") && !el.contains("h264"));
        }

        Self {
            family: family.clone(),
            elements: elements.into_iter().map(str::to_string).collect(),
            wait_for_eos_on_stop: true,
            stage: DestinationPipelineStage::Idle,
        }
    }
}
```

---

## Verification

### Compile

```bash
cargo check -p migration-runtime
```

Clean once STEP-1 + STEP-3 are also applied (the match is non-exhaustive
otherwise).

### Profile unit tests (STEP-4, host, no GStreamer)

```bash
cargo test -p migration-runtime -- srt_profile
```

- `srt_profile_lists_srtsink_and_mpegtsmux` — confirms `srtsink`, `mpegtsmux`,
  `h264parse`, `avenc_aac` present with `audio=true, video=true`.
- `srt_profile_audio_disabled_removes_audio_elements` — confirms
  `audioconvert` gone, `srtsink` retained with `audio=false, video=true`.

### Spot-check via `getinfo` (on device, after STEP-3)

```bash
curl -s -X POST http://127.0.0.1:8080/command \
  -d '{"createdestination":{"id":"srt-dbg","family":{"Srt":{"uri":"srt://10.0.0.42:9000"}},"audio":true,"video":true}}'
curl -s -X POST http://127.0.0.1:8080/command -d '{"getinfo":{"id":"srt-dbg"}}'
# → the destination's "elements" array includes "srtsink" and "mpegtsmux".
```

---

## Done — STEP-2 complete

| Sub-step | Status |
|---|---|
| 2A from_family arm | ✅ |
| 2B element roles & filter | ✅ |
| 2C full function & verification | ✅ |

→ Next top-level step: [../step-3/INDEX.md](../step-3/INDEX.md)
