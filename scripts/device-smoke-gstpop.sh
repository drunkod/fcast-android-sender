#!/usr/bin/env bash
# scripts/device-smoke-gstpop.sh — Automated ADB-driven gstpop daemon smoke test.
#
# Covers:
#   - Port forwarding (9000 -> 9000)
#   - Daemon startup detection via logcat
#   - HTTP probe via curl
#   - Full JSON-RPC lifecycle (list -> create -> play -> pause -> stop -> remove -> verify empty)
#   - Background/foreground resilience check
#   - Force-stop and clean relaunch
#   - Error count in logcat

set -euo pipefail

# ANSI color codes
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

PKG="org.fcast.android.sender"
PORT=9000
PASS=0
FAIL=0

ok()   { echo -e "  ${GREEN}✓${NC} $*"; PASS=$((PASS+1)); }
fail() { echo -e "  ${RED}✗${NC} $*"; FAIL=$((FAIL+1)); }
warn() { echo -e "  ${YELLOW}~${NC} $*"; }
step() { echo -e "\n${BLUE}── $* ──────────────────────────────────${NC}"; }

# ── Prerequisites ────────────────────────────────────────────────────────────
step "Prerequisites"
if adb get-state >/dev/null 2>&1; then
    ok "ADB device/emulator connected"
else
    fail "No active ADB device/emulator found. Please connect a device first."
    exit 1
fi

if adb shell pm list packages | grep -q "$PKG"; then
    ok "App installed: $PKG"
else
    fail "App not installed on device: $PKG"
    exit 1
fi

# ── Setup ────────────────────────────────────────────────────────────────────
step "Setup & Port Forwarding"
adb logcat -c
ok "Cleared device logcat buffer"

if adb forward tcp:$PORT tcp:$PORT; then
    ok "Forwarded device localhost:$PORT → host localhost:$PORT"
else
    fail "Failed to setup adb port forward"
    exit 1
fi

# ── Launch app ───────────────────────────────────────────────────────────────
step "Relaunch & Start Daemon"
adb shell am force-stop "$PKG"
sleep 1
adb shell am start -n "$PKG/.MainActivity" >/dev/null
ok "App launched. Navigate to Settings -> Media Backend, select 'gst-pop' and start the service."

# ── Wait for daemon ──────────────────────────────────────────────────────────
step "Daemon startup detection (via logcat)"
DAEMON_UP=false
for i in $(seq 1 15); do
    if adb logcat -d | grep -q "Embedded gst-pop running on"; then
        DAEMON_UP=true
        break
    fi
    sleep 1
done

if $DAEMON_UP; then
    ok "Daemon successfully reached Running state"
else
    fail "Daemon did not start within 15 seconds"
fi

# ── Port probe ───────────────────────────────────────────────────────────────
step "HTTP Port Probe"
if command -v curl >/dev/null; then
    HTTP=$(curl -s --max-time 3 -o /dev/null -w "%{http_code}" http://127.0.0.1:$PORT/ || true)
    if [[ "$HTTP" != "000" && "$HTTP" != "" ]]; then
        ok "HTTP probe succeeded: Received HTTP $HTTP"
    else
        fail "No HTTP response on port $PORT"
    fi
else
    warn "curl not found on host; skipping HTTP port probe"
fi

# ── JSON-RPC via websocat ────────────────────────────────────────────────────
step "JSON-RPC Lifecycle Verification"
if ! command -v websocat >/dev/null; then
    warn "websocat not found on host; skipping WebSocket tests. (Install: cargo install websocat)"
else
    # 1. list_pipelines (should be empty initially)
    LIST=$(echo '{"id":"l0","method":"list_pipelines","params":{}}' \
        | websocat --no-close ws://127.0.0.1:$PORT/ 2>/dev/null | head -1)
    if echo "$LIST" | grep -q '"result"'; then
        ok "list_pipelines succeeded: $LIST"
    else
        fail "list_pipelines failed: $LIST"
    fi

    # 2. create_pipeline
    CREATE=$(echo '{"id":"c0","method":"create_pipeline","params":{"description":"videotestsrc ! fakesink"}}' \
        | websocat --no-close ws://127.0.0.1:$PORT/ 2>/dev/null | head -1)
    if echo "$CREATE" | grep -q '"pipeline_id"'; then
        ok "create_pipeline succeeded: $CREATE"
    else
        fail "create_pipeline failed: $CREATE"
    fi

    # Parse pipeline_id
    PID=$(echo "$CREATE" | grep -o '"pipeline_id":"[^"]*' | cut -d'"' -f4 || true)
    if [[ -z "$PID" ]]; then
        fail "Could not parse pipeline_id from response"
    else
        ok "Parsed pipeline ID: $PID"

        # 3. play
        PLAY=$(echo "{\"id\":\"p0\",\"method\":\"play\",\"params\":{\"pipeline_id\":\"$PID\"}}" \
            | websocat --no-close ws://127.0.0.1:$PORT/ 2>/dev/null | head -1)
        if echo "$PLAY" | grep -q '"result"'; then
            ok "play succeeded: $PLAY"
        else
            fail "play failed: $PLAY"
        fi
        sleep 0.5

        # 4. pause
        PAUSE=$(echo "{\"id\":\"pa0\",\"method\":\"pause\",\"params\":{\"pipeline_id\":\"$PID\"}}" \
            | websocat --no-close ws://127.0.0.1:$PORT/ 2>/dev/null | head -1)
        if echo "$PAUSE" | grep -q '"result"'; then
            ok "pause succeeded"
        else
            fail "pause failed: $PAUSE"
        fi

        # 5. stop
        STOP=$(echo "{\"id\":\"st0\",\"method\":\"stop\",\"params\":{\"pipeline_id\":\"$PID\"}}" \
            | websocat --no-close ws://127.0.0.1:$PORT/ 2>/dev/null | head -1)
        if echo "$STOP" | grep -q '"result"'; then
            ok "stop succeeded"
        else
            fail "stop failed: $STOP"
        fi

        # 6. remove_pipeline
        REMOVE=$(echo "{\"id\":\"r0\",\"method\":\"remove_pipeline\",\"params\":{\"pipeline_id\":\"$PID\"}}" \
            | websocat --no-close ws://127.0.0.1:$PORT/ 2>/dev/null | head -1)
        if echo "$REMOVE" | grep -q '"result"'; then
            ok "remove_pipeline succeeded"
        else
            fail "remove_pipeline failed: $REMOVE"
        fi

        # 7. list_pipelines (should be empty again)
        LIST2=$(echo '{"id":"l1","method":"list_pipelines","params":{}}' \
            | websocat --no-close ws://127.0.0.1:$PORT/ 2>/dev/null | head -1)
        if echo "$LIST2" | grep -q '"pipelines":\[\]'; then
            ok "list_pipelines is empty after remove"
        else
            fail "List is not empty after remove: $LIST2"
        fi
    fi
fi

# ── Background / foreground resilience ──────────────────────────────────────
step "Background/Foreground Resilience"
adb shell input keyevent KEYCODE_HOME
ok "Sent app to background (KEYCODE_HOME)"
sleep 3

adb shell am start -n "$PKG/.MainActivity" >/dev/null
ok "Brought app back to foreground"
sleep 2

LOG_RESTART=$(adb logcat -d | grep "Embedded gst-pop running on" | wc -l | tr -d ' ')
if [[ "$LOG_RESTART" -le 1 ]]; then
    ok "No spurious restarts detected (started $LOG_RESTART times)"
else
    fail "Server restarted unexpectedly ($LOG_RESTART times)"
fi

# ── Force-stop port release ──────────────────────────────────────────────────
step "Force-Stop & Port Reuse"
adb shell am force-stop "$PKG"
ok "Force-stopped app"
sleep 2

adb shell am start -n "$PKG/.MainActivity" >/dev/null
ok "Relaunching app..."
sleep 4

BIND_FAIL=$(adb logcat -d | grep "gst-pop bind failed" | wc -l | tr -d ' ')
if [[ "$BIND_FAIL" -eq 0 ]]; then
    ok "No bind failures detected after relaunch"
else
    fail "Bind failure detected on relaunch"
fi

# ── Error check ──────────────────────────────────────────────────────────────
step "Error logcat audit"
ERRORS=$(adb logcat -d | grep -iE "gstpop.*failed|gstpop.*error|last_error" | grep -v "last_error.*null" | wc -l | tr -d ' ')
if [[ "$ERRORS" -eq 0 ]]; then
    ok "No unexpected errors found in logcat"
else
    fail "$ERRORS error(s) found in logcat — please check logcat output"
fi

# ── Summary ──────────────────────────────────────────────────────────────────
echo -e "\n${BLUE}═════════════════════════════════════════════════════════${NC}"
if [[ "$FAIL" -eq 0 ]]; then
    echo -e "  ${GREEN}All tests passed successfully! ($PASS passed, 0 failed)${NC}"
else
    echo -e "  ${RED}Some tests failed ($PASS passed, $FAIL failed)${NC}"
fi
echo -e "${BLUE}═════════════════════════════════════════════════════════${NC}"

if [[ "$FAIL" -eq 0 ]]; then
    exit 0
else
    exit 1
fi
