# Step 1 — Kotlin `CodecDump.kt` (new)

← [Index](README.md) · Next → [Step 2](step-2-bridge-slint.md)

**File:** `app/src/main/java/org/fcast/android/sender/codec/CodecDump.kt`

Three `@JvmStatic` entry points, each returning a `String` report and also logging
to `CodecDump`. `@JvmStatic` is required so JNI sees them as real static methods on
the class (not on a `Companion`), letting Rust call `call_static_method(class, "…")`.

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

← [Index](README.md) · Next → [Step 2](step-2-bridge-slint.md)
