# STEP-5A — Panel enum variants

> Add the two panel routes the SRT/RTMP settings pages (STEP-6/7) are pushed
> under.

---

## Goal

Add `protocol-rtmp-settings` and `protocol-srt-settings` to the `Panel` enum so
`PanelBridge.push(Panel.protocol-srt-settings)` resolves.

---

## Pre-flight

| Exists | Location |
|---|---|
| `Panel` enum (insert after `rtmp-wizard`) | `ui/bridge.slint:113–140` |
| Rust `Panel` usage is `== Panel::Variant` only (no exhaustive `match`) | `src/android_main.rs`, `src/backend/lifecycle.rs`, … |

Because Rust never matches `Panel` exhaustively, **adding variants cannot break
the Rust build** — verified across `src/`.

---

## The change

**File:** `ui/bridge.slint`

Append the two variants inside the `Panel` enum, after `rtmp-wizard` (line 139):

```slint
export enum Panel {
    none,
    settings,
    debug,
    codec-test,
    codec-perf,
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
    // ── v0.1.0 additions (STEP-5A) ──────────────────────────────────
    protocol-rtmp-settings,
    protocol-srt-settings,
}
```

---

## Generated Rust names

Slint converts kebab-case variants to PascalCase:

| Slint | Rust |
|---|---|
| `protocol-rtmp-settings` | `Panel::ProtocolRtmpSettings` |
| `protocol-srt-settings` | `Panel::ProtocolSrtSettings` |

Used in STEP-6/7's `main.slint` registration as
`if Bridge.active-panel == Panel.protocol-srt-settings : …`.

---

## Verification

```bash
slint-lsp ui/main.slint 2>&1 | grep -c error
# → 0
```

---

## Next

→ [STEP-5B-srt-destination-struct.md](STEP-5B-srt-destination-struct.md)
