#!/usr/bin/env bash
# build-deploy.sh — Build the Android APK and install on a USB-connected device.
#
# Usage:
#   ./scripts/build-deploy.sh              # debug build
#   ./scripts/build-deploy.sh --release    # release build
#   ./scripts/build-deploy.sh --no-install # build only, skip adb install
#   ./scripts/build-deploy.sh --clean      # force cleanup of generated build outputs first
#   ./scripts/build-deploy.sh --no-clean   # skip automatic low-disk cleanup
#
# Automatic cleanup runs when free disk space is below BUILD_DEPLOY_MIN_FREE_GB
# (default: 10 GiB). It removes only generated build outputs.
#
# Prerequisites: run inside `nix develop .#android`.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

# ── Parse args ────────────────────────────────────────────────────────
BUILD_TYPE="debug"
GRADLE_TASK=":app:assembleDebug"
DO_INSTALL=true
CLEAN_MODE="auto"
MIN_FREE_GB="${BUILD_DEPLOY_MIN_FREE_GB:-10}"

for arg in "$@"; do
  case "$arg" in
    --release)
      BUILD_TYPE="release"
      GRADLE_TASK=":app:assembleRelease"
      ;;
    --no-install)
      DO_INSTALL=false
      ;;
    --clean)
      CLEAN_MODE="always"
      ;;
    --no-clean)
      CLEAN_MODE="never"
      ;;
    --help|-h)
      sed -n '2,15p' "$0"
      exit 0
      ;;
    *)
      echo "Unknown argument: $arg" >&2
      echo "Run with --help for usage." >&2
      exit 2
      ;;
  esac
done

# ── Preflight checks ─────────────────────────────────────────────────
fail() { echo "✗ $1" >&2; exit 1; }

case "$MIN_FREE_GB" in
  ''|*[!0-9]*) fail "BUILD_DEPLOY_MIN_FREE_GB must be a whole number of GiB, got: $MIN_FREE_GB" ;;
esac

free_space_gb() {
  df -Pk "$PROJECT_ROOT" | awk 'NR == 2 { printf "%d", $4 / 1024 / 1024 }'
}

remove_generated_path() {
  local path="$1"
  [ -e "$path" ] || return 0

  local size
  size=$(du -sh "$path" 2>/dev/null | awk '{ print $1 }' || true)
  echo "  rm -rf ${path#$PROJECT_ROOT/}${size:+ ($size)}"
  rm -rf "$path"
}

cleanup_generated_outputs() {
  echo "▸ Cleaning generated build outputs to free disk space..."
  remove_generated_path "$PROJECT_ROOT/app/build"
  remove_generated_path "$PROJECT_ROOT/app/obj"
  remove_generated_path "$PROJECT_ROOT/app/libs"
  remove_generated_path "$PROJECT_ROOT/app/.cxx"
  remove_generated_path "$PROJECT_ROOT/app/src/main/jniLibs"
  remove_generated_path "$PROJECT_ROOT/build"
  remove_generated_path "$PROJECT_ROOT/target"
  remove_generated_path "$PROJECT_ROOT/.gradle"
  remove_generated_path "$PROJECT_ROOT/.kotlin"

  local after_gb
  after_gb=$(free_space_gb)
  echo "  Free space after cleanup: ${after_gb} GiB"
}

[ -n "${ANDROID_HOME:-}" ]              || fail "ANDROID_HOME not set. Run inside: nix develop .#android"
[ -n "${ANDROID_NDK_ROOT:-}" ]          || fail "ANDROID_NDK_ROOT not set. Run inside: nix develop .#android"
[ -n "${GSTREAMER_ROOT_ANDROID:-}" ]    || fail "GSTREAMER_ROOT_ANDROID not set. Run inside: nix develop .#android"
[ -d "$GSTREAMER_ROOT_ANDROID/arm64" ]  || fail "GStreamer Android binaries not found at $GSTREAMER_ROOT_ANDROID/arm64"
command -v cargo-ndk >/dev/null         || fail "cargo-ndk not found"
command -v adb >/dev/null               || fail "adb not found"

echo "┌──────────────────────────────────────────────────────┐"
echo "│  Building fcast-android-sender ($BUILD_TYPE)              │"
echo "└──────────────────────────────────────────────────────┘"

FREE_GB=$(free_space_gb)
if [ "$CLEAN_MODE" = "always" ]; then
  cleanup_generated_outputs
elif [ "$CLEAN_MODE" = "auto" ] && [ "$FREE_GB" -lt "$MIN_FREE_GB" ]; then
  echo "⚠ Low disk space detected: ${FREE_GB} GiB free; threshold is ${MIN_FREE_GB} GiB."
  cleanup_generated_outputs
elif [ "$CLEAN_MODE" = "auto" ]; then
  echo "▸ Disk space OK: ${FREE_GB} GiB free (cleanup threshold: ${MIN_FREE_GB} GiB)"
else
  echo "▸ Skipping build-output cleanup (--no-clean). Free space: ${FREE_GB} GiB"
fi

# ── Step 1: Build Rust native library with cargo-ndk ──────────────────
# Delegate to the CI script so local and CI stay in lock-step. It pins
# ANDROID_PLATFORM/ANDROID_JAR which build scripts (e.g. Slint's
# i-slint-backend-android-activity) require to locate android.jar.
echo ""
echo "▸ Step 1/3: cargo ndk (Rust → libfcastsender.so)"
bash "$PROJECT_ROOT/ci/build-rust-android-lib.sh" "$BUILD_TYPE"

# ── Step 2: Build GStreamer JNI via ndk-build ─────────────────────────
echo ""
echo "▸ Step 2/3: ndk-build (GStreamer → libgstreamer_android.so)"
"$ANDROID_NDK_ROOT/ndk-build" \
  NDK_PROJECT_PATH="$PROJECT_ROOT/app" \
  APP_BUILD_SCRIPT="$PROJECT_ROOT/app/jni/Android.mk" \
  NDK_APPLICATION_MK="$PROJECT_ROOT/app/jni/Application.mk" \
  GSTREAMER_ROOT_ANDROID="$GSTREAMER_ROOT_ANDROID" \
  APP_ABI=arm64-v8a \
  -j"$(sysctl -n hw.ncpu 2>/dev/null || nproc 2>/dev/null || echo 4)"

# No copy needed: app/build.gradle declares jniLibs.srcDirs = ['libs', 'src/main/jniLibs'],
# so Gradle merges ndk-build's app/libs/ and cargo-ndk's app/src/main/jniLibs/ directly.
# Copying would duplicate libgstreamer_android.so etc. across both dirs and fail
# :app:mergeDebugJniLibFolders with "Duplicate resources".

# ── Step 3: Gradle APK build ─────────────────────────────────────────
echo ""
echo "▸ Step 3/3: Gradle ($GRADLE_TASK)"
./gradlew $GRADLE_TASK

# ── Install + Launch ──────────────────────────────────────────────────
APK_DIR="app/build/outputs/apk/$BUILD_TYPE"
APK=$(find "$APK_DIR" -name "*.apk" -type f | head -1)

if [ -z "$APK" ]; then
  fail "No APK found in $APK_DIR"
fi

echo ""
echo "✓ APK built: $APK"

if [ "$DO_INSTALL" = true ]; then
  # Check device connected
  DEVICE_COUNT=$(adb devices | grep -c 'device$' || true)
  if [ "$DEVICE_COUNT" -eq 0 ]; then
    echo ""
    echo "⚠ No USB device detected. Connect your phone and enable USB debugging."
    echo "  Then run: adb install -r $APK"
    exit 0
  fi

  echo "▸ Installing on device..."
  adb install -r "$APK"

  echo "▸ Launching..."
  adb shell am start -n org.fcast.android.sender/.MainActivity

  echo ""
  echo "┌──────────────────────────────────────────────────────┐"
  echo "│  ✓ App installed and launched!                       │"
  echo "│                                                      │"
  echo "│  View logs:                                          │"
  echo "│    adb logcat -s fcastsender RustStdoutStderr        │"
  echo "│                                                      │"
  echo "│  Full logcat:                                        │"
  echo "│    adb logcat | grep -i fcast                        │"
  echo "└──────────────────────────────────────────────────────┘"
else
  echo ""
  echo "Install manually: adb install -r $APK"
fi
