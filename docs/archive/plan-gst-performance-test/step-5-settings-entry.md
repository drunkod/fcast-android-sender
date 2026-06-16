# Step 5 — `ui/pages/settings_page.slint`: DEBUG-section row

← [Step 4](step-4-main-slint-route.md) · [Index](README.md) · Next → [Step 6](step-6-codec-perf-rs.md)

The DEBUG section's "H.264 encoder test" `SettingsValueRow` is at
**lines 338–344**. Add a "Codec performance test" row right after it (before the
"Debug log" row), mirroring the existing row structure:

```slint
                // ── Section: DEBUG ────────────────────────────────────────────
                SettingsSection {
                    title: @tr("DEBUG");
                    SettingsValueRow {
                        icon: "🧪";
                        icon-bg: Theme.icon-bg-neutral;
                        title: @tr("H.264 encoder test");
                        value: @tr("open-panel-action" => "Open");
                        clicked => { PanelBridge.push(Panel.codec-test); }
                    }
                    SettingsValueRow {                                       // ← ADD (whole block)
                        icon: "⚡";
                        icon-bg: Theme.icon-bg-neutral;
                        title: @tr("Codec performance test");
                        value: @tr("open-panel-action" => "Open");
                        clicked => { PanelBridge.push(Panel.codec-perf); }
                    }
                    SettingsValueRow {
                        icon: "📋";
                        icon-bg: Theme.icon-bg-neutral;
                        title: @tr("Debug log");
                        value: @tr("open-panel-action" => "Open");
                        clicked => { PanelBridge.push(Panel.debug-log); }
                    }
```

> Note: the research doc cited line 340; in the current tree the row is at
> 338–344 and uses `icon-bg: Theme.icon-bg-neutral` + the
> `@tr("open-panel-action" => "Open")` value form — matched above.

---

← [Step 4](step-4-main-slint-route.md) · [Index](README.md) · Next → [Step 6](step-6-codec-perf-rs.md)
