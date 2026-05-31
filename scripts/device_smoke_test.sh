#!/usr/bin/env bash
# scripts/device_smoke_test.sh — ADB-driven gstpop daemon smoke test.
# Usage: ./scripts/device_smoke_test.sh [package_name]
set -euo pipefail

PKG="${1:-org.fcast.android.sender}"
PORT=9000
PASS=0
FAIL=0

ok()   { echo "  ✓ $*"; PASS=$((PASS+1)); }
fail() { echo "  ✗ $*"; FAIL=$((FAIL+1)); }
step() { echo; echo "── $* ──────────────────────────────────"; }

# ── Prerequisites ────────────────────────────────────────────────────────────
step "Prerequisites"
adb get-state >/dev/null 2>&1 && ok "ADB device connected" || { fail "No ADB device"; exit 1; }
adb shell pm list packages | grep -q "$PKG"  && ok "App installed: $PKG" || { fail "App not installed"; exit 1; }

# ── Setup ────────────────────────────────────────────────────────────────────
step "Setup"
adb logcat -c
adb forward tcp:$PORT tcp:$PORT
ok "Forwarded device localhost:$PORT → host :$PORT"

# ── Launch app ───────────────────────────────────────────────────────────────
step "Launch"
adb shell am force-stop "$PKG"
sleep 1
adb shell am start -n "$PKG/.MainActivity" >/dev/null
sleep 3
ok "App launched"

# ── Wait for daemon ──────────────────────────────────────────────────────────
step "Daemon startup (via logcat)"
DAEMON_UP=false
for i in $(seq 1 15); do
    if adb logcat -d | grep -q "Embedded gst-pop running on"; then
        DAEMON_UP=true
        break
    fi
    sleep 1
done
$DAEMON_UP && ok "Daemon reached Running state" || fail "Daemon did not start within 15s"

# ── Port probe ───────────────────────────────────────────────────────────────
step "Port probe"
if command -v curl >/dev/null; then
    HTTP=$(curl -s --max-time 3 -o /dev/null -w "%{http_code}" http://127.0.0.1:$PORT/ || true)
    [[ "$HTTP" != "000" ]] && ok "HTTP probe: $HTTP" || fail "No response on port $PORT"
else
    echo "  (curl not found; skipping HTTP probe)"
fi

# ── JSON-RPC via websocat ────────────────────────────────────────────────────
step "JSON-RPC"
if ! command -v websocat >/dev/null; then
    echo "  (websocat not found; install with: cargo install websocat)"
    echo "  Skipping JSON-RPC tests"
else
    LIST=$(echo '{"id":"l0","method":"list_pipelines","params":{}}' \
        | websocat --no-close ws://127.0.0.1:$PORT/ 2>/dev/null | head -1)
    echo "$LIST" | grep -q '"result"' && ok "list_pipelines → $LIST" || fail "list_pipelines failed: $LIST"

    CREATE=$(echo '{"id":"c0","method":"create_pipeline","params":{"description":"videotestsrc ! fakesink"}}' \
        | websocat --no-close ws://127.0.0.1:$PORT/ 2>/dev/null | head -1)
    echo "$CREATE" | grep -q '"pipeline_id"' && ok "create_pipeline: $CREATE" || { fail "create_pipeline failed"; }

    PID=$(echo "$CREATE" | python3 -c "import sys,json; print(json.load(sys.stdin)['result']['pipeline_id'])" 2>/dev/null || true)
    [[ -z "$PID" ]] && { fail "Could not parse pipeline_id"; } || ok "Pipeline ID: $PID"

    PLAY=$(echo "{\"id\":\"p0\",\"method\":\"play\",\"params\":{\"pipeline_id\":\"$PID\"}}" \
        | websocat --no-close ws://127.0.0.1:$PORT/ 2>/dev/null | head -1)
    echo "$PLAY" | grep -q '"result"' && ok "play: $PLAY" || fail "play failed: $PLAY"
    sleep 0.5

    PAUSE=$(echo "{\"id\":\"pa0\",\"method\":\"pause\",\"params\":{\"pipeline_id\":\"$PID\"}}" \
        | websocat --no-close ws://127.0.0.1:$PORT/ 2>/dev/null | head -1)
    echo "$PAUSE" | grep -q '"result"' && ok "pause" || fail "pause failed: $PAUSE"

    STOP=$(echo "{\"id\":\"st0\",\"method\":\"stop\",\"params\":{\"pipeline_id\":\"$PID\"}}" \
        | websocat --no-close ws://127.0.0.1:$PORT/ 2>/dev/null | head -1)
    echo "$STOP" | grep -q '"result"' && ok "stop" || fail "stop failed: $STOP"

    REMOVE=$(echo "{\"id\":\"r0\",\"method\":\"remove_pipeline\",\"params\":{\"pipeline_id\":\"$PID\"}}" \
        | websocat --no-close ws://127.0.0.1:$PORT/ 2>/dev/null | head -1)
    echo "$REMOVE" | grep -q '"result"' && ok "remove_pipeline" || fail "remove_pipeline failed: $REMOVE"

    LIST2=$(echo '{"id":"l1","method":"list_pipelines","params":{}}' \
        | websocat --no-close ws://127.0.0.1:$PORT/ 2>/dev/null | head -1)
    echo "$LIST2" | grep -q '"pipelines":\[\]' && ok "list_pipelines empty after remove" || fail "List not empty: $LIST2"
fi

# ── Background / foreground resilience ──────────────────────────────────────
step "Background/foreground resilience"
adb shell input keyevent KEYCODE_HOME
sleep 3
adb shell am start -n "$PKG/.MainActivity" >/dev/null
sleep 2
LOG_RESTART=$(adb logcat -d | grep "Embedded gst-pop running" | wc -l | tr -d ' ')
[[ "$LOG_RESTART" -le 1 ]] && ok "No spurious restart (start count: $LOG_RESTART)" \
    || fail "Server restarted unexpectedly ($LOG_RESTART times)"

# ── Force-stop port release ──────────────────────────────────────────────────
step "Force-stop and clean relaunch"
adb shell am force-stop "$PKG"
sleep 2
adb shell am start -n "$PKG/.MainActivity" >/dev/null
sleep 4
BIND_FAIL=$(adb logcat -d | grep "gst-pop bind failed" | wc -l | tr -d ' ')
[[ "$BIND_FAIL" -eq 0 ]] && ok "No bind failure after force-stop" || fail "Bind failure detected after force-stop"

# ── Error check ──────────────────────────────────────────────────────────────
step "Error check"
ERRORS=$(adb logcat -d | grep -iE "gstpop.*failed|gstpop.*error|last_error" | grep -v "last_error.*null" | wc -l | tr -d ' ')
[[ "$ERRORS" -eq 0 ]] && ok "No unexpected errors in logcat" || fail "$ERRORS error(s) in logcat — check output"

# ── Summary ──────────────────────────────────────────────────────────────────
echo
echo "══════════════════════════════════"
echo "  Result: $PASS passed, $FAIL failed"
echo "══════════════════════════════════"
[[ "$FAIL" -eq 0 ]] && exit 0 || exit 1
