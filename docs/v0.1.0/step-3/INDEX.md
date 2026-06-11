# STEP-3 — SRT `build_live_pipeline` arm (sub-steps)

> Split of the original STEP-3 (the largest step) into five self-contained
> sub-steps along the pipeline's natural seams.
> **Lands in the same commit** as STEP-1 + STEP-2 — the `Srt` variant makes
> the `match` in `build_live_pipeline` non-exhaustive until 3A–3D are present.

---

## Sub-step map

| # | File | Scope | Net Δ |
|---|------|-------|-------|
| 3A | [STEP-3A-mux-sink-setup.md](STEP-3A-mux-sink-setup.md) | Create `mpegtsmux` + `srtsink`, set `alignment`, `uri`, `latency`, `wait-for-connection`, encryption | ~30 lines |
| 3B | [STEP-3B-video-chain.md](STEP-3B-video-chain.md) | `appsrc(video) → videoconvert → encoder → h264parse → mux` | ~30 lines |
| 3C | [STEP-3C-audio-chain.md](STEP-3C-audio-chain.md) | `appsrc(audio) → audioconvert → audioresample → avenc_aac → mux` | ~25 lines |
| 3D | [STEP-3D-assemble-and-link.md](STEP-3D-assemble-and-link.md) | `mux.link(sink)`, full assembled arm, diagram, SRT-vs-UDP diff | docs + ~2 lines |
| 3E | [STEP-3E-smoke-and-pitfalls.md](STEP-3E-smoke-and-pitfalls.md) | End-to-end smoke test + P1–P4 pitfalls catalogue | docs only |

Single file edited (3A–3D): `crates/migration-runtime/src/nodes/destination.rs`.

---

## What this step is

`DestinationNode::build_live_pipeline` constructs the **real** GStreamer graph
(`gst::Pipeline` + elements + links). The `Srt` arm is structurally identical
to the existing `Udp` arm — `mpegtsmux → srtsink` instead of
`mpegtsmux → udpsink` — plus three SRT-specific `srtsink` properties.

### Is there any UI in STEP-3?

**No.** This is the deepest backend step — pure GStreamer wiring in Rust.
The slintcn UI that *triggers* this pipeline (the Save / Stop buttons, the
URI/latency/encryption inputs) is **STEP-7**
([../step-7/INDEX.md](../step-7/INDEX.md)); the
property values set here (`uri`, `latency`, `passphrase`, `pbkeylen`) come
from the Bridge fields STEP-7 writes and the Rust handler forwards.

```
STEP-7 (UI, slintcn)        STEP-5 (Bridge)             STEP-3 (this)
Input/CyclerRow/Switch  ──► srt-destination.uri     ──► srtsink "uri"
                            srt-destination.latency-ms  srtsink "latency"
                            srt-destination-pbkeylen-idx srtsink "pbkeylen"
                            srt-destination-passphrase   srtsink "passphrase"
```

---

## Landing order

```
3A (setup) ─► 3B (video) ─► 3C (audio) ─► 3D (link + assemble)
                                              │
                                              └─ squash with STEP-1 + STEP-2
3E (smoke + pitfalls) — verify after the commit
```

→ Next top-level step: [../step-4/INDEX.md](../step-4/INDEX.md)
