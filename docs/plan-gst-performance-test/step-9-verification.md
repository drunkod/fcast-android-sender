# Step 9 — Verification & risks

← [Step 8](step-8-android-main-handlers.md) · [Index](README.md)

## Build & verify

1. Host compile (catches Rust + Slint codegen errors fast; uses the gst crate on host):
   `cargo check`
2. Real Android build (Gradle alone does **not** compile Rust/Slint):
   ```bash
   nix develop .#android -c bash scripts/build-deploy.sh
   ```
3. On device: Settings → **DEBUG** → **Codec performance test**.
4. Tap **List factories** → expect `amcvidenc-*` / `amcviddec-*` entries grouped
   into Encoders / Decoders / Other.
5. Tap **Full benchmark** → expect AVC/HEVC encode + decode FPS lines, a BEST
   result, and a RECOMMENDATION block. Takes ~1–2 min.
6. Scroll the report — should stay smooth (virtualised `ListView`).
7. Logs: `nix develop .#android -c bash -c 'adb logcat -s fcastsender RustStdoutStderr | grep perf-bench'`

## Expected warnings (non-blocking)

The research Rust source has a few unused items in `src/codec_perf.rs`:
- `let registry = gst::Registry::get();` — unused → `let _registry = …` or delete.
- `let mut got_eos = false;` — set but never read → drop it or `let mut _got_eos`.
- `use tracing::{error, info, warn};` — `warn` unused → remove `warn`.

Clean these to keep `cargo check` warning-free; none stop the build.

## Risks / notes

- **GStreamer API pinning.** Snippets target `gst = "gstreamer" 0.25`
  (`gst::parse::launch`, `ElementFactory::factories_with_type`, `PadProbeType`,
  `MessageView`). If a minor-version bump changes a signature, fix at `cargo check`.
- **`ensure_gstreamer_initialized()` is `pub(crate)`** — fine because `codec_perf`
  is in the same crate. Don't move it out of the crate without re-exporting.
- **Benchmark side effects / duration.** `Full benchmark` spins up real encoders
  AND decoders (encode→decode roundtrips) across multiple resolutions with a 30 s
  timeout each — worst case several minutes and real device load. The page disables
  all buttons via `perf-test-running` while a run is active; `List factories` is the
  cheap, safe first tap.
- **Thread vs UI.** Heavy work runs on `std::thread::spawn`; UI mutations only via
  `upgrade_in_event_loop`. Matches the codec-test handlers.
- **`amcviddec` raw output.** Some devices only emit decoder output to a Surface;
  a `videoconvert ! fakesink` roundtrip can fail or report 0 buffers on those —
  that surfaces as `FAILED`/`No decode pipeline succeeded`, not a crash.
- **Slint `ListView`** reintroduces a `std-widgets` import on this page — the same
  deliberate virtualisation exception used by `debug_log_page.slint` and
  `codec_test_page.slint`. Consistent, but it does cut against the no-std-widgets goal.

## Summary of files

| Action | File | Step |
|--------|------|------|
| Edit | `ui/bridge.slint` (Panel enum) | 1 |
| Edit | `ui/bridge.slint` (perf props + line model + callbacks) | 2 |
| New  | `ui/pages/codec_perf_page.slint` | 3 |
| Edit | `ui/main.slint` (import + route) | 4 |
| Edit | `ui/pages/settings_page.slint` (DEBUG row) | 5 |
| New  | `src/codec_perf.rs` | 6 |
| Edit | `src/lib.rs` (`pub mod codec_perf;`) | 7 |
| Edit | `src/android_main.rs` (4 handlers + `set_perf_log`) | 8 |

---

← [Step 8](step-8-android-main-handlers.md) · [Index](README.md)
