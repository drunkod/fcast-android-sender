# Step 14 — On-device gstpop daemon smoke test via ADB

**Phase:** 2 — Android polish  
**Priority:** high (validates the full Phase 1 stack on real hardware)  
**Depends on:** Steps 1–9 (Phase 1 complete)  
**Unblocks:** Phase 3 (production pipeline flows)

---

## Goal

Prove end-to-end on a real Android device (or emulator) that:

1. The embedded gstpop server starts, reaches `Running`, and logs its port.
2. A `videotestsrc ! fakesink` pipeline survives create → play → pause → stop → remove.
3. `EmbeddedStatus.last_error` is empty on a clean run.
4. The server survives app background/foreground cycling.
5. Force-stopping the app releases the port so the next launch reuses it cleanly.

These are the runtime guarantees that Steps 1–9 defined in unit/integration tests.  
Step 14 verifies them on real hardware with real GStreamer plugins.

---

## Files touched

| File | Action |
|---|---|
| `ui/pages/gstpop_device_test_page.slint` | New — dedicated on-device test UI |
| `ui/bridge.slint` | Extend with `gstpop-device-test-*` properties and callbacks |
| `ui/main.slint` or `ui/pages/debug_page.slint` | Wire navigation to new page |
| `src/shell/SenderController.kt` (or equivalent) | Handle new bridge callbacks |

---

## Prerequisites

### 1. Device / emulator setup

```bash
# Confirm ADB sees the device
adb devices
# Expected: one device listed as "device" (not "unauthorized")

# Confirm architecture
adb shell getprop ro.product.cpu.abi
# arm64-v8a  — matches our build target

# Confirm Android version
adb shell getprop ro.build.version.release
# Must be ≥ 8.0 (API 26) for the foreground service requirements
```

### 2. Build the debug APK

```bash
# From repo root, inside the nix dev shell:
nix develop .#android --command \
  cargo ndk -t arm64-v8a -o app/src/main/jniLibs build \
    -p android-sender \
    --features "typed-client media-tools" \
    --release

./gradlew :app:assembleDebug

# Verify the .so is present
unzip -l app/build/outputs/apk/debug/app-debug.apk | grep '\.so$'
# Expect:   lib/arm64-v8a/libandroid_sender.so
# No x86_64 entries until Step 11
```

### 3. Install and clear logcat

```bash
adb install -r app/build/outputs/apk/debug/app-debug.apk
adb logcat -c          # clear existing log buffer
```

---

## Part A — Logcat-only smoke test (no UI changes needed)

This tests the daemon with the **existing** Media Backend page.

### Step A1 — Launch app and start gstpop service

```bash
# Terminal 1: tail relevant log tags
adb logcat -s "gstpop-runtime" -s "GstPopBridge" -s "GstPopServiceBridge" \
           -s "fcastsender" -s "JniRuntimeBridge"
```

On device:
1. Open app → navigate to **Settings → Media Backend**
2. Change engine to **gst-pop (WebSocket)**
3. Tap **Start service**

Expected logcat output (within ~2 s):

```
I gstpop-runtime: Embedded gst-pop running on 127.0.0.1:9000
I GstPopServiceBridge: status={"state":"running","bind":"127.0.0.1","port":9000,...}
```

**FAIL criteria:**
- `Embedded gst-pop bind failed` → port conflict; check if another instance is running
- `Failed to bind WebSocket server` → firewall/socket issue
- No log within 5 s → GStreamer init failure; check `adb logcat | grep -i gstreamer`

### Step A2 — Port forward and probe via curl

```bash
# Forward device localhost:9000 → host localhost:9000
adb forward tcp:9000 tcp:9000

# In a separate terminal — verify the server accepts connections
curl -s --max-time 3 http://127.0.0.1:9000/  | head -20
# gstpop serves an HTTP upgrade page; any response means the port is live

# WebSocket ping via websocat (install: cargo install websocat)
websocat ws://127.0.0.1:9000/ <<'EOF'
{"id":"test-1","method":"list_pipelines","params":{}}
EOF
# Expected: {"id":"test-1","result":{"pipelines":[]},...}
```

### Step A3 — Create and play a pipeline via JSON-RPC

```bash
# Create pipeline
websocat ws://127.0.0.1:9000/ <<'EOF'
{"id":"c1","method":"create_pipeline","params":{"description":"videotestsrc ! fakesink"}}
EOF
# Expected: {"id":"c1","result":{"pipeline_id":"<id>"},...}
# Save the returned pipeline_id as $PID

PID="0"   # replace with actual id from the response

# Play
websocat ws://127.0.0.1:9000/ <<'EOF'
{"id":"p1","method":"play","params":{"pipeline_id":"0"}}
EOF

# Logcat should show:
# I gstpop-runtime: ...StateChanged playing

# Pause
websocat ws://127.0.0.1:9000/ <<'EOF'
{"id":"p2","method":"pause","params":{"pipeline_id":"0"}}
EOF

# Stop
websocat ws://127.0.0.1:9000/ <<'EOF'
{"id":"p3","method":"stop","params":{"pipeline_id":"0"}}
EOF

# Remove
websocat ws://127.0.0.1:9000/ <<'EOF'
{"id":"p4","method":"remove_pipeline","params":{"pipeline_id":"0"}}
EOF

# Verify empty
websocat ws://127.0.0.1:9000/ <<'EOF'
{"id":"l1","method":"list_pipelines","params":{}}
EOF
# Expected: {"result":{"pipelines":[]},...}
```

### Step A4 — Background / foreground resilience

```bash
# Send app to background
adb shell input keyevent KEYCODE_HOME

sleep 3

# Bring back to foreground
adb shell am start -n org.fcast.android.sender/.MainActivity

# Check service still Running (not restarted)
adb logcat | grep -E "gstpop-runtime|GstPopServiceBridge" | tail -5
# Must NOT contain "Embedded gst-pop running on" a second time
# (idempotent: second start_embedded returns immediately)
```

### Step A5 — Force-stop and port reuse

```bash
adb shell am force-stop org.fcast.android.sender
sleep 2
adb shell am start -n org.fcast.android.sender/.MainActivity

# Navigate to Media Backend → Start service again
# Logcat MUST show a clean "running on 127.0.0.1:9000" without "bind failed"
```

---

## Part B — Dedicated device-test UI page

Create a self-contained test panel that runs the full pipeline lifecycle from
the device screen — no ADB shell needed after install.

### B1 — New Slint page

Create `ui/pages/gstpop_device_test_page.slint`:

```slint
// ui/pages/gstpop_device_test_page.slint — On-device gstpop daemon test panel.
//
// Exercises the full embedded-server lifecycle:
//   daemon start → create pipeline → play → pause → stop → remove → daemon stop
//
// All state flows through Bridge.gstpop-dt-* properties so Rust can drive
// it from the gstpop-runtime typed client without touching UI thread directly.

import { ScrollView, LineEdit } from "std-widgets.slint";
import { Bridge } from "../bridge.slint";
import { PanelBridge } from "../state/panel_bridge.slint";
import { Theme } from "../theme.slint";
import { PrimaryButton, DestructiveButton, TextButton } from "../components/buttons.slint";
import { PanelHeader, Card } from "../components/panel_chrome.slint";
import { SettingsSection } from "../components/settings_rows.slint";


// ─────────────────────────────────────────────────────────────────────────────
// Internal: coloured state dot
// ─────────────────────────────────────────────────────────────────────────────
component StateDot inherits Rectangle {
    in property <string> state: "stopped";
    width: 10px;
    height: 10px;
    border-radius: 5px;
    background:
        root.state == "running"  ? Theme.success   :
        root.state == "starting" ? Theme.warning    :
        root.state == "error"    ? Theme.error-fg   :
        Theme.text-disabled;
    y: (parent.height - self.height) / 2;
}


// ─────────────────────────────────────────────────────────────────────────────
// Internal: single log line
// ─────────────────────────────────────────────────────────────────────────────
component LogLine inherits Text {
    in property <string> level: "info";   // "info" | "ok" | "warn" | "error"
    font-size: 11px;
    wrap: word-wrap;
    color:
        root.level == "ok"    ? Theme.success  :
        root.level == "warn"  ? Theme.warning  :
        root.level == "error" ? Theme.error-fg :
        Theme.text-secondary;
}


// ─────────────────────────────────────────────────────────────────────────────
// Internal: result row (label + value)
// ─────────────────────────────────────────────────────────────────────────────
component ResultRow inherits HorizontalLayout {
    in property <string> label;
    in property <string> value;
    in property <string> value-color: "normal";  // "normal"|"ok"|"error"
    spacing: 8px;
    min-height: 28px;

    Text {
        text: root.label;
        color: Theme.text-secondary;
        font-size: Theme.font-size-label;
        width: 120px;
        vertical-alignment: center;
    }
    Text {
        text: root.value;
        color:
            root.value-color == "ok"    ? Theme.success  :
            root.value-color == "error" ? Theme.error-fg :
            Theme.text-primary;
        font-size: Theme.font-size-label;
        horizontal-stretch: 1;
        vertical-alignment: center;
        wrap: word-wrap;
    }
}


// ─────────────────────────────────────────────────────────────────────────────
// Exported: GstPopDeviceTestPage
// ─────────────────────────────────────────────────────────────────────────────
export component GstPopDeviceTestPage inherits Rectangle {
    width: 100%;
    height: 100%;
    background: Theme.surface-primary;

    forward-focus: scope;
    scope := FocusScope {
        key-pressed(event) => {
            if event.text == Key.Escape { PanelBridge.pop(); return accept; }
            return reject;
        }

    VerticalLayout {

        // ── Header ────────────────────────────────────────────────────────
        PanelHeader {
            title: @tr("gstpop Daemon Test");
            close-clicked => { PanelBridge.pop(); }
        }

        // ── Scrollable body ───────────────────────────────────────────────
        ScrollView {
            mouse-drag-pan-enabled: true;

            VerticalLayout {
                alignment: start;
                spacing: Theme.spacing-default;
                padding: Theme.padding-screen;

                // ── DAEMON STATUS ─────────────────────────────────────────
                SettingsSection {
                    title: @tr("DAEMON STATUS");
                    Card {
                        VerticalLayout {
                            padding: Theme.padding-screen;
                            spacing: 8px;

                            HorizontalLayout {
                                spacing: 8px;
                                StateDot { state: Bridge.gstpop-dt-daemon-state; }
                                Text {
                                    text:
                                        Bridge.gstpop-dt-daemon-state == "running"  ? @tr("Running") :
                                        Bridge.gstpop-dt-daemon-state == "starting" ? @tr("Starting\u{2026}") :
                                        Bridge.gstpop-dt-daemon-state == "error"    ? @tr("Error") :
                                                                                      @tr("Stopped");
                                    color: Theme.text-primary;
                                    font-size: Theme.font-size-body;
                                    horizontal-stretch: 1;
                                    vertical-alignment: center;
                                }
                            }

                            ResultRow {
                                label: @tr("Bind address");
                                value: Bridge.gstpop-dt-daemon-bind != ""
                                    ? Bridge.gstpop-dt-daemon-bind
                                    : @tr("—");
                            }
                            ResultRow {
                                label: @tr("Port");
                                value: Bridge.gstpop-dt-daemon-port > 0
                                    ? Bridge.gstpop-dt-daemon-port
                                    : @tr("—");
                            }
                            ResultRow {
                                label: @tr("Externally owned");
                                value: Bridge.gstpop-dt-daemon-external
                                    ? @tr("Yes")
                                    : @tr("No");
                            }

                            if Bridge.gstpop-dt-daemon-last-error != "": ResultRow {
                                label: @tr("Last error");
                                value: Bridge.gstpop-dt-daemon-last-error;
                                value-color: "error";
                            }
                        }
                    }
                }

                // ── PIPELINE DESCRIPTION INPUT ────────────────────────────
                SettingsSection {
                    title: @tr("PIPELINE");
                    Card {
                        VerticalLayout {
                            padding: Theme.padding-screen;
                            spacing: 8px;

                            Text {
                                text: @tr("GStreamer launch description");
                                color: Theme.text-secondary;
                                font-size: Theme.font-size-label;
                            }
                            LineEdit {
                                text <=> Bridge.gstpop-dt-pipeline-desc;
                                placeholder-text: "videotestsrc ! fakesink";
                            }

                            if Bridge.gstpop-dt-pipeline-id != "": ResultRow {
                                label: @tr("Pipeline ID");
                                value: Bridge.gstpop-dt-pipeline-id;
                                value-color: "ok";
                            }

                            ResultRow {
                                label: @tr("State");
                                value: Bridge.gstpop-dt-pipeline-state != ""
                                    ? Bridge.gstpop-dt-pipeline-state
                                    : @tr("—");
                                value-color:
                                    Bridge.gstpop-dt-pipeline-state == "playing" ? "ok" :
                                    Bridge.gstpop-dt-pipeline-state == "error"   ? "error" :
                                    "normal";
                            }
                        }
                    }
                }

                // ── ACTIONS ───────────────────────────────────────────────
                SettingsSection {
                    title: @tr("ACTIONS");
                    Card {
                        VerticalLayout {
                            padding: Theme.padding-screen;
                            spacing: 8px;

                            // Row 1: daemon lifecycle
                            HorizontalLayout {
                                spacing: 8px;
                                PrimaryButton {
                                    label: @tr("Start daemon");
                                    enabled: Bridge.gstpop-dt-daemon-state == "stopped"
                                          || Bridge.gstpop-dt-daemon-state == "error";
                                    clicked => { Bridge.gstpop-dt-start-daemon(); }
                                    horizontal-stretch: 1;
                                }
                                DestructiveButton {
                                    label: @tr("Stop daemon");
                                    enabled: Bridge.gstpop-dt-daemon-state == "running";
                                    clicked => { Bridge.gstpop-dt-stop-daemon(); }
                                    horizontal-stretch: 1;
                                }
                            }

                            // Row 2: pipeline lifecycle
                            HorizontalLayout {
                                spacing: 8px;
                                PrimaryButton {
                                    label: @tr("Create");
                                    enabled: Bridge.gstpop-dt-daemon-state == "running"
                                          && Bridge.gstpop-dt-pipeline-id == "";
                                    clicked => { Bridge.gstpop-dt-create-pipeline(); }
                                    horizontal-stretch: 1;
                                }
                                DestructiveButton {
                                    label: @tr("Remove");
                                    enabled: Bridge.gstpop-dt-pipeline-id != "";
                                    clicked => { Bridge.gstpop-dt-remove-pipeline(); }
                                    horizontal-stretch: 1;
                                }
                            }

                            // Row 3: playback control
                            HorizontalLayout {
                                spacing: 8px;
                                PrimaryButton {
                                    label: @tr("Play");
                                    enabled: Bridge.gstpop-dt-pipeline-id != ""
                                          && Bridge.gstpop-dt-pipeline-state != "playing";
                                    clicked => { Bridge.gstpop-dt-play(); }
                                    horizontal-stretch: 1;
                                }
                                TextButton {
                                    label: @tr("Pause");
                                    enabled: Bridge.gstpop-dt-pipeline-state == "playing";
                                    clicked => { Bridge.gstpop-dt-pause(); }
                                    horizontal-stretch: 1;
                                }
                                DestructiveButton {
                                    label: @tr("Stop");
                                    enabled: Bridge.gstpop-dt-pipeline-state == "playing"
                                          || Bridge.gstpop-dt-pipeline-state == "paused";
                                    clicked => { Bridge.gstpop-dt-stop-pipeline(); }
                                    horizontal-stretch: 1;
                                }
                            }

                            // Full-lifecycle shortcut
                            PrimaryButton {
                                label: @tr("Run full lifecycle test");
                                enabled: Bridge.gstpop-dt-daemon-state == "running"
                                      && !Bridge.gstpop-dt-test-running;
                                clicked => { Bridge.gstpop-dt-run-full-test(); }
                                horizontal-stretch: 1;
                            }
                        }
                    }
                }

                // ── EVENT LOG ─────────────────────────────────────────────
                SettingsSection {
                    title: @tr("EVENT LOG");
                    Card {
                        min-card-height: 200px;
                        VerticalLayout {
                            padding: Theme.padding-screen;
                            spacing: 4px;

                            HorizontalLayout {
                                Text {
                                    text: @tr("Recent events (newest first)");
                                    color: Theme.text-secondary;
                                    font-size: Theme.font-size-label;
                                    horizontal-stretch: 1;
                                }
                                TextButton {
                                    label: @tr("Clear");
                                    clicked => { Bridge.gstpop-dt-clear-log(); }
                                }
                            }

                            if Bridge.gstpop-dt-log-line-0 != "": LogLine {
                                text: Bridge.gstpop-dt-log-line-0;
                                level: Bridge.gstpop-dt-log-level-0;
                            }
                            if Bridge.gstpop-dt-log-line-1 != "": LogLine {
                                text: Bridge.gstpop-dt-log-line-1;
                                level: Bridge.gstpop-dt-log-level-1;
                            }
                            if Bridge.gstpop-dt-log-line-2 != "": LogLine {
                                text: Bridge.gstpop-dt-log-line-2;
                                level: Bridge.gstpop-dt-log-level-2;
                            }
                            if Bridge.gstpop-dt-log-line-3 != "": LogLine {
                                text: Bridge.gstpop-dt-log-line-3;
                                level: Bridge.gstpop-dt-log-level-3;
                            }
                            if Bridge.gstpop-dt-log-line-4 != "": LogLine {
                                text: Bridge.gstpop-dt-log-line-4;
                                level: Bridge.gstpop-dt-log-level-4;
                            }
                            if Bridge.gstpop-dt-log-line-5 != "": LogLine {
                                text: Bridge.gstpop-dt-log-line-5;
                                level: Bridge.gstpop-dt-log-level-5;
                            }
                            if Bridge.gstpop-dt-log-line-6 != "": LogLine {
                                text: Bridge.gstpop-dt-log-line-6;
                                level: Bridge.gstpop-dt-log-level-6;
                            }
                            if Bridge.gstpop-dt-log-line-7 != "": LogLine {
                                text: Bridge.gstpop-dt-log-line-7;
                                level: Bridge.gstpop-dt-log-level-7;
                            }

                            if Bridge.gstpop-dt-log-line-0 == "": Text {
                                text: @tr("No events yet. Start the daemon and run actions.");
                                color: Theme.text-disabled;
                                font-size: Theme.font-size-label;
                                horizontal-alignment: center;
                            }
                        }
                    }
                }

                // ── FULL TEST RESULT ──────────────────────────────────────
                if Bridge.gstpop-dt-test-result != "": SettingsSection {
                    title: @tr("LAST FULL TEST");
                    Card {
                        VerticalLayout {
                            padding: Theme.padding-screen;
                            spacing: 4px;
                            Text {
                                text: Bridge.gstpop-dt-test-result;
                                color:
                                    Bridge.gstpop-dt-test-passed
                                        ? Theme.success
                                        : Theme.error-fg;
                                font-size: Theme.font-size-body;
                                wrap: word-wrap;
                            }
                        }
                    }
                }

                Rectangle { height: 48px; background: transparent; }
            }
        }
    }
    }  // FocusScope
}
```

### B2 — Bridge additions (`ui/bridge.slint`)

Add to the `AppBridge` singleton (inside `export global Bridge`):

```slint
// ── gstpop device-test panel (Step 14) ──────────────────────────────────────
// Daemon status
in-out property <string> gstpop-dt-daemon-state:      "stopped";
in-out property <string> gstpop-dt-daemon-bind:       "";
in-out property <int>    gstpop-dt-daemon-port:        0;
in-out property <bool>   gstpop-dt-daemon-external:   false;
in-out property <string> gstpop-dt-daemon-last-error: "";
// Pipeline under test
in-out property <string> gstpop-dt-pipeline-desc:  "videotestsrc ! fakesink";
in-out property <string> gstpop-dt-pipeline-id:    "";
in-out property <string> gstpop-dt-pipeline-state: "";
// Log lines (8-slot ring, newest-first; level: "info"|"ok"|"warn"|"error")
in-out property <string> gstpop-dt-log-line-0: "";  in-out property <string> gstpop-dt-log-level-0: "info";
in-out property <string> gstpop-dt-log-line-1: "";  in-out property <string> gstpop-dt-log-level-1: "info";
in-out property <string> gstpop-dt-log-line-2: "";  in-out property <string> gstpop-dt-log-level-2: "info";
in-out property <string> gstpop-dt-log-line-3: "";  in-out property <string> gstpop-dt-log-level-3: "info";
in-out property <string> gstpop-dt-log-line-4: "";  in-out property <string> gstpop-dt-log-level-4: "info";
in-out property <string> gstpop-dt-log-line-5: "";  in-out property <string> gstpop-dt-log-level-5: "info";
in-out property <string> gstpop-dt-log-line-6: "";  in-out property <string> gstpop-dt-log-level-6: "info";
in-out property <string> gstpop-dt-log-line-7: "";  in-out property <string> gstpop-dt-log-level-7: "info";
// Full lifecycle test
in-out property <bool>   gstpop-dt-test-running: false;
in-out property <bool>   gstpop-dt-test-passed:  false;
in-out property <string> gstpop-dt-test-result:  "";
// Callbacks (UI → Rust)
callback gstpop-dt-start-daemon();
callback gstpop-dt-stop-daemon();
callback gstpop-dt-create-pipeline();
callback gstpop-dt-remove-pipeline();
callback gstpop-dt-play();
callback gstpop-dt-pause();
callback gstpop-dt-stop-pipeline();
callback gstpop-dt-run-full-test();
callback gstpop-dt-clear-log();
```

### B3 — Navigation wiring (`ui/pages/debug_page.slint`)

Inside `DebugPageInner`, add a button that pushes the new page:

```slint
// Add to debug_page.slint imports:
import { GstPopDeviceTestPage } from "gstpop_device_test_page.slint";

// Add inside DebugPageInner VerticalBox, after existing buttons:
PrimaryButton {
    label: @tr("gstpop Daemon Test");
    clicked => { PanelBridge.push(Panel.gstpop-device-test); }
}
```

And register `Panel.gstpop-device-test` in the panel router (`ui/state/panel_bridge.slint` and `main.slint`) following the same pattern as the existing `TestFunctionality` panel.

### B4 — Rust callback handlers

In `src/shell/SenderController.kt` (or the equivalent handler file), wire the callbacks. Pseudocode — adapt to match your actual bridge Rust API:

```rust
// In the Rust side that handles bridge callbacks:

bridge.on_gstpop_dt_start_daemon({
    let ui = ui_handle.clone();
    move || {
        tokio::spawn(async move {
            push_log(&ui, "Starting embedded daemon…", "info");
            let status = gstpop_runtime::start_embedded(9000).await;
            let st = format!("{:?}", status.state).to_lowercase();
            ui.upgrade_in_event_loop(move |ui| {
                ui.global::<Bridge>().set_gstpop_dt_daemon_state(st.into());
                ui.global::<Bridge>().set_gstpop_dt_daemon_bind(status.bind.into());
                ui.global::<Bridge>().set_gstpop_dt_daemon_port(status.port as i32);
                ui.global::<Bridge>().set_gstpop_dt_daemon_external(status.externally_owned);
                if let Some(err) = status.last_error {
                    ui.global::<Bridge>().set_gstpop_dt_daemon_last_error(err.into());
                }
            }).ok();
            let level = if status.state == EmbeddedState::Running { "ok" } else { "error" };
            push_log(&ui, &format!("Daemon state: {:?}", status.state), level);
        });
    }
});

bridge.on_gstpop_dt_run_full_test({
    let ui = ui_handle.clone();
    move || {
        tokio::spawn(async move {
            run_full_lifecycle_test(ui.clone()).await;
        });
    }
});

// Full lifecycle test coroutine
async fn run_full_lifecycle_test(ui: slint::Weak<AppWindow>) {
    use gstpop_runtime::{GstPopClient, TypedGstPopClient};

    set_prop(&ui, |b| b.set_gstpop_dt_test_running(true));
    set_prop(&ui, |b| b.set_gstpop_dt_test_result("".into()));

    macro_rules! step {
        ($label:expr, $result:expr) => {{
            match $result {
                Ok(v) => { push_log(&ui, &format!("✓ {}", $label), "ok"); v }
                Err(e) => {
                    let msg = format!("✗ {}: {e:#}", $label);
                    push_log(&ui, &msg, "error");
                    set_prop(&ui, |b| { b.set_gstpop_dt_test_running(false); b.set_gstpop_dt_test_passed(false); b.set_gstpop_dt_test_result(msg.into()); });
                    return;
                }
            }
        }};
    }

    // 1. Ensure daemon running
    let status = gstpop_runtime::start_embedded(9000).await;
    if !matches!(status.state, EmbeddedState::Running) {
        let msg = format!("✗ Daemon failed to start: {:?}", status.last_error);
        push_log(&ui, &msg, "error");
        set_prop(&ui, |b| { b.set_gstpop_dt_test_running(false); b.set_gstpop_dt_test_passed(false); b.set_gstpop_dt_test_result(msg.into()); });
        return;
    }
    push_log(&ui, "✓ Daemon running", "ok");

    // 2. Connect typed client
    let url = format!("ws://127.0.0.1:{}/", status.port);
    let inner = step!("Client connect", GstPopClient::connect(&url, None).await);
    let client = TypedGstPopClient::new(inner);

    // 3. Pipeline lifecycle
    let desc = ui.upgrade().map(|u| u.global::<Bridge>().get_gstpop_dt_pipeline_desc().to_string())
        .unwrap_or_else(|| "videotestsrc ! fakesink".into());
    let pid = step!("create_pipeline", client.create_pipeline(&desc).await);
    push_log(&ui, &format!("  id={pid}"), "info");
    set_prop(&ui, |b| b.set_gstpop_dt_pipeline_id(pid.clone().into()));

    step!("play", client.play(Some(&pid)).await);
    set_prop(&ui, |b| b.set_gstpop_dt_pipeline_state("playing".into()));
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    step!("pause", client.pause(Some(&pid)).await);
    set_prop(&ui, |b| b.set_gstpop_dt_pipeline_state("paused".into()));
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    step!("stop",  client.stop(Some(&pid)).await);
    set_prop(&ui, |b| b.set_gstpop_dt_pipeline_state("null".into()));

    step!("remove_pipeline", client.remove_pipeline(&pid).await);
    set_prop(&ui, |b| { b.set_gstpop_dt_pipeline_id("".into()); b.set_gstpop_dt_pipeline_state("".into()); });

    let pipelines = step!("list_pipelines (empty)", client.list_pipelines().await);
    if !pipelines.is_empty() {
        let msg = format!("✗ list_pipelines: expected empty, got {} entries", pipelines.len());
        push_log(&ui, &msg, "error");
        set_prop(&ui, |b| { b.set_gstpop_dt_test_running(false); b.set_gstpop_dt_test_passed(false); b.set_gstpop_dt_test_result(msg.into()); });
        return;
    }

    let result = "✓ All steps passed — daemon is healthy".to_string();
    push_log(&ui, &result, "ok");
    set_prop(&ui, |b| {
        b.set_gstpop_dt_test_running(false);
        b.set_gstpop_dt_test_passed(true);
        b.set_gstpop_dt_test_result(result.into());
    });
}
```

---

## Part C — ADB automated test script

Save as `scripts/device_smoke_test.sh`:

```bash
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
```

Make it executable:

```bash
chmod +x scripts/device_smoke_test.sh
```

Run:

```bash
./scripts/device_smoke_test.sh
```

---

## Part D — Manual UI test checklist

Use the `GstPopDeviceTestPage` (navigate: Debug panel → **gstpop Daemon Test**):

| # | Action | Expected result |
|---|---|---|
| 1 | Tap **Start daemon** | State dot turns yellow (Starting) then green (Running). Bind shows `127.0.0.1`, port shows `9000`. Last error empty. |
| 2 | Default desc `videotestsrc ! fakesink`, tap **Create** | Pipeline ID field fills in. Pipeline State shows `—`. |
| 3 | Tap **Play** | Pipeline State → `playing`. Event log shows `StateChanged playing`. |
| 4 | Tap **Pause** | Pipeline State → `paused`. |
| 5 | Tap **Stop** | Pipeline State → `null`. |
| 6 | Tap **Remove** | Pipeline ID clears. |
| 7 | Tap **Run full lifecycle test** | All log entries green (✓). Last Full Test section shows `✓ All steps passed`. |
| 8 | Background app (Home) → return | Daemon state still shows Running. No restart in log. |
| 9 | Tap **Stop daemon** | State → Stopped, port → 0. |
| 10 | Re-tap **Start daemon** | Clean restart; no error. |

---

## Common failure modes

| Symptom | Likely cause | Fix |
|---|---|---|
| `Failed to bind on 127.0.0.1:9000` | Previous instance still holds port | Force-stop app; wait 2 s; restart |
| `Daemon did not start within 15s` | GStreamer init crash | `adb logcat \| grep -i gstreamer`; check .so in APK |
| `list_pipelines` returns unparseable JSON | Server not fully up when client connected | Increase grace period in `wait_for_port_on` |
| Daemon shows `externally_owned: true` | Another process listens on 9000 | `adb shell ss -tlnp \| grep 9000`; kill it |
| State stays `starting` indefinitely | Tokio runtime init blocked | Check for blocking calls on the JNI thread |
| `play` returns error | GStreamer plugin missing on device | Use `fakesrc ! fakesink` instead; check gst-inspect output |
| `websocat: connection refused` | `adb forward` not run, or wrong port | Run `adb forward tcp:9000 tcp:9000` first |

---

## Logcat filter reference

```bash
# Full gstpop-runtime trace
adb logcat -s "gstpop-runtime" "*:V"

# JNI bridge and Kotlin bridge
adb logcat -s "GstPopServiceBridge" -s "GstPopBridge" -s "JniRuntimeBridge"

# All gstpop-relevant tags
adb logcat | grep -iE "gstpop|embedded|pipeline|fakesink|gstreamer"

# Errors only
adb logcat "*:E" | grep -iE "gstpop|gstreamer|sender"

# WebSocket events (StateChanged, EOS, Error)
adb logcat | grep -E "StateChanged|EndOfStream|gst-pop error"
```

---

## Done when

- [ ] `scripts/device_smoke_test.sh` exits 0 on arm64 device with no failures.
- [ ] Manual UI checklist rows 1–10 all pass without crash or ANR.
- [ ] `last_error` remains empty throughout a clean run.
- [ ] Background/foreground cycle does not restart the daemon.
- [ ] Force-stop followed by relaunch binds the port cleanly.
- [ ] `adb logcat` shows `Embedded gst-pop running on 127.0.0.1:9000` exactly once per process lifetime.
