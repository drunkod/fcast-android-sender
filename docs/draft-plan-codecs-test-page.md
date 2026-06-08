## Repository: kodyka/fcast-android-sender

This plan implements the codec test page end-to-end: Kotlin code to query Android's MediaCodecList API, JNI bridge to call it from Rust, Bridge properties/callbacks in Slint, and updated UI.

---

### Step 1: Create Kotlin `CodecDump.kt`

**File:** `app/src/main/java/org/fcast/android/sender/codec/CodecDump.kt` (new)

Create a Kotlin object `CodecDump` based on the user's research. It should contain three public methods that each return a `String` report:

1. `dumpAllCodecsToLog()` — Iterates `MediaCodecList(MediaCodecList.ALL_CODECS).codecInfos`, logging encoder/decoder, hardware flags (API 29+), supported MIME types, color formats, profile/levels, video capabilities (widths, heights, bitrate, framerates, `areSizeAndRateSupported` for common resolutions), and audio capabilities.

2. `quickFindCodecsForFormats()` — Uses `MediaCodecList.findDecoderForFormat` / `findEncoderForFormat` for common formats (H.264, H.265, AV1, AAC, Opus at 1920x1080 / 48kHz stereo).

3. `smokeTestVideoEncoders()` — For each of H.264 1280x720@30, H.264 1920x1080@30, H.265 1920x1080@30, AV1 1280x720@30: finds an encoder, calls `MediaCodec.createByCodecName`, `configure`, `createInputSurface`, `start`, and reports CONFIGURE_START_OK or FAILED.

Use the complete `CodecDump.kt` code from the user's research message (the second, longer version). The package should be `org.fcast.android.sender.codec`.

---

### Step 2: Add Bridge properties and callbacks in `bridge.slint`

**File:** `ui/bridge.slint`

Add near the existing `test-status` or codec-test related section (around line 250, in the callbacks area):

```slint
// ── Codec test (Phase 8 wiring) ──────────────────────────────────────
in property <string> codec-test-log: "";
in property <bool> codec-test-running: false;
callback run-codec-test();          // full dump + quick-find + smoke test
callback run-codec-dump-only();     // just the codec list dump (faster)
callback run-codec-smoke-only();    // just the encoder smoke test
```

The `codec-test-log` property replaces the page's `mock-log`. `codec-test-running` lets the UI disable the button while running.

---

### Step 3: Update `codec_test_page.slint` UI

**File:** `ui/pages/codec_test_page.slint`

Replace the mock-log placeholder with real Bridge bindings:

1. Remove the `in-out property <string> mock-log: ...` stub property.
2. Change the "Run encoder test" button to invoke `Bridge.run-codec-test()` and bind `enabled` to `!Bridge.codec-test-running`.
3. Add a second button "Dump codecs only" that invokes `Bridge.run-codec-dump-only()` (for a faster, non-destructive listing).
4. Change the ScrollView's Text to bind to `Bridge.codec-test-log` instead of `root.mock-log`.
5. Optionally add a small status line or spinner while `Bridge.codec-test-running` is true.

The result should look like:

```slint
PrimaryButton {
    label: Bridge.codec-test-running ? @tr("Running…") : @tr("Run full codec test");
    enabled: !Bridge.codec-test-running;
    clicked => { Bridge.run-codec-test(); }
}
PrimaryButton {
    label: @tr("Dump codecs only");
    enabled: !Bridge.codec-test-running;
    clicked => { Bridge.run-codec-dump-only(); }
}

ScrollView {
    mouse-drag-pan-enabled: true;
    vertical-stretch: 1;
    Text {
        text: Bridge.codec-test-log == "" 
            ? @tr("Press a button above to start.")
            : Bridge.codec-test-log;
        color: Theme.text-secondary;
        font-size: Theme.font-size-label;
        wrap: word-wrap;
    }
}
```

---

### Step 4: Add JNI bridge for codec dump

**File:** `src/jni_bridge/codec_test.rs` (new)

Create a new JNI bridge module that calls `CodecDump` methods from Rust. Follow the pattern in `src/jni_bridge/main_activity.rs`.

The function should:
1. Get the JNIEnv (via `android_activity` or the stored JavaVM — follow whichever pattern `main_activity.rs` uses).
2. Find the `org/fcast/android/sender/codec/CodecDump` class.
3. Call the static method (e.g., `dumpAllCodecsToLog` which returns `String`).
4. Convert the returned jstring to a Rust String and return it.

Example structure:
```rust
#[cfg(target_os = "android")]
pub fn run_codec_dump_all() -> Result<String, String> {
    // Get JNIEnv from the stored JavaVM
    // Call CodecDump.dumpAllCodecsToLog() via JNI
    // Return the report string
}

#[cfg(target_os = "android")]
pub fn run_codec_smoke_test() -> Result<String, String> {
    // Call CodecDump.smokeTestVideoEncoders()
}

#[cfg(target_os = "android")]
pub fn run_codec_quick_find() -> Result<String, String> {
    // Call CodecDump.quickFindCodecsForFormats()
}
```

Also register the module in `src/jni_bridge/mod.rs`:
```rust
pub mod codec_test;
```

**Important:** Check how existing JNI calls obtain the `JNIEnv`. The app likely stores the `JavaVM` in a global (look at how `main_activity.rs` or `helpers.rs` get their JNI environment). Use the same pattern.

---

### Step 5: Register Bridge callback handlers in Rust

**File:** `src/android_main.rs`

Add callback handler registrations near the existing `on_invoke_action` or migration test callback registrations (around line 935+). Follow the pattern from MVP-PHASE-9's `on_run_migration_test` handler:

```rust
// ── Codec test callbacks ─────────────────────────────────────────────
ui.global::<Bridge>().on_run_codec_test({
    let ui_weak = ui.as_weak();
    move || {
        // Set running state
        let _ = ui_weak.upgrade_in_event_loop(|ui| {
            ui.global::<Bridge>().set_codec_test_running(true);
            ui.global::<Bridge>().set_codec_test_log("Running full codec test…\n".into());
        });
        
        let ui_weak_inner = ui_weak.clone();
        std::thread::spawn(move || {
            let mut report = String::new();
            
            // Call JNI bridge functions
            match crate::jni_bridge::codec_test::run_codec_dump_all() {
                Ok(r) => report.push_str(&r),
                Err(e) => report.push_str(&format!("FAIL dump: {e}\n")),
            }
            report.push_str("\n===== QUICK FIND =====\n");
            match crate::jni_bridge::codec_test::run_codec_quick_find() {
                Ok(r) => report.push_str(&r),
                Err(e) => report.push_str(&format!("FAIL quick-find: {e}\n")),
            }
            report.push_str("\n===== SMOKE TEST =====\n");
            match crate::jni_bridge::codec_test::run_codec_smoke_test() {
                Ok(r) => report.push_str(&r),
                Err(e) => report.push_str(&format!("FAIL smoke: {e}\n")),
            }
            
            let _ = ui_weak_inner.upgrade_in_event_loop(move |ui| {
                ui.global::<Bridge>().set_codec_test_log(report.into());
                ui.global::<Bridge>().set_codec_test_running(false);
            });
        });
    }
});

ui.global::<Bridge>().on_run_codec_dump_only({
    let ui_weak = ui.as_weak();
    move || {
        let _ = ui_weak.upgrade_in_event_loop(|ui| {
            ui.global::<Bridge>().set_codec_test_running(true);
            ui.global::<Bridge>().set_codec_test_log("Dumping codecs…\n".into());
        });
        let ui_weak_inner = ui_weak.clone();
        std::thread::spawn(move || {
            let report = match crate::jni_bridge::codec_test::run_codec_dump_all() {
                Ok(r) => r,
                Err(e) => format!("FAIL: {e}\n"),
            };
            let _ = ui_weak_inner.upgrade_in_event_loop(move |ui| {
                ui.global::<Bridge>().set_codec_test_log(report.into());
                ui.global::<Bridge>().set_codec_test_running(false);
            });
        });
    }
});
```

Use `std::thread::spawn` (not `tokio::spawn`) because the JNI calls are blocking. This matches the existing pattern used by the migration test handlers.

---

### Step 6: Verification

1. Build check: `cargo check -p android-sender --target aarch64-linux-android`
2. Build the APK: `./gradlew assembleDebug`
3. Install and run on device: `adb install -r app/build/outputs/apk/debug/app-debug.apk`
4. Open Settings → "H.264 encoder test" → tap "Run full codec test"
5. Verify the log area fills with codec information
6. Check logcat: `adb logcat -s CodecDump`

---

### Notes

- The JNI bridge is the trickiest part. Study how `src/jni_bridge/main_activity.rs` and `src/jni_bridge/helpers.rs` obtain the JNI environment to call Java methods. The codec dump methods are static on a Kotlin `object`, so you call them as static Java methods.
- The `CodecDump` Kotlin object compiles to a class with a `INSTANCE` singleton and static methods annotated with `@JvmStatic` — or if methods are on the `object` directly, JNI sees them as static methods on the companion. You may need to add `@JvmStatic` annotations to the three public methods in `CodecDump.kt`.
- For non-Android targets (host tests), the JNI functions won't exist. Gate them with `#[cfg(target_os = "android")]` and provide a stub for host builds that returns a placeholder string.
