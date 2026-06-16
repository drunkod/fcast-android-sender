# STEP-1 — RIST destination (sub-steps)

> Fully independent of the scene/widget work — ship anytime. Mirrors the v0.1.0
> SRT arm (`mpegtsmux → ristsink`). **Moblin analogue:** `Media/RistServer/`.

| # | File | Scope |
|---|------|-------|
| 1A | [STEP-1A-protocol-variant.md](STEP-1A-protocol-variant.md) | `DestinationFamily::Rist` variant + default helpers |
| 1B | [STEP-1B-from-family-arm.md](STEP-1B-from-family-arm.md) | `from_family` element-inventory arm |
| 1C | [STEP-1C-build-live-pipeline.md](STEP-1C-build-live-pipeline.md) | full `build_live_pipeline` RIST arm |
| 1D | [STEP-1D-android-plugin.md](STEP-1D-android-plugin.md) | add `rist` to `Android.mk` |
| 1E | [STEP-1E-tests-and-smoke.md](STEP-1E-tests-and-smoke.md) | unit tests + on-device smoke + pitfalls |

**Squash 1A+1B+1C** (non-exhaustive `match` otherwise). 1D independent. 1E test-only.

→ Next: [../step-2/INDEX.md](../step-2/INDEX.md)
