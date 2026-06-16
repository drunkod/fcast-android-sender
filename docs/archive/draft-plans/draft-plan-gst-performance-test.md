## Repository: kodyka/fcast-android-sender

Implement a new Codec Performance page that runs real GStreamer encode/decode pipelines using androidmedia (AMC) elements and measures throughput (FPS). All code snippets are provided in the conversation above — use them as drop-in replacements/additions.

### Step 1: Add `codec-perf` Panel variant
Edit `ui/bridge.slint`. In the `export enum Panel { ... }` block (line 113), add `codec-perf,` after `codec-test,` (after line 117).

### Step 2: Add Bridge properties and callbacks
Edit `ui/bridge.slint`. Inside `export global Bridge { ... }`, before the `// ── Public functions` comment (line 462), insert:
```
    in property <string> perf-test-log: "";
    in property <bool>   perf-test-running: false;
    callback run-perf-test();
    callback run-perf-encode-only();
    callback run-perf-decode-only();
    callback run-perf-list-factories();
```

### Step 3: Create `ui/pages/codec_perf_page.slint`
Create new file with the full Slint page content from the conversation. It has 4 buttons (Full benchmark, List factories, Encode only, Decode only) and a ScrollView log area bound to `Bridge.perf-test-log`.

### Step 4: Route in `ui/main.slint`
1. Add import after line 70: `import { CodecPerfPage } from "pages/codec_perf_page.slint";`
2. Add route inside PanelHost after line 178: `if PanelBridge.active == Panel.codec-perf: CodecPerfPage { }`

### Step 5: Add settings entry in `ui/pages/settings_page.slint`
After the "H.264 encoder test" SettingsValueRow (line 340), add a new SettingsValueRow with icon "⚡", title "Codec performance test", that pushes `Panel.codec-perf`.

### Step 6: Create `src/codec_perf.rs`
Create new file with the full Rust benchmark module from the conversation. It uses `gst::parse::launch()` and pad probes to count buffers at fakesink, auto-discovers `amcvidenc-*` / `amcviddec-*` factories from the GStreamer registry. Public functions: `list_codec_factories()`, `run_encode_benchmarks()`, `run_decode_benchmarks()`, `run_full_benchmark()`.

### Step 7: Register module in `src/lib.rs`
Add `pub mod codec_perf;` after line 10 (after `pub mod application;`).

### Step 8: Wire Bridge callbacks in `src/android_main.rs`
After the `on_pick_test_overlay_image` handler (line 1199), before the `use crate::jni_bridge::camera::` block (line 1201), insert the 4 callback registrations: `on_run_perf_test`, `on_run_perf_encode_only`, `on_run_perf_decode_only`, `on_run_perf_list_factories`. All use `std::thread::spawn` for blocking GStreamer operations and `upgrade_in_event_loop` to update the UI.

### Step 9: Verify
1. `cargo check` for Rust compilation
2. Build the APK
3. Install on device, open Settings → "Codec performance test"
4. Tap "List factories" — should list amcvidenc-*/amcviddec-* entries
5. Tap "Full benchmark" — should run encode/decode tests and report FPS
6. Check logcat for `perf-bench` log lines

### Important notes
- No Kotlin/JNI needed — the benchmark is pure Rust using the `gst` crate
- GStreamer is already initialized via `ensure_gstreamer_initialized()` in `src/platform/gst_init.rs`
- The `androidmedia` plugin is already in `app/jni/Android.mk` GSTREAMER_PLUGINS list (line 75)
- `videotestsrc` generates synthetic frames so no test files needed on device
- 300 buffers with 30s timeout per test gives enough data while keeping test duration reasonable
- Uses `parking_lot::Mutex` (already a crate dependency) for buffer timing
