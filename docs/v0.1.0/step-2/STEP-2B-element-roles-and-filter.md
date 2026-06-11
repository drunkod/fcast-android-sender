# STEP-2B — Element roles & the audio/video filter

> Documentation sub-step (no code edit). Explains what each element in the
> 2A list does and how the shared `retain` filter prunes the list when
> `audio` or `video` is disabled.

---

## Element roles

| Element | Role |
|---|---|
| `mpegtsmux` | Multiplexes H.264 video + AAC audio into an MPEG-TS container |
| `srtsink` | Network sink — pushes MPEG-TS over the SRT protocol |
| `videoconvert` | Pixel-format adaptation before the encoder |
| `h264enc` | H.264 encoder (**placeholder name** — see below) |
| `h264parse` | Parses the encoded H.264 stream for the muxer |
| `audioconvert` | Audio sample-format adaptation |
| `audioresample` | Resamples audio to the encoder's required rate |
| `avenc_aac` | AAC audio encoder |

> **`h264enc` is a placeholder in the inventory list only.** The real factory
> is chosen at pipeline-build time by `select_video_encoder` (STEP-3):
> `amcvidenc-*` (hardware) on Android, `x264enc`/`openh264enc` on host. The
> profile list is for `getinfo` diagnostics — it never instantiates elements.

---

## The shared `audio` / `video` filter

After the `match`, `from_family` prunes the element list based on the
`audio`/`video` flags. This code is **shared by every arm** (not SRT-specific):

```rust
if !audio {
    elements.retain(|el| !el.contains("audio"));
}
if !video {
    elements.retain(|el| !el.contains("video") && !el.contains("h264"));
}
```

### How the SRT list survives each case

| Element | Contains `"audio"`? | Contains `"video"`/`"h264"`? | Kept when `audio=false`? | Kept when `video=false`? |
|---|---|---|---|---|
| `mpegtsmux` | no | no | ✅ | ✅ |
| `srtsink` | no | no | ✅ | ✅ |
| `videoconvert` | no | `video` | ✅ | ❌ removed |
| `h264enc` | no | `h264` | ✅ | ❌ removed |
| `h264parse` | no | `h264` | ✅ | ❌ removed |
| `audioconvert` | `audio` | no | ❌ removed | ✅ |
| `audioresample` | no¹ | no | ✅¹ | ✅ |
| `avenc_aac` | no | no | ✅² | ✅ |

¹ `audioresample` does **not** contain the substring `"audio"`? It does —
`audioresample` starts with `audio`. So it **is** removed when `audio=false`.
² `avenc_aac` does **not** contain `"audio"`, so the substring filter leaves it;
this is a pre-existing quirk of the substring approach shared by the UDP arm —
harmless because with no audio appsrc, STEP-3 never links `avenc_aac` anyway.

> **Key invariant for SRT:** `srtsink` and `mpegtsmux` survive **both**
> `audio=false` and `video=false`, because neither name contains the filtered
> substrings. This is exactly what the STEP-4 test
> `srt_profile_audio_disabled_removes_audio_elements` asserts.

---

## Why mirror UDP's substring filter instead of fixing it?

The substring filter is imperfect (it relies on element names containing
`"audio"`/`"video"`/`"h264"`), but it is the **existing, tested behaviour**
shared by all five current arms. STEP-2 deliberately mirrors `Udp` so SRT
behaves identically to the protocol it most resembles. Reworking the filter is
out of scope — it would change behaviour for every family, not just SRT.

---

## Next

→ [STEP-2C-full-function-verification.md](STEP-2C-full-function-verification.md)
