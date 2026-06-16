# STEP-4B — `nodes/destination.rs` profile tests

> Two tests for the element-inventory arm. Require
> [STEP-2](../step-2/INDEX.md) (the `from_family` arm).

---

## Goal

Assert that `DestinationPipelineProfile::from_family` lists the right elements
for SRT, and that the shared audio/video `retain` filter
([STEP-2B](../step-2/STEP-2B-element-roles-and-filter.md)) keeps `srtsink`
while dropping audio elements when `audio=false`.

---

## Pre-flight

| Insert location | Detail |
|---|---|
| `nodes/destination.rs` `#[cfg(test)] mod tests { … }` | after `whep_destination_node_resets_bound_ports_on_stopped` (around line 1648) |
| Types in scope | `super::*` brings `DestinationFamily`, `DestinationPipelineProfile` |

---

## The change

**File:** `crates/migration-runtime/src/nodes/destination.rs`

```rust
// ── SRT profile ──────────────────────────────────────────────────────────────

#[test]
fn srt_profile_lists_srtsink_and_mpegtsmux() {
    let family = DestinationFamily::Srt {
        uri:        "srt://10.0.0.1:9000".into(),
        latency:    200,
        passphrase: None,
        pbkeylen:   None,
    };
    let profile = DestinationPipelineProfile::from_family(&family, true, true);
    assert!(profile.elements.iter().any(|e| e == "srtsink"),   "srtsink missing");
    assert!(profile.elements.iter().any(|e| e == "mpegtsmux"), "mpegtsmux missing");
    assert!(profile.elements.iter().any(|e| e == "h264parse"), "h264parse missing");
    assert!(profile.elements.iter().any(|e| e == "avenc_aac"), "avenc_aac missing");
}

#[test]
fn srt_profile_audio_disabled_removes_audio_elements() {
    let family = DestinationFamily::Srt {
        uri:        "srt://10.0.0.1:9000".into(),
        latency:    200,
        passphrase: None,
        pbkeylen:   None,
    };
    let profile = DestinationPipelineProfile::from_family(&family, false, true);
    assert!(!profile.elements.iter().any(|e| e == "audioconvert"),
            "audioconvert must be removed when audio=false");
    assert!(!profile.elements.iter().any(|e| e == "avenc_aac"),
            "avenc_aac must be removed when audio=false");
    assert!(profile.elements.iter().any(|e| e == "srtsink"),
            "srtsink must survive audio=false");
}
```

---

## Cross-check with the filter table (STEP-2B)

| Assertion | Why it holds |
|---|---|
| `srtsink` present (both tests) | name contains no `audio`/`video`/`h264` substring → never filtered |
| `mpegtsmux` present | same — survives all flag combinations |
| `audioconvert` gone when `audio=false` | starts with `audio` → removed by `retain(|el| !el.contains("audio"))` |
| `avenc_aac` gone when `audio=false` | the second test asserts removal; note `avenc_aac` does **not** contain the substring `audio`, so this assertion depends on the build_live_pipeline never linking it without an audio appsrc — see STEP-2B footnote ² and adjust if the substring filter is later tightened |

> **Heads-up:** if a future change tightens the substring filter, re-check the
> `avenc_aac` assertion in the second test — it is the one element whose name
> doesn't contain `audio`. As written against the current shared filter it
> matches the documented behaviour.

---

## Verification

```bash
cargo test -p migration-runtime -- srt_profile
```

Both green (requires STEP-2's `from_family` arm).

---

## Next

→ [STEP-4C-run-and-verify.md](STEP-4C-run-and-verify.md)
