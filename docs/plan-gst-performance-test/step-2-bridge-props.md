# Step 2 — `ui/bridge.slint`: perf properties + callbacks

← [Step 1](step-1-bridge-panel-enum.md) · [Index](README.md) · Next → [Step 3](step-3-codec-perf-page-slint.md)

Insert **inside `export global Bridge { … }`**, immediately after the existing
**codec test** block (the last codec line is `codec-test-save-status` at
**line 478**), before the global's closing `}`.

> Deviation from research: it added only `perf-test-log`. We also add
> `perf-test-log-lines` so the page can render the report in a virtualised
> `ListView` (Step 3) — a single huge `Text` janks on scroll.

```slint
    // ── Codec performance benchmark (GStreamer pipeline throughput) ───────
    in property <string> perf-test-log: "";
    // Same report split into lines for a virtualised ListView (smooth scroll).
    in property <[string]> perf-test-log-lines: [];
    in property <bool>   perf-test-running: false;
    callback run-perf-test();
    callback run-perf-encode-only();
    callback run-perf-decode-only();
    callback run-perf-list-factories();
```

These generate the Rust accessors used in Step 8:
`set_perf_test_log`, `set_perf_test_log_lines`, `set_perf_test_running`,
`on_run_perf_test`, `on_run_perf_encode_only`, `on_run_perf_decode_only`,
`on_run_perf_list_factories`.

---

← [Step 1](step-1-bridge-panel-enum.md) · [Index](README.md) · Next → [Step 3](step-3-codec-perf-page-slint.md)
