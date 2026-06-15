# Step 8 — `src/android_main.rs`: wire Bridge callbacks

← [Step 7](step-7-lib-rs-register.md) · [Index](README.md) · Next → [Step 9](step-9-verification.md)

Insert **after the codec-test callback handlers** (the `on_run_codec_*` /
`on_save_codec_log` block), just before the
`use crate::jni_bridge::camera::{ … };` block.

> Deviation from research: each handler publishes the report through a
> `set_perf_log` helper that sets **both** `perf-test-log` (string) and
> `perf-test-log-lines` (model), so the page's virtualised `ListView` (Step 3)
> renders it. This mirrors the existing `set_codec_log` helper. The blocking
> GStreamer work stays on `std::thread::spawn`.

```rust
    // ── Codec performance benchmark callbacks ────────────────────────────
    // Publish a report as both the raw string and a per-line model so the page
    // renders it in a virtualised ListView (mirrors set_codec_log).
    fn set_perf_log(ui: &MainWindow, text: &str) {
        let lines: Vec<slint::SharedString> = text.lines().map(|l| l.into()).collect();
        ui.global::<Bridge>()
            .set_perf_test_log_lines(std::rc::Rc::new(slint::VecModel::from(lines)).into());
        ui.global::<Bridge>().set_perf_test_log(text.into());
    }

    ui.global::<Bridge>().on_run_perf_test({
        let ui_weak = ui.as_weak();
        move || {
            let _ = ui_weak.upgrade_in_event_loop(|ui| {
                ui.global::<Bridge>().set_perf_test_running(true);
                set_perf_log(&ui, "Running full codec benchmark…\nThis may take 1-2 minutes.");
            });
            let ui_inner = ui_weak.clone();
            std::thread::spawn(move || {
                let report = crate::codec_perf::run_full_benchmark();
                let _ = ui_inner.upgrade_in_event_loop(move |ui| {
                    set_perf_log(&ui, &report);
                    ui.global::<Bridge>().set_perf_test_running(false);
                });
            });
        }
    });

    ui.global::<Bridge>().on_run_perf_encode_only({
        let ui_weak = ui.as_weak();
        move || {
            let _ = ui_weak.upgrade_in_event_loop(|ui| {
                ui.global::<Bridge>().set_perf_test_running(true);
                set_perf_log(&ui, "Running encode benchmark…");
            });
            let ui_inner = ui_weak.clone();
            std::thread::spawn(move || {
                let report = crate::codec_perf::run_encode_benchmarks();
                let _ = ui_inner.upgrade_in_event_loop(move |ui| {
                    set_perf_log(&ui, &report);
                    ui.global::<Bridge>().set_perf_test_running(false);
                });
            });
        }
    });

    ui.global::<Bridge>().on_run_perf_decode_only({
        let ui_weak = ui.as_weak();
        move || {
            let _ = ui_weak.upgrade_in_event_loop(|ui| {
                ui.global::<Bridge>().set_perf_test_running(true);
                set_perf_log(&ui, "Running decode benchmark…");
            });
            let ui_inner = ui_weak.clone();
            std::thread::spawn(move || {
                let report = crate::codec_perf::run_decode_benchmarks();
                let _ = ui_inner.upgrade_in_event_loop(move |ui| {
                    set_perf_log(&ui, &report);
                    ui.global::<Bridge>().set_perf_test_running(false);
                });
            });
        }
    });

    ui.global::<Bridge>().on_run_perf_list_factories({
        let ui_weak = ui.as_weak();
        move || {
            let _ = ui_weak.upgrade_in_event_loop(|ui| {
                ui.global::<Bridge>().set_perf_test_running(true);
                set_perf_log(&ui, "Listing GStreamer codec factories…");
            });
            let ui_inner = ui_weak.clone();
            std::thread::spawn(move || {
                let report = crate::codec_perf::list_codec_factories();
                let _ = ui_inner.upgrade_in_event_loop(move |ui| {
                    set_perf_log(&ui, &report);
                    ui.global::<Bridge>().set_perf_test_running(false);
                });
            });
        }
    });
```

> If you keep the research's plain-string version instead (no line model), drop
> the `set_perf_log` helper and the `perf-test-log-lines` property (Step 2) and
> revert the page (Step 3) to a `ScrollView`+`Text` — but you'll reintroduce the
> scroll-freeze on long reports.

---

← [Step 7](step-7-lib-rs-register.md) · [Index](README.md) · Next → [Step 9](step-9-verification.md)
