#!/usr/bin/env bash
# slint-smoke.sh — Local equivalent of .github/workflows/slint-viewer-smoke.yml
#
# Downloads the exact slint-viewer binary that matches the Slint version
# pinned in Cargo.toml (same logic as CI), caches it in .slint-viewer-cache/,
# then smoke-compiles ui/main.slint.
#
# Exit codes:
#   0  — compiled OK
#   1  — compile error (slint-viewer exited 255)
#
# Usage (from repo root, inside nix develop):
#   bash scripts/slint-smoke.sh

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

# ── 1. Read pinned Slint version ─────────────────────────────────────────────
VERSION=$(awk -F'"' '/^slint = / { print $2 }' Cargo.toml | head -1)
if [ -z "$VERSION" ]; then
    echo "ERROR: could not parse slint version from Cargo.toml" >&2
    exit 1
fi
echo "Pinned slint version: $VERSION"

# ── 2. Resolve platform archive name ─────────────────────────────────────────
case "$(uname -s)" in
    Darwin) ARCHIVE="slint-viewer-macos.tar.gz" ;;
    Linux)  ARCHIVE="slint-viewer-linux.tar.gz" ;;
    *)      echo "Unsupported platform: $(uname -s)" >&2; exit 1 ;;
esac

# ── 3. Download (cached) ──────────────────────────────────────────────────────
CACHE_DIR="$ROOT/.slint-viewer-cache/$VERSION"
VIEWER="$CACHE_DIR/slint-viewer/slint-viewer"

if [ ! -x "$VIEWER" ]; then
    mkdir -p "$CACHE_DIR"
    URL="https://github.com/slint-ui/slint/releases/download/v${VERSION}/${ARCHIVE}"
    echo "Downloading $URL"
    curl -fL --retry 5 --retry-all-errors --connect-timeout 30 \
        -o "$CACHE_DIR/slint-viewer.tar.gz" "$URL"
    tar -xzf "$CACHE_DIR/slint-viewer.tar.gz" -C "$CACHE_DIR"
    chmod +x "$VIEWER"
    rm -f "$CACHE_DIR/slint-viewer.tar.gz"
fi

echo "Using $("$VIEWER" --version 2>&1 | head -1)"

# ── 4. Smoke-compile ui/main.slint ────────────────────────────────────────────
# Exit-code map (mirrors CI):
#   255 (-1)  → .slint compile error   → FAIL
#   124       → timeout reached        → PASS (viewer opened a window)
#   137       → SIGKILL after timeout  → PASS
#   0 / 1     → clean close            → PASS
EXIT=0
timeout --kill-after=2 8 "$VIEWER" ui/main.slint 2>&1 || EXIT=$?

case "$EXIT" in
    255)
        echo ""
        echo "FAIL: ui/main.slint failed to compile (slint-viewer exit -1)"
        exit 1
        ;;
    0|1|124|137|143)
        echo "PASS: ui/main.slint compiled successfully (exit=$EXIT)"
        ;;
    *)
        echo "UNEXPECTED exit code: $EXIT" >&2
        exit 1
        ;;
esac
