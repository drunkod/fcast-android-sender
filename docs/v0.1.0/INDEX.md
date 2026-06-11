# v0.1.0 — Implementation Index

> MVP cast loop: RTMP · SRT · WHEP destinations + protocol settings UI + connect wiring.
> Each step is self-contained — goal, pre-flight, full code, verification.
>
> **Landing order:** Steps 1+2+3 must land in a single commit (non-exhaustive match otherwise).
> Steps 4–8 are independent and can land separately.

---

## Step map

| Step | File | Scope | Net Δ |
|------|------|-------|-------|
| 1 | [step-1/](step-1/INDEX.md) (1A · 1B · 1C · 1D) | Add `DestinationFamily::Srt` to `protocol.rs` — split into 4 sub-steps | ~20 lines Rust |
| 2 | [step-2/](step-2/INDEX.md) (2A · 2B · 2C) | Extend `DestinationPipelineProfile::from_family` with SRT arm — split into 3 sub-steps | ~12 lines Rust |
| 3 | [step-3/](step-3/INDEX.md) (3A · 3B · 3C · 3D · 3E) | Full `build_live_pipeline` SRT match arm — split into 5 sub-steps | ~85 lines Rust |
| 4 | [step-4/](step-4/INDEX.md) (4A · 4B · 4C) | 6 host-runnable unit tests (no GStreamer required) — split into 3 sub-steps | ~80 lines Rust |
| 5 | [step-5/](step-5/INDEX.md) (5A · 5B · 5C · 5D) | `Panel` variants + `SrtDestination` struct in `bridge.slint` — split into 4 sub-steps | ~25 lines Slint |
| 6 | [step-6/](step-6/INDEX.md) (6A · 6B · 6C · 6D) | `protocol_rtmp_settings_page.slint` — slintcn Switch + Badge + Button + Card — split into 4 sub-steps | ~90 lines Slint |
| 7 | [step-7/](step-7/INDEX.md) (7A · 7B · 7C · 7D · 7E) | `protocol_srt_settings_page.slint` — slintcn Input + Switch + Badge + Button + Card + in-file CyclerRow — split into 5 sub-steps | ~160 lines Slint |
| 8 | [step-8/](step-8/INDEX.md) (8A · 8B · 8C · 8D) | `connect_page.slint` — receiver list wiring — split into 4 sub-steps | ~65 lines Slint |

---

## Current-state snapshot

| Component | Status | Location |
|---|---|---|
| `DestinationFamily::Rtmp/Udp/Whep/LocalFile/LocalPlayback` | ✅ done | `crates/migration-runtime/src/protocol.rs:191` |
| `DestinationFamily::Srt` | ❌ **Steps 1–3** | to add |
| `srt` GStreamer plugin in Android.mk | ✅ done | `app/jni/Android.mk:63` — no change needed |
| RTMP/UDP/WHEP `build_live_pipeline` | ✅ done | `crates/migration-runtime/src/nodes/destination.rs` |
| SRT `build_live_pipeline` | ❌ **Step 3** | to add |
| `Panel::protocol-rtmp-settings` / `protocol-srt-settings` | ❌ **Step 5** | to add: `ui/bridge.slint:139` |
| `protocol_rtmp_settings_page.slint` | ❌ **Step 6** | to create: `ui/pages/` |
| `protocol_srt_settings_page.slint` | ❌ **Step 7** | to create: `ui/pages/` |
| `ConnectView` | ❌ placeholder **Step 8** | `ui/pages/connect_page.slint` |

---

## Commit discipline

Steps 1 + 2 + 3 break `match` exhaustiveness independently — squash them:

```
git add crates/migration-runtime/src/protocol.rs \
        crates/migration-runtime/src/nodes/destination.rs
git commit -m "feat(srt): add DestinationFamily::Srt + pipeline profile + build_live_pipeline arm"
```

Steps 4–8 can each land as a separate commit.

---

## slintcn components used in Steps 6–8

All listed components are already installed in `ui/slintcn/components/` —
verified present. **`select.slint` is NOT installed**, so Step 7 uses a
dependency-free inline cycler instead of `Select`.

| Component | Import path (from `ui/pages/`) | Used in | Installed? |
|---|---|---|---|
| `Switch` | `../slintcn/components/switch.slint` | Steps 6, 7 | ✅ |
| `Input`  | `../slintcn/components/input.slint`  | Steps 7, 8 | ✅ |
| `Badge`  | `../slintcn/components/badge.slint`  | Steps 6, 7, 8 (`BadgeVariant` + `BadgeSize`) | ✅ |
| `Button` | `../slintcn/components/button.slint` | Steps 6, 7, 8 | ✅ |
| `Card`   | `../slintcn/components/card.slint`   | Steps 6, 7, 8 | ✅ |
| ~~`Select`~~ | — | replaced by in-file `CyclerRow` in Step 7 | ❌ not installed |

`RowColors` (from `../components/settings_rows.slint`) is imported in Steps
6–8 for the shared `normal` / `pressed` row hex values instead of hardcoding
them.
