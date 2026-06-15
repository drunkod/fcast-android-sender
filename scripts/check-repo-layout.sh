#!/usr/bin/env bash
# check-repo-layout.sh — Repo-health guard for fcast-android-sender.
#
# Verifies that the repository layout has not drifted back into the pre-refresh
# state: required community files exist, stale/forbidden paths are gone, the
# README and flake agree on the NDK version, and no broken internal doc links
# were introduced.
#
# Usage:
#   ./scripts/check-repo-layout.sh           # report and exit non-zero on failure
#   ./scripts/check-repo-layout.sh --warn    # report only, always exit 0
#   ./scripts/check-repo-layout.sh --quiet   # only print failures
#
# Exit codes: 0 = all checks passed (or --warn), 1 = one or more checks failed.
#
# Designed to run both locally and in CI (.github/workflows/repo-health.yml).
# No external dependencies beyond coreutils + grep + find.

set -euo pipefail

# ── Resolve repo root (script lives in scripts/) ─────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${ROOT}"

WARN_ONLY=0
QUIET=0
for arg in "$@"; do
  case "${arg}" in
    --warn)  WARN_ONLY=1 ;;
    --quiet) QUIET=1 ;;
    -h|--help)
      sed -n '2,18p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *) echo "unknown arg: ${arg}" >&2; exit 2 ;;
  esac
done

FAILURES=0
PASSES=0

red()   { printf '\033[31m%s\033[0m' "$1"; }
green() { printf '\033[32m%s\033[0m' "$1"; }
yellow(){ printf '\033[33m%s\033[0m' "$1"; }

pass() {
  PASSES=$((PASSES + 1))
  [ "${QUIET}" -eq 1 ] || printf '  %s %s\n' "$(green PASS)" "$1"
}

fail() {
  FAILURES=$((FAILURES + 1))
  printf '  %s %s\n' "$(red FAIL)" "$1"
  [ -n "${2:-}" ] && printf '       %s\n' "$2"
}

section() { [ "${QUIET}" -eq 1 ] || printf '\n%s\n' "$1"; }

# ── 1. Required files must exist ─────────────────────────────────────────────
section "Required files"
REQUIRED_FILES=(
  "README.md"
  "LICENSE"
  ".gitignore"
  "CONTRIBUTING.md"
  "SECURITY.md"
  "CHANGELOG.md"
  "Cargo.toml"
  "settings.gradle"
  "flake.nix"
)
for f in "${REQUIRED_FILES[@]}"; do
  if [ -f "${f}" ]; then
    pass "exists: ${f}"
  else
    fail "missing required file: ${f}" "create it as part of the repo refresh"
  fi
done

# ── 2. Forbidden / stale paths must NOT exist at root ────────────────────────
section "Forbidden stale paths"
FORBIDDEN_PATHS=(
  "TODO.codecs"                       # → docs/plans/codecs/
  ".gitlab-ci.yml"                    # → ci/legacy-gitlab-ci.yml
  "draft/slint-ui/docs/astro"         # upstream Slint mirror — delete
  "draft/slint-ui/docs/safety"        # upstream Slint mirror — delete
  "draft/slint-ui/docs/nodejs"        # upstream Slint mirror — delete
)
for p in "${FORBIDDEN_PATHS[@]}"; do
  if [ -e "${p}" ]; then
    fail "stale path still present: ${p}" "should have been moved/removed in the refresh"
  else
    pass "absent: ${p}"
  fi
done

# ── 3. No loose draft-plan files left at docs/ top level ─────────────────────
section "docs/ tidiness"
LOOSE=$(find docs -maxdepth 1 -type f -name 'draft-plan-*.md' 2>/dev/null || true)
if [ -n "${LOOSE}" ]; then
  fail "loose draft-plan files at docs/ root" "move to docs/archive/draft-plans/:
$(printf '%s\n' "${LOOSE}" | sed 's/^/         /')"
else
  pass "no loose draft-plan-*.md at docs/ root"
fi

# ── 4. NDK version agreement: README must match flake.nix ────────────────────
section "Toolchain version consistency"
FLAKE_NDK="$(grep -oE 'ndkVersion = "[0-9.]+"' flake.nix | head -1 | grep -oE '[0-9.]+' || true)"
# Map the numeric NDK package version to its r-name major (e.g. 28.x -> r28).
if [ -n "${FLAKE_NDK}" ]; then
  NDK_MAJOR="${FLAKE_NDK%%.*}"
  if grep -qE "r${NDK_MAJOR}[a-z]" README.md; then
    pass "README NDK rev matches flake.nix (r${NDK_MAJOR}, pkg ${FLAKE_NDK})"
  else
    STALE="$(grep -oE 'NDK r[0-9]+[a-z]' README.md | head -1 || echo 'none')"
    fail "README NDK rev disagrees with flake.nix" \
         "flake pins ${FLAKE_NDK} (r${NDK_MAJOR}); README says '${STALE}'"
  fi
else
  fail "could not parse ndkVersion from flake.nix" "expected: ndkVersion = \"X.Y.Z\""
fi

# ── 5. No broken internal references to moved paths ──────────────────────────
section "Internal reference integrity"
# These tokens should no longer point at old locations anywhere except this
# script and the refresh plan (which document the move intentionally).
declare -A MOVED=(
  ["TODO.codecs/"]="docs/plans/codecs/"
  ["docs/refactor-implementation-guide/"]="docs/archive/refactor-implementation-guide/"
)
for token in "${!MOVED[@]}"; do
  HITS=$(grep -rIl --exclude-dir=.git \
            --exclude="check-repo-layout.sh" \
            --exclude="repository-refresh-plan.md" \
            -- "${token}" . 2>/dev/null || true)
  if [ -n "${HITS}" ]; then
    fail "dangling reference to '${token}' (now '${MOVED[$token]}')" \
         "$(printf '%s\n' "${HITS}" | sed 's/^/         /')"
  else
    pass "no dangling references to '${token}'"
  fi
done

# ── 6. Community files live where links expect them ──────────────────────────
section "Community file placement"
if [ -f "CONTRIBUTING.md" ] || [ -f ".github/CONTRIBUTING.md" ]; then
  pass "CONTRIBUTING.md resolvable"
else
  fail "CONTRIBUTING.md not found at root or .github/"
fi

# ── Summary ──────────────────────────────────────────────────────────────────
printf '\n────────────────────────────────────────\n'
printf 'repo-health: %s passed, %s failed\n' "$(green "${PASSES}")" \
  "$( [ "${FAILURES}" -gt 0 ] && red "${FAILURES}" || green 0 )"

if [ "${FAILURES}" -gt 0 ]; then
  if [ "${WARN_ONLY}" -eq 1 ]; then
    printf '%s (--warn set, exiting 0)\n' "$(yellow 'drift detected')"
    exit 0
  fi
  exit 1
fi
exit 0
