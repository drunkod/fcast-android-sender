# STEP-2A — Add the `Srt` arm to `from_family`

> The only code edit in STEP-2. **Requires STEP-1's `Srt` variant** and is
> what un-breaks the `from_family` match — squash with STEP-1 + STEP-3.

---

## Goal

Add a `DestinationFamily::Srt { .. }` arm to
`DestinationPipelineProfile::from_family` that lists the GStreamer elements the
SRT pipeline uses. Mirrors the existing `Udp` arm, swapping `udpsink` for
`srtsink`.

---

## Pre-flight

| Exists (template to mirror) | Location |
|---|---|
| `from_family` signature | `crates/migration-runtime/src/nodes/destination.rs:54` |
| `Udp` arm (closest template) | `nodes/destination.rs:73–82` |
| `Whep` arm (insert the new arm after this) | `nodes/destination.rs:107–111` |

The `Udp` arm being mirrored:

```rust
DestinationFamily::Udp { .. } => {
    elements.extend([
        "mpegtsmux",
        "udpsink",
        "videoconvert",
        "h264enc",
        "h264parse",
        "audioconvert",
        "audioresample",
        "avenc_aac",
    ]);
}
```

---

## The change

**File:** `crates/migration-runtime/src/nodes/destination.rs`

Insert after the `Whep` arm (around line 111), before the closing `}` of the
outer `match`:

```rust
// ── NEW (STEP-2A) ──────────────────────────────────────────────────────────
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
```

The only difference from `Udp` is `"udpsink"` → `"srtsink"`. The arm binds
`{ .. }` (ignores all fields) because the element *names* don't depend on
`uri`/`latency`/`passphrase`/`pbkeylen` — those drive property values in
STEP-3's `build_live_pipeline`, not the inventory list.

---

## Verification

```bash
cargo check -p migration-runtime
```

Compiles once STEP-1 (variant) and STEP-3 (`build_live_pipeline` arm) are also
applied. Element-role rationale → [STEP-2B](STEP-2B-element-roles-and-filter.md).

---

## Next

→ [STEP-2B-element-roles-and-filter.md](STEP-2B-element-roles-and-filter.md)
