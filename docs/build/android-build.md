# Android build guide

Single source of truth for building and deploying the app. The README links here
instead of duplicating build steps. **All version numbers are taken from
`flake.nix`, which is authoritative** — see the drift guard below.

## Toolchain (authoritative versions)

| Tool | Version | Source of truth |
|------|---------|-----------------|
| Rust | stable + Android targets | `rust-toolchain` / flake |
| Android SDK | API 34 (target), min 26 | `Cargo.toml` `[package.metadata.android.sdk]` |
| Android NDK | **r28c** = `28.0.13004108` | `flake.nix` `ndkVersion` |
| GStreamer Android SDK | 1.28.0 | `build.rs` env contract |
| Java | 21 (Temurin) | flake / Gradle |
| Nix (optional) | latest | `flake.nix` |

> **Drift guard.** The README once said *NDK r25c* while `flake.nix` pinned r28c.
> The number now lives in exactly one place (`flake.nix`); `check-repo-layout.sh`
> fails CI if the README disagrees. When bumping the NDK, change `flake.nix` and
> the single README reference together.

## Build targets

The crate is a `cdylib` named `fcastsender` (produces `libfcastsender.so`). The
declared Android targets are:

```text
aarch64-linux-android      # CI-validated, primary
armv7-linux-androideabi
x86_64-linux-android
i686-linux-android
```

CI validates only `aarch64-linux-android` for speed; the others are for local and
release builds.

## Option A — Nix dev shell (recommended)

```bash
# Full Android shell: SDK + NDK + cargo-ndk + adb
nix develop .#android -L

# Confirm a device is attached and authorized (status must be `device`)
adb devices

# Build, install on the connected device, and launch
./scripts/build-deploy.sh

# Variants
./scripts/build-deploy.sh --release      # release build
./scripts/build-deploy.sh --no-install   # build only, skip adb install
./scripts/build-deploy.sh --clean        # force-clean generated outputs first
```

A lightweight shell (`nix develop`, no Android SDK) is enough for Rust/UI checks.

## Option B — manual environment

`build.rs` is a no-op on non-Android targets. For Android targets, export the
toolchain locations first:

```bash
export ANDROID_HOME=/path/to/Android/Sdk
export ANDROID_SDK_ROOT="$ANDROID_HOME"
export ANDROID_NDK_ROOT=/path/to/android-ndk-r28c     # 28.0.13004108
export ANDROID_NDK_HOME="$ANDROID_NDK_ROOT"
export GSTREAMER_ROOT_ANDROID=/path/to/gstreamer-1.0-android-universal-1.28.0
```

Then build the native library and/or the APK:

```bash
# Rust native library
cargo check --target aarch64-linux-android
cargo build --release --target aarch64-linux-android

# Full APK via Gradle
./gradlew assembleDebug
./gradlew installDebug
```

## UI preview without a device

```bash
# Verify your slint-viewer matches the pinned Slint version (1.16.0)
bash scripts/check-slint-viewer.sh

# Preview the whole app
nix-shell -p slint-viewer --run "slint-viewer ui/main.slint --auto-reload"

# Preview a single page in isolation
slint-viewer ui/pages/media_backend_page.slint --component MediaBackendPage
```

UI validation and headless snapshot tests:

```bash
ci/ui-validate.sh
cargo test --test ui_snapshots
# Refresh accessibility golden files if they legitimately changed:
UI_SNAPSHOT_REFRESH=1 cargo test --test ui_snapshots
```

## Debug logs

```bash
adb logcat -s fcastsender RustStdoutStderr   # app-focused
adb logcat | grep -i fcast                   # broader filter
```

## Troubleshooting

### Device not detected / `unauthorized`

```bash
adb kill-server
adb start-server
adb devices        # accept the RSA prompt on the device; status must be `device`
```

Ensure the phone is unlocked and USB mode is file-transfer/PTP (not charge-only).
If needed, revoke USB-debugging authorizations on the device and reconnect.

### Gradle cannot find the SDK

```bash
echo "$ANDROID_HOME"
echo "$ANDROID_SDK_ROOT"
ls "$ANDROID_HOME/platforms"
```

### Native library packaging problem

```bash
# Confirm the built .so landed where Gradle packages it
find app -name 'libfcastsender.so' -print
find app -name 'libgstreamer_android.so' -print
```

### NDK version mismatch

If `cargo build` complains about the NDK, confirm `ANDROID_NDK_ROOT` points at
`28.0.13004108` (r28c) — the version pinned in `flake.nix`.
