# fcast-android-sender

[![Android Debug APK](https://github.com/kodyka/fcast-android-sender/actions/workflows/android-debug-apk.yml/badge.svg)](https://github.com/kodyka/fcast-android-sender/actions/workflows/android-debug-apk.yml)
[![Android Release APK](https://github.com/kodyka/fcast-android-sender/actions/workflows/android-release-apk.yml/badge.svg)](https://github.com/kodyka/fcast-android-sender/actions/workflows/android-release-apk.yml)
[![UI Lint](https://github.com/kodyka/fcast-android-sender/actions/workflows/ui-lint.yml/badge.svg)](https://github.com/kodyka/fcast-android-sender/actions/workflows/ui-lint.yml)

Standalone Android sender app for the [FCast protocol](https://github.com/kodyka/fcast).
A thin Kotlin/Android shell hosts a Rust core (`fcastsender`, a `cdylib`) that
drives a Slint UI and a GStreamer/WebRTC media pipeline over a JNI boundary:
screen- or camera-captured video is encoded via GStreamer and streamed to FCast
receivers over WHEP/WebRTC (or RTMP).

Extracted from `kodyka/fcast` at commit `63980e6736e65adbd15588d21903d0c02223c15c`
via MVP phase 10.

## Architecture

See [docs/architecture/overview.md](docs/architecture/overview.md) for the layered
diagram and [ARCHITECTURE.md](ARCHITECTURE.md) for the full Mermaid graph. In
short: Android shell (Kotlin/Java) → JNI boundary (`src/lib.rs`) → Rust core
(`app.rs`, backend registry) → workspace crates (`gstpop-runtime`,
`migration-runtime`) → GStreamer/WHEP → FCast receiver, with a Slint UI bound via
the `Bridge` global.

## Building

The Android build still expects the same toolchain components the monorepo used:

- Rust toolchain with Android targets
- Android SDK command-line tools
- Android NDK r28c (`28.0.13004108`, pinned in `flake.nix`)
- GStreamer Android SDK 1.28.0
- Java 21

> The NDK version is single-sourced from `flake.nix`. `scripts/check-repo-layout.sh`
> fails CI if this README and `flake.nix` disagree.

## Flake-based local setup (recommended)

This repo ships two Nix dev shells:

- `nix develop` — lightweight shell for Rust/UI checks
- `nix develop .#android` — full Android shell (SDK + NDK + cargo-ndk + adb)

For USB device deploy/debug, always use the Android shell.

### 1. Enter Android dev shell

```console
$ nix develop .#android -L
```

On first run, Nix may download large Android/JDK artifacts. This is expected.

### 2. Prepare your phone

1. Enable Developer options
2. Enable USB debugging
3. Connect via USB
4. Accept the RSA fingerprint prompt on device

Verify:

```console
$ adb devices
```

You should see one device with `device` status (not `unauthorized`).

### 3. Build, install, launch

```console
$ ./scripts/build-deploy.sh
```

Release build:

```console
$ ./scripts/build-deploy.sh --release
```

Build only (no install):

```console
$ ./scripts/build-deploy.sh --no-install
```

### 4. Debug logs

App-focused logs:

```console
$ adb logcat -s fcastsender RustStdoutStderr
```

Broader filter:

```console
$ adb logcat | grep -i fcast
```

### 5. If USB install fails

- Ensure phone is unlocked and USB mode is file transfer/PTP (not charge-only)
- Re-run:
  - `adb kill-server`
  - `adb start-server`
  - `adb devices`
- If needed, revoke USB debugging authorizations on device, reconnect, accept prompt again

### Required environment

`build.rs` is a no-op on non-Android targets. On Android targets
(`aarch64-linux-android`, `armv7-linux-androideabi`, `x86_64-linux-android`,
`i686-linux-android`), export:

| Variable | Purpose |
| --- | --- |
| `ANDROID_NDK_ROOT` or `ANDROID_NDK_HOME` | Path to Android NDK r28c (`28.0.13004108`) |
| `GSTREAMER_ROOT_ANDROID` | Path to GStreamer Android SDK 1.28.0 |

For Gradle builds, also export:

| Variable | Purpose |
| --- | --- |
| `ANDROID_HOME` | Path to Android SDK |
| `ANDROID_SDK_ROOT` | Alias for `ANDROID_HOME` |

### Cargo

```console
$ cargo check --target aarch64-linux-android
$ cargo build --release --target aarch64-linux-android
```

### Gradle

```console
$ ./gradlew assembleDebug
$ ./gradlew installDebug
```

CI intentionally validates the `aarch64-linux-android` path only for speed.
The crate metadata still lists the other Android targets for local and release
builds.

## Repository layout

| Path | Description |
|------|-------------|
| `src/` | Main Rust crate `android-sender` — JNI exports, app logic, backends |
| `crates/` | Internal Rust runtimes: `migration-runtime`, `gstpop-runtime` |
| `ui/` | Slint UI tree: `main.slint`, `bridge.slint`, pages, components, theme, i18n |
| `app/` | Android Gradle module: Kotlin/Java shell, JNI glue, manifest, resources |
| `ci/` | CI build + UI validation scripts, JNI symbol baseline |
| `scripts/` | Developer helpers (`build-deploy`, smoke tests, slint-viewer checks) |
| `tests/` | Headless UI snapshot tests |
| `vendor/` | Vendored `gstpop` crate |
| `docs/` | Documentation — start at [docs/README.md](docs/README.md) |

## SDK dependencies

The crate depends on three SDK crates that remain in the FCast monorepo:

- `fcast-protocol`
- `fcast-sender-sdk`
- `mcore`

They are pinned as Git dependencies to `kodyka/fcast` commit
`63980e6736e65adbd15588d21903d0c02223c15c`.

To bump the SDK pin, see [docs/cross-repo-sync.md](docs/cross-repo-sync.md).

## UI development

For designing and testing the Slint UI without booting an Android emulator or physical device, you can run and preview the UI components locally.

### 1. Verify `slint-viewer` Version
To prevent confusing errors due to version mismatch, ensure your local `slint-viewer` version matches the Slint dependency pinned in `Cargo.toml` (`1.16.0`).

You can verify this using the provided script:
```console
$ bash scripts/check-slint-viewer.sh
```

### 2. Previewing the UI via `nix-shell` (Recommended)
You can run `slint-viewer` in an isolated environment without globally installing it:

```console
# Preview the whole app (MainWindow)
$ nix-shell -p slint-viewer --run "slint-viewer ui/main.slint --auto-reload"

# Preview a single page in isolation (e.g. MediaBackendPage)
$ nix-shell -p slint-viewer --run "slint-viewer ui/pages/media_backend_page.slint --component MediaBackendPage"
```

### 3. Alternative Local Cargo Installation
If you prefer to install it globally via Cargo, run:
```console
$ cargo install slint-viewer --version "=1.16.0" --force
```
Then launch it using:
```console
$ slint-viewer ui/main.slint --auto-reload
```

### 4. Running UI Validation and Headless Tests
Run the automated UI validation suite (which checks for raw hex colors, hardcoded sizes, nested layout issues, etc.) and headless snapshot tests:

```console
# Run local pre-commit checks and validation scripts
$ ci/ui-validate.sh

# Run headless UI tests (using i-slint-backend-testing)
$ cargo test --test ui_snapshots
```

To refresh accessibility golden files if they legitimately changed:
```console
$ UI_SNAPSHOT_REFRESH=1 cargo test --test ui_snapshots
```

## CI/CD

| Workflow | Trigger | Purpose |
|----------|---------|---------|
| `android-debug-apk` | push, PR | UI audit + arm64 debug APK + JVM unit tests |
| `android-release-apk` | release published, manual | Signed release APK attached to the GitHub Release |
| `android-instrumented-tests` | PR (`app/src` changes), nightly | Emulator-based instrumented tests |
| `symbol-stability` | PR (`src`/`crates`/`app` changes) | JNI symbol baseline diff |
| `ui-lint` | PR (`ui/` changes) | Forbid raw hex colors, hardcoded sizes, direct state writes |
| `slint-viewer-smoke` | PR (`ui/` changes) | Compile `ui/main.slint` with `slint-viewer` |
| `gstpop-smoke` | push, PR (`src`/`ui`/`ci` changes) | gst-pop backend integration tests |
| `repo-health` | push, PR (docs/scripts/README) | Layout drift + shellcheck + link check |

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for setup, hooks, branch/PR conventions,
and the test matrix. Security reports: [SECURITY.md](SECURITY.md).

## License

MIT
