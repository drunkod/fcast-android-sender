# Step 1 — `ui/bridge.slint`: add `codec-perf` to `Panel`

← [Index](README.md) · Next → [Step 2](step-2-bridge-props.md)

The `Panel` enum starts at **line 113**; `codec-test,` is at **line 117**. Add
`codec-perf,` directly after it.

```slint
export enum Panel {
    none,
    settings,
    debug,
    codec-test,
    codec-perf,          // ← ADD THIS LINE
    backup-reset,
    audio,
    camera,
    quick-actions,
    cast-history,
    cast-history-detail,
    recording,
    pairing,
    receiver-rename,
    bitrate-presets,
    bitrate-preset-edit,
    macros,
    macro-edit,
    debug-log,
    debug-video,
    network,
    mixer,
    media-backend,
    test-functionality,
    camera-rtmp-stream,
    rtmp-wizard,
}
```

---

← [Index](README.md) · Next → [Step 2](step-2-bridge-props.md)
