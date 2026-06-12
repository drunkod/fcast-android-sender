# Step 01 — Dependencies + manifest check

**Master plan:** §3, §4 · **Phase:** 1 · **Depends on:** nothing · **Lang:** Gradle

## Goal

Add `streampack-core` + `streampack-srt` (3.1.1) to the build via the version catalog,
without changing the legacy build behaviour. Confirm the manifest already has what we
need (it does).

## Files touched

- `gradle/libs.versions.toml` — add version + library coordinates
- `app/build.gradle` (Groovy) — add two `implementation` lines
- `app/src/main/AndroidManifest.xml` — **no change** (verification only)

## Code

### `gradle/libs.versions.toml`

```toml
[versions]
# … existing …
streampack = "3.1.1"   # matches draft/StreamPack-boilerplate

[libraries]
# … existing …
streampack-core = { group = "io.github.thibaultbee.streampack", name = "streampack-core", version.ref = "streampack" }
streampack-srt  = { group = "io.github.thibaultbee.streampack", name = "streampack-srt",  version.ref = "streampack" }
# Optional, do NOT enable initially:
# streampack-ui   = { group = "io.github.thibaultbee.streampack", name = "streampack-ui",   version.ref = "streampack" }
# streampack-rtmp = { group = "io.github.thibaultbee.streampack", name = "streampack-rtmp", version.ref = "streampack" }
```

### `app/build.gradle` (Groovy)

```groovy
dependencies {
    implementation libs.material
    implementation 'androidx.security:security-crypto:1.1.0-alpha06'
    implementation libs.kotlin.stdlib
    implementation libs.kotlinx.coroutines.android
    implementation libs.androidx.activity.ktx
    implementation libs.androidx.lifecycle.runtime.ktx
    implementation libs.androidx.lifecycle.viewmodel.ktx
    implementation "com.journeyapps:zxing-android-embedded:4.3.0"

    // StreamPack — camera/encoder Surface pipeline.
    implementation libs.streampack.core
    // Phase 1 only: direct SRT egress for end-to-end validation.
    implementation libs.streampack.srt
    // (intentionally NOT streampack-ui — Slint owns the UI)

    // … existing test deps unchanged …
}
```

### Manifest — verify only, do not edit

Already present in `app/src/main/AndroidManifest.xml`:
`INTERNET`, `ACCESS_NETWORK_STATE`, `CAMERA`, `RECORD_AUDIO`, plus `FOREGROUND_SERVICE*`.
**Do not** touch the `android.app.lib_name = fcastsender` meta-data — it is how the
Slint/Rust NativeActivity host loads.

## Notes

- **minSdk:** ours is `26`; boilerplate is `24`. StreamPack 3.1.1 supports `minSdk 24` →
  no floor change.
- **ABI:** we ship `arm64-v8a` only. StreamPack's MediaCodec/Camera2 usage is
  Java/NDK-agnostic → no `Android.mk`/`abiFilters` change for Phase 1.
- **Repos:** `streampack-*` is on `mavenCentral()` (already in `settings.gradle`/catalog
  resolution). The boilerplate resolves it from `mavenCentral()`.

## How to verify

```
✅ Gradle sync succeeds.
✅ ./gradlew :app:assembleDebug builds.
✅ Existing unit + instrumentation builds still compile (no behaviour change yet).
✅ No streampack-ui / streampack-rtmp pulled in (check the resolved dependency tree).
```

## Risks

- A long-running background camera stream will later need a
  `foregroundServiceType="camera|microphone"` service — **out of scope** here (Phase 1
  is foreground-only).
