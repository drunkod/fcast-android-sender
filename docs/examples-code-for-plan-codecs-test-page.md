Here are the complete, drop-in code snippets for every file in the plan.

---

## 1. Kotlin: `CodecDump.kt`

**New file:** `app/src/main/java/org/fcast/android/sender/codec/CodecDump.kt`

```kotlin
package org.fcast.android.sender.codec

import android.media.MediaCodec
import android.media.MediaCodecInfo
import android.media.MediaCodecList
import android.media.MediaFormat
import android.os.Build
import android.util.Log
import java.lang.reflect.Modifier

object CodecDump {
    private const val TAG = "CodecDump"

    private val interestingVideoSizes = listOf(
        Triple(640, 360, 30.0),
        Triple(1280, 720, 30.0),
        Triple(1920, 1080, 30.0),
        Triple(1920, 1080, 60.0),
        Triple(3840, 2160, 30.0),
        Triple(3840, 2160, 60.0),
    )

    // ── Public entry points (called from Rust via JNI) ──────────────────

    @JvmStatic
    fun dumpAllCodecsToLog(): String {
        val report = StringBuilder()
        appendLine(report, "Android SDK: ${Build.VERSION.SDK_INT}")
        appendLine(report, "Device: ${Build.MANUFACTURER} ${Build.MODEL}")
        appendLine(report)

        dumpCodecList(report, MediaCodecList.ALL_CODECS, "ALL_CODECS")
        dumpCodecList(report, MediaCodecList.REGULAR_CODECS, "REGULAR_CODECS")

        val text = report.toString()
        Log.i(TAG, text)
        return text
    }

    @JvmStatic
    fun quickFindCodecsForFormats(): String {
        val report = StringBuilder()
        val list = MediaCodecList(MediaCodecList.REGULAR_CODECS)

        val formats = listOf(
            MediaFormat.createVideoFormat(MediaFormat.MIMETYPE_VIDEO_AVC, 1920, 1080),
            MediaFormat.createVideoFormat(MediaFormat.MIMETYPE_VIDEO_HEVC, 1920, 1080),
            MediaFormat.createVideoFormat(MediaFormat.MIMETYPE_VIDEO_AV1, 1920, 1080),
            MediaFormat.createAudioFormat(MediaFormat.MIMETYPE_AUDIO_AAC, 48_000, 2),
            MediaFormat.createAudioFormat(MediaFormat.MIMETYPE_AUDIO_OPUS, 48_000, 2),
        )

        for (format in formats) {
            val mime = format.getString(MediaFormat.KEY_MIME)
            val decoder = safe { list.findDecoderForFormat(format) }
            val encoder = safe {
                val encFormat = cloneForEncoder(format)
                list.findEncoderForFormat(encFormat)
            }
            appendLine(report, "Format: $mime")
            appendLine(report, "  decoder=$decoder")
            appendLine(report, "  encoder=$encoder")
            appendLine(report)
        }

        val text = report.toString()
        Log.i(TAG, text)
        return text
    }

    @JvmStatic
    fun smokeTestVideoEncoders(): String {
        val report = StringBuilder()

        val tests = listOf(
            EncoderTest(MediaFormat.MIMETYPE_VIDEO_AVC, 1280, 720, 30, 2_000_000),
            EncoderTest(MediaFormat.MIMETYPE_VIDEO_AVC, 1920, 1080, 30, 4_000_000),
            EncoderTest(MediaFormat.MIMETYPE_VIDEO_HEVC, 1920, 1080, 30, 4_000_000),
            EncoderTest(MediaFormat.MIMETYPE_VIDEO_AV1, 1280, 720, 30, 2_000_000),
        )

        for (test in tests) {
            appendLine(report, "Smoke test encoder: ${test.mime} ${test.width}x${test.height}@${test.fps}")
            appendLine(report, smokeTestOneVideoEncoder(test))
            appendLine(report)
        }

        val text = report.toString()
        Log.i(TAG, text)
        return text
    }

    // ── Internal helpers ────────────────────────────────────────────────

    private fun dumpCodecList(report: StringBuilder, kind: Int, title: String) {
        appendLine(report, "==============================")
        appendLine(report, title)
        appendLine(report, "==============================")

        val list = MediaCodecList(kind)
        val codecInfos = list.codecInfos
        appendLine(report, "Codec count: ${codecInfos.size}")
        appendLine(report)

        for (codec in codecInfos) {
            dumpCodec(report, codec)
        }
    }

    private fun dumpCodec(report: StringBuilder, codec: MediaCodecInfo) {
        val encoderDecoder = if (codec.isEncoder) "ENCODER" else "DECODER"
        appendLine(report, "--------------------------------")
        appendLine(report, "$encoderDecoder: ${codec.name}")

        if (Build.VERSION.SDK_INT >= 29) {
            appendLine(report, "  hardwareAccelerated=${codec.isHardwareAccelerated}")
            appendLine(report, "  softwareOnly=${codec.isSoftwareOnly}")
            appendLine(report, "  vendor=${codec.isVendor}")
            appendLine(report, "  alias=${codec.isAlias}")
        } else {
            appendLine(report, "  hw/sw/vendor/alias flags require Android 10+")
        }

        val supportedTypes = codec.supportedTypes.sorted()
        appendLine(report, "  supportedTypes=${supportedTypes.joinToString()}")

        for (mime in supportedTypes) {
            dumpMimeCapabilities(report, codec, mime)
        }
        appendLine(report)
    }

    private fun dumpMimeCapabilities(
        report: StringBuilder,
        codec: MediaCodecInfo,
        mime: String,
    ) {
        appendLine(report, "    MIME: $mime")

        val caps = try {
            codec.getCapabilitiesForType(mime)
        } catch (e: Throwable) {
            appendLine(report, "      cannot read capabilities: ${e.javaClass.simpleName}: ${e.message}")
            return
        }

        if (Build.VERSION.SDK_INT >= 23) {
            appendLine(report, "      maxSupportedInstances=${caps.maxSupportedInstances}")
        }

        appendLine(report, "      defaultFormat=${safe { caps.defaultFormat.toString() }}")
        appendLine(report, "      features=${supportedFeatures(caps).joinToString()}")

        appendLine(report, "      colorFormats:")
        for (color in caps.colorFormats) {
            appendLine(report, "        $color = ${colorFormatName(color)}")
        }

        appendLine(report, "      profileLevels:")
        for (pl in caps.profileLevels) {
            appendLine(report, "        profile=${pl.profile}, level=${pl.level}")
        }

        if (mime.startsWith("video/")) {
            dumpVideoCapabilities(report, caps)
        }
        if (mime.startsWith("audio/")) {
            dumpAudioCapabilities(report, caps)
        }
    }

    private fun dumpVideoCapabilities(
        report: StringBuilder,
        caps: MediaCodecInfo.CodecCapabilities,
    ) {
        val video = safe { caps.videoCapabilities } ?: return

        appendLine(report, "      videoCapabilities:")
        appendLine(report, "        widths=${safe { video.supportedWidths }}")
        appendLine(report, "        heights=${safe { video.supportedHeights }}")
        appendLine(report, "        frameRates=${safe { video.supportedFrameRates }}")
        appendLine(report, "        bitrateRange=${safe { video.bitrateRange }}")
        appendLine(report, "        widthAlignment=${safe { video.widthAlignment }}")
        appendLine(report, "        heightAlignment=${safe { video.heightAlignment }}")

        for ((w, h, fps) in interestingVideoSizes) {
            val ok = safe { video.areSizeAndRateSupported(w, h, fps) } ?: false
            appendLine(report, "        supports ${w}x${h}@${fps.toInt()}fps = $ok")

            if (Build.VERSION.SDK_INT >= 23) {
                val achievable = safe { video.getAchievableFrameRatesFor(w, h) }
                appendLine(report, "          achievableFps=$achievable")
            }
        }

        if (Build.VERSION.SDK_INT >= 29) {
            val performancePoints = safe { video.supportedPerformancePoints }
            appendLine(report, "        performancePoints=$performancePoints")
        }
    }

    private fun dumpAudioCapabilities(
        report: StringBuilder,
        caps: MediaCodecInfo.CodecCapabilities,
    ) {
        val audio = safe { caps.audioCapabilities } ?: return
        appendLine(report, "      audioCapabilities:")
        appendLine(report, "        bitrateRange=${safe { audio.bitrateRange }}")
        appendLine(report, "        maxInputChannelCount=${safe { audio.maxInputChannelCount }}")
        appendLine(report, "        supportedSampleRateRanges=${safe { audio.supportedSampleRateRanges.joinToString() }}")
        appendLine(report, "        supportedSampleRates=${safe { audio.supportedSampleRates?.joinToString() }}")
    }

    private fun smokeTestOneVideoEncoder(test: EncoderTest): String {
        val format = MediaFormat.createVideoFormat(test.mime, test.width, test.height).apply {
            setInteger(MediaFormat.KEY_BIT_RATE, test.bitrate)
            setInteger(MediaFormat.KEY_FRAME_RATE, test.fps)
            setInteger(MediaFormat.KEY_I_FRAME_INTERVAL, 1)
            setInteger(
                MediaFormat.KEY_COLOR_FORMAT,
                MediaCodecInfo.CodecCapabilities.COLOR_FormatSurface,
            )
        }

        val list = MediaCodecList(MediaCodecList.REGULAR_CODECS)
        val codecName = safe { list.findEncoderForFormat(format) }

        if (codecName == null) {
            return "  result=NO_ENCODER_FOUND"
        }

        var codec: MediaCodec? = null
        return try {
            codec = MediaCodec.createByCodecName(codecName)
            codec.configure(format, null, null, MediaCodec.CONFIGURE_FLAG_ENCODE)
            val surface = codec.createInputSurface()
            codec.start()
            surface.release()
            "  result=CONFIGURE_START_OK codec=$codecName"
        } catch (e: Throwable) {
            "  result=FAILED codec=$codecName error=${e.javaClass.simpleName}: ${e.message}"
        } finally {
            try { codec?.stop() } catch (_: Throwable) {}
            try { codec?.release() } catch (_: Throwable) {}
        }
    }

    private fun cloneForEncoder(format: MediaFormat): MediaFormat {
        val mime = format.getString(MediaFormat.KEY_MIME) ?: return format

        return if (mime.startsWith("video/")) {
            val width = format.getInteger(MediaFormat.KEY_WIDTH)
            val height = format.getInteger(MediaFormat.KEY_HEIGHT)
            MediaFormat.createVideoFormat(mime, width, height).apply {
                setInteger(MediaFormat.KEY_BIT_RATE, 2_000_000)
                setInteger(MediaFormat.KEY_FRAME_RATE, 30)
                setInteger(MediaFormat.KEY_I_FRAME_INTERVAL, 1)
                setInteger(
                    MediaFormat.KEY_COLOR_FORMAT,
                    MediaCodecInfo.CodecCapabilities.COLOR_FormatSurface,
                )
            }
        } else if (mime.startsWith("audio/")) {
            val sampleRate = format.getInteger(MediaFormat.KEY_SAMPLE_RATE)
            val channels = format.getInteger(MediaFormat.KEY_CHANNEL_COUNT)
            MediaFormat.createAudioFormat(mime, sampleRate, channels).apply {
                setInteger(MediaFormat.KEY_BIT_RATE, 128_000)
            }
        } else {
            format
        }
    }

    private fun supportedFeatures(caps: MediaCodecInfo.CodecCapabilities): List<String> {
        val features = mutableListOf<String>()
        fun addIfSupported(name: String, minSdk: Int = 16) {
            if (Build.VERSION.SDK_INT >= minSdk) {
                val ok = safe { caps.isFeatureSupported(name) } ?: false
                if (ok) features += name
            }
        }
        addIfSupported(MediaCodecInfo.CodecCapabilities.FEATURE_AdaptivePlayback, 19)
        addIfSupported(MediaCodecInfo.CodecCapabilities.FEATURE_SecurePlayback, 19)
        addIfSupported(MediaCodecInfo.CodecCapabilities.FEATURE_TunneledPlayback, 21)
        addIfSupported(MediaCodecInfo.CodecCapabilities.FEATURE_IntraRefresh, 24)
        addIfSupported(MediaCodecInfo.CodecCapabilities.FEATURE_PartialFrame, 26)
        return features
    }

    private fun colorFormatName(value: Int): String {
        return constantName(MediaCodecInfo.CodecCapabilities::class.java, value, "COLOR_") ?: "UNKNOWN"
    }

    private fun constantName(clazz: Class<*>, value: Int, prefix: String?): String? {
        return try {
            clazz.declaredFields
                .filter { field ->
                    val m = field.modifiers
                    Modifier.isStatic(m) && Modifier.isPublic(m) && Modifier.isFinal(m)
                        && field.type == Int::class.javaPrimitiveType
                        && (prefix == null || field.name.startsWith(prefix))
                }
                .firstOrNull { field -> field.getInt(null) == value }
                ?.name
        } catch (_: Throwable) { null }
    }

    private inline fun <T> safe(block: () -> T): T? {
        return try { block() } catch (_: Throwable) { null }
    }

    private fun appendLine(sb: StringBuilder, text: String = "") {
        sb.append(text).append('\n')
    }

    private data class EncoderTest(
        val mime: String,
        val width: Int,
        val height: Int,
        val fps: Int,
        val bitrate: Int,
    )
}
```

---

## 2. Slint: `bridge.slint` additions

**File:** `ui/bridge.slint` — add these 4 lines inside `export global Bridge {`, right before the closing `}` (before line 466):

```slint
    // ── Codec test (Phase 8 wiring) ──────────────────────────────────────
    in property <string> codec-test-log: "";
    in property <bool>   codec-test-running: false;
    callback run-codec-test();
    callback run-codec-dump-only();
    callback run-codec-smoke-only();
``` [2-cite-0](#2-cite-0) 

---

## 3. Slint: updated `codec_test_page.slint`

**File:** `ui/pages/codec_test_page.slint` — full replacement:

```slint
// codec_test_page.slint — H.264 / codec test panel.
//
// Reachable from FullSettingsPage's "H.264 encoder test" row, which sets
// `PanelBridge.push(Panel.codec-test)`.

import { ScrollView } from "std-widgets.slint";
import { Bridge, Panel } from "../bridge.slint";
import { PanelBridge } from "../state/panel_bridge.slint";
import { Theme } from "../theme.slint";
import { PrimaryButton, TextButton } from "../components/buttons.slint";
import { PanelHeader } from "../components/panel_chrome.slint";

export component CodecTestPage inherits Rectangle {
    width: 100%;
    height: 100%;
    background: Theme.surface-primary;

    VerticalLayout {
        // ── Header ────────────────────────────────────────────────────────
        PanelHeader {
            title: @tr("Codec test");
            close-clicked => { PanelBridge.pop(); }
        }

        // ── Body ──────────────────────────────────────────────────────────
        VerticalLayout {
            padding: Theme.padding-screen;
            spacing: Theme.spacing-default;

            // ── Buttons row ───────────────────────────────────────────────
            HorizontalLayout {
                spacing: Theme.spacing-default;

                PrimaryButton {
                    label: Bridge.codec-test-running
                        ? @tr("Running…")
                        : @tr("Run full codec test");
                    enabled: !Bridge.codec-test-running;
                    clicked => { Bridge.run-codec-test(); }
                    horizontal-stretch: 1;
                }

                PrimaryButton {
                    label: @tr("Dump codecs only");
                    enabled: !Bridge.codec-test-running;
                    clicked => { Bridge.run-codec-dump-only(); }
                    horizontal-stretch: 1;
                }
            }

            PrimaryButton {
                label: @tr("Smoke test encoders");
                enabled: !Bridge.codec-test-running;
                clicked => { Bridge.run-codec-smoke-only(); }
            }

            // ── Log output ────────────────────────────────────────────────
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
        }
    }
}
``` [2-cite-1](#2-cite-1) 

---

## 4. Rust: `src/jni_bridge/codec_test.rs`

**New file:** `src/jni_bridge/codec_test.rs`

This follows the same `load_app_class` + `call_static_method` pattern used by `src/secret/jni.rs`: [2-cite-2](#2-cite-2) 

```rust
//! JNI bridge — codec test upcalls into Kotlin CodecDump.

#[cfg(target_os = "android")]
use crate::jni_bridge::helpers::{load_app_class, vm};

#[cfg(target_os = "android")]
const CODEC_DUMP_CLASS: &str = "org/fcast/android/sender/codec/CodecDump";

#[cfg(target_os = "android")]
fn call_static_string_method(method_name: &str) -> Result<String, String> {
    let vm = vm();
    let mut env = vm.attach_current_thread().map_err(|e| format!("attach: {e}"))?;

    let class = load_app_class(&mut env, CODEC_DUMP_CLASS)
        .map_err(|e| format!("load_app_class({CODEC_DUMP_CLASS}): {e}"))?;

    let result = env
        .call_static_method(class, method_name, "()Ljava/lang/String;", &[])
        .map_err(|e| format!("call_static_method {method_name}: {e}"))?;

    let jstr = result
        .l()
        .map_err(|e| format!("{method_name} result not an object: {e}"))?;

    if jstr.is_null() {
        return Ok(String::new());
    }

    let jstring = jni::objects::JString::from(jstr);
    let rust_str = env
        .get_string(&jstring)
        .map_err(|e| format!("get_string: {e}"))?
        .to_string_lossy()
        .to_string();

    Ok(rust_str)
}

#[cfg(target_os = "android")]
pub fn run_codec_dump_all() -> Result<String, String> {
    call_static_string_method("dumpAllCodecsToLog")
}

#[cfg(target_os = "android")]
pub fn run_codec_quick_find() -> Result<String, String> {
    call_static_string_method("quickFindCodecsForFormats")
}

#[cfg(target_os = "android")]
pub fn run_codec_smoke_test() -> Result<String, String> {
    call_static_string_method("smokeTestVideoEncoders")
}

// ── Non-Android stubs ───────────────────────────────────────────────────

#[cfg(not(target_os = "android"))]
pub fn run_codec_dump_all() -> Result<String, String> {
    Ok("codec dump not available on this platform\n".into())
}

#[cfg(not(target_os = "android"))]
pub fn run_codec_quick_find() -> Result<String, String> {
    Ok("codec quick-find not available on this platform\n".into())
}

#[cfg(not(target_os = "android"))]
pub fn run_codec_smoke_test() -> Result<String, String> {
    Ok("codec smoke test not available on this platform\n".into())
}
```

---

## 5. Rust: register module in `src/jni_bridge/mod.rs`

**File:** `src/jni_bridge/mod.rs` — add one line:

```rust
//! Java <-> Rust shims, grouped by the Java class that calls them.
//!
//! See refactor step 07 for context.

pub mod camera;
pub mod codec_test;          // ← ADD THIS LINE
pub mod discovery;
pub mod gstpop_bridge;
pub mod helpers;
pub mod main_activity;
pub mod migration_bridge;
``` [2-cite-3](#2-cite-3) 

---

## 6. Rust: register Bridge callbacks in `src/android_main.rs`

**File:** `src/android_main.rs` — insert this block after the existing `on_pick_test_overlay_image` handler (after line 1199), before the `use crate::jni_bridge::camera::` block:

```rust
    // ── Codec test callbacks ─────────────────────────────────────────────
    ui.global::<Bridge>().on_run_codec_test({
        let ui_weak = ui.as_weak();
        move || {
            let _ = ui_weak.upgrade_in_event_loop(|ui| {
                ui.global::<Bridge>().set_codec_test_running(true);
                ui.global::<Bridge>()
                    .set_codec_test_log("Running full codec test…\n".into());
            });
            let ui_inner = ui_weak.clone();
            std::thread::spawn(move || {
                let mut report = String::new();

                report.push_str("===== FULL CODEC DUMP =====\n");
                match crate::jni_bridge::codec_test::run_codec_dump_all() {
                    Ok(r) => report.push_str(&r),
                    Err(e) => report.push_str(&format!("FAIL dump: {e}\n")),
                }

                report.push_str("\n===== QUICK FIND =====\n");
                match crate::jni_bridge::codec_test::run_codec_quick_find() {
                    Ok(r) => report.push_str(&r),
                    Err(e) => report.push_str(&format!("FAIL quick-find: {e}\n")),
                }

                report.push_str("\n===== ENCODER SMOKE TEST =====\n");
                match crate::jni_bridge::codec_test::run_codec_smoke_test() {
                    Ok(r) => report.push_str(&r),
                    Err(e) => report.push_str(&format!("FAIL smoke: {e}\n")),
                }

                let _ = ui_inner.upgrade_in_event_loop(move |ui| {
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
                ui.global::<Bridge>()
                    .set_codec_test_log("Dumping codecs…\n".into());
            });
            let ui_inner = ui_weak.clone();
            std::thread::spawn(move || {
                let report = match crate::jni_bridge::codec_test::run_codec_dump_all() {
                    Ok(r) => r,
                    Err(e) => format!("FAIL: {e}\n"),
                };
                let _ = ui_inner.upgrade_in_event_loop(move |ui| {
                    ui.global::<Bridge>().set_codec_test_log(report.into());
                    ui.global::<Bridge>().set_codec_test_running(false);
                });
            });
        }
    });

    ui.global::<Bridge>().on_run_codec_smoke_only({
        let ui_weak = ui.as_weak();
        move || {
            let _ = ui_weak.upgrade_in_event_loop(|ui| {
                ui.global::<Bridge>().set_codec_test_running(true);
                ui.global::<Bridge>()
                    .set_codec_test_log("Running encoder smoke test…\n".into());
            });
            let ui_inner = ui_weak.clone();
            std::thread::spawn(move || {
                let report = match crate::jni_bridge::codec_test::run_codec_smoke_test() {
                    Ok(r) => r,
                    Err(e) => format!("FAIL: {e}\n"),
                };
                let _ = ui_inner.upgrade_in_event_loop(move |ui| {
                    ui.global::<Bridge>().set_codec_test_log(report.into());
                    ui.global::<Bridge>().set_codec_test_running(false);
                });
            });
        }
    });
``` [2-cite-4](#2-cite-4) 

Key design choices:

- Uses `std::thread::spawn` (not `tokio::spawn`) because the JNI `attach_current_thread` + `call_static_method` calls are blocking. This matches how the codebase handles blocking JNI work elsewhere. [2-cite-5](#2-cite-5) 

- Uses `load_app_class` (not `env.find_class`) so it works from native-spawned threads where the bootstrap classloader can't see app dex classes. [2-cite-6](#2-cite-6) 

- The `@JvmStatic` annotation on each Kotlin method ensures JNI sees them as real static methods on the class (not on a `Companion` inner class), so `call_static_method(class, "dumpAllCodecsToLog", ...)` works directly.

---

## Summary of files to touch

| Action | File |
|--------|------|
| **New** | `app/src/main/java/org/fcast/android/sender/codec/CodecDump.kt` |
| **New** | `src/jni_bridge/codec_test.rs` |
| **Edit** | `src/jni_bridge/mod.rs` — add `pub mod codec_test;` |
| **Edit** | `ui/bridge.slint` — add 5 properties/callbacks before closing `}` |
| **Replace** | `ui/pages/codec_test_page.slint` — full replacement |
| **Edit** | `src/android_main.rs` — insert 3 callback handlers after line ~1199 |
