#!/usr/bin/env bash
#
# dump-frames.sh — pull & view the debug frame dumps from the FCast sender.
#
# The app writes raw I420 frames to <files>/dump/ when the marker file
# <files>/dump/on exists (toggled here). Two capture points:
#   cam_WxH_NNNNN.i420   raw camera → GL → native, BEFORE videocrop/flip
#   pipe_WxH_NNNNN.i420   encoder input, AFTER videocrop/flip
# Comparing them localises artefacts (e.g. a green line) to a pipeline stage.
#
# Usage:
#   scripts/dump-frames.sh on        enable dumps + clear logcat
#   scripts/dump-frames.sh off       disable dumps (remove marker)
#   scripts/dump-frames.sh status    list the on-device dump dir
#   scripts/dump-frames.sh pull      copy *.i420 to the local out dir
#   scripts/dump-frames.sh convert   turn local *.i420 into *.png
#   scripts/dump-frames.sh grab      pull + convert (the usual one)
#   scripts/dump-frames.sh log       print camera/crop geometry log lines
#   scripts/dump-frames.sh clean     wipe on-device dumps + local out dir
#
# Env overrides:
#   ADB   path to adb (auto-detected; falls back to the nix store SDK)
#   PKG   app package id  (default: org.fcast.android.sender)
#   OUT   local output dir (default: /tmp/fcastdump)
#
set -euo pipefail

PKG="${PKG:-org.fcast.android.sender}"
OUT="${OUT:-/tmp/fcastdump}"
DEVICE_DIR="files/dump"   # relative to the app data dir (resolved by run-as)

# ── locate adb ─────────────────────────────────────────────────────────────
find_adb() {
  if [ -n "${ADB:-}" ] && [ -x "${ADB}" ]; then echo "$ADB"; return; fi
  if command -v adb >/dev/null 2>&1; then command -v adb; return; fi
  for c in \
    "${ANDROID_HOME:-}/platform-tools/adb" \
    "$HOME/Library/Android/sdk/platform-tools/adb" \
    /nix/store/*-androidsdk/libexec/android-sdk/platform-tools/adb; do
    [ -x "$c" ] && { echo "$c"; return; }
  done
  echo "adb"  # last resort; will error clearly if missing
}
ADB="$(find_adb)"

run_as() { "$ADB" shell run-as "$PKG" "$@"; }

require_device() {
  if ! "$ADB" get-state >/dev/null 2>&1; then
    echo "✗ no device (adb: $ADB). Connect/authorise the phone." >&2
    exit 1
  fi
}

# ── subcommands ────────────────────────────────────────────────────────────
cmd_on() {
  require_device
  run_as mkdir -p "$DEVICE_DIR"
  run_as touch "$DEVICE_DIR/on"
  "$ADB" logcat -c || true
  echo "✓ dumps ENABLED (marker $DEVICE_DIR/on) and logcat cleared."
  echo "  → now go live (SRT) on the phone for ~10s, then: $0 grab"
}

cmd_off() {
  require_device
  run_as rm -f "$DEVICE_DIR/on" || true
  echo "✓ dumps DISABLED (marker removed)."
}

cmd_status() {
  require_device
  echo "device $PKG:$DEVICE_DIR —"
  run_as ls -l "$DEVICE_DIR" 2>/dev/null || echo "  (empty or missing)"
}

cmd_pull() {
  require_device
  mkdir -p "$OUT"
  local files
  files="$(run_as ls -1 "$DEVICE_DIR" 2>/dev/null | tr -d '\r' | grep '\.i420$' || true)"
  if [ -z "$files" ]; then
    echo "✗ no *.i420 on device. Did you enable dumps ($0 on) and go live?" >&2
    echo "  Tip: the app must (re)start after enabling. Try:" >&2
    echo "    $ADB shell am force-stop $PKG && $0 on  # then relaunch + go live" >&2
    exit 1
  fi
  local n=0
  while IFS= read -r f; do
    [ -z "$f" ] && continue
    # exec-out gives a binary-clean stream (no CRLF translation).
    "$ADB" exec-out run-as "$PKG" cat "$DEVICE_DIR/$f" > "$OUT/$f"
    n=$((n+1))
  done <<< "$files"
  echo "✓ pulled $n file(s) → $OUT"
  ls -l "$OUT"/*.i420
}

# Pick a converter: gst-launch (in the nix dev shell) or ffmpeg.
convert_one() {
  local f="$1" w="$2" h="$3" png="${1%.i420}.png"
  if command -v gst-launch-1.0 >/dev/null 2>&1; then
    gst-launch-1.0 -q \
      filesrc location="$f" \
      ! rawvideoparse width="$w" height="$h" format=i420 \
      ! videoconvert ! pngenc ! filesink location="$png" >/dev/null
  elif command -v ffmpeg >/dev/null 2>&1; then
    ffmpeg -hide_banner -loglevel error -y \
      -f rawvideo -pix_fmt yuv420p -s "${w}x${h}" -i "$f" -frames:v 1 "$png"
  else
    echo "✗ need gst-launch-1.0 or ffmpeg to convert." >&2
    echo "  Run inside the dev shell:  nix develop .#android -c $0 convert" >&2
    return 1
  fi
  echo "  $png  (${w}x${h})"
}

cmd_convert() {
  shopt -s nullglob
  local any=0
  for f in "$OUT"/*.i420; do
    any=1
    # filename: tag_WxH_NNNNN.i420  → extract W and H
    local base wh w h
    base="$(basename "$f")"
    wh="$(printf '%s' "$base" | sed -nE 's/.*_([0-9]+)x([0-9]+)_.*/\1 \2/p')"
    if [ -z "$wh" ]; then echo "  skip $base (no WxH in name)"; continue; fi
    w="${wh% *}"; h="${wh#* }"
    convert_one "$f" "$w" "$h"
  done
  [ "$any" = 1 ] || { echo "✗ no *.i420 in $OUT — run '$0 pull' first." >&2; exit 1; }
  echo "✓ PNGs written to $OUT — open cam_* (raw camera) and pipe_* (encoder input)."
}

cmd_grab() { cmd_pull; cmd_convert; }

cmd_log() {
  require_device
  echo "── camera geometry / crop / frame logs ──"
  "$ADB" logcat -d 2>&1 \
    | grep -iE "camera pipeline built|set_crop applied|process_frame: count|dumped (camera|pipeline) frame|Selected Android H.264" \
    || echo "  (nothing yet — go live, then re-run)"
}

cmd_clean() {
  require_device
  run_as sh -c "rm -f $DEVICE_DIR/*.i420" 2>/dev/null || true
  rm -f "$OUT"/*.i420 "$OUT"/*.png 2>/dev/null || true
  echo "✓ cleared on-device *.i420 and local $OUT."
}

# ── dispatch ───────────────────────────────────────────────────────────────
case "${1:-}" in
  on)       cmd_on ;;
  off)      cmd_off ;;
  status)   cmd_status ;;
  pull)     cmd_pull ;;
  convert)  cmd_convert ;;
  grab)     cmd_grab ;;
  log)      cmd_log ;;
  clean)    cmd_clean ;;
  *)
    sed -n '2,30p' "$0" | sed 's/^# \{0,1\}//'
    exit 1
    ;;
esac
