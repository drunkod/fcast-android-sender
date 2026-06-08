# Step 6 — `src/android_main.rs` (edit)

← [Step 5](step-5-mod-rs.md) · [Index](README.md) · Next → [Step 7](step-7-verification.md)

Insert after the existing `on_pick_test_overlay_image` handler block (closes around
**line 1199**, just before `use crate::jni_bridge::camera::{ … };` at line 1201).

Each handler: flips `codec-test-running` true + a placeholder log on the UI thread,
then does the **blocking** JNI work on a `std::thread::spawn` (not `tokio::spawn` —
`attach_current_thread` + `call_static_method` block), then posts the report back via
`upgrade_in_event_loop`.

```rust
    // ── Codec test callbacks ─────────────────────────────────────────────
    ui.global::<Bridge>().on_run_codec_test({
        let ui_weak = ui.as_weak();
        move || {
            let _ = ui_weak.upgrade_in_event_loop(|ui| {
                ui.global::<Bridge>().set_codec_test_running(true);
                ui.global::<Bridge>()
                    .set_codec_test_log("Running full codec test…\n".into());
            });
            let ui_inner = ui_weak.clone();
            std::thread::spawn(move || {
                let mut report = String::new();

                report.push_str("===== FULL CODEC DUMP =====\n");
                match crate::jni_bridge::codec_test::run_codec_dump_all() {
                    Ok(r) => report.push_str(&r),
                    Err(e) => report.push_str(&format!("FAIL dump: {e}\n")),
                }

                report.push_str("\n===== QUICK FIND =====\n");
                match crate::jni_bridge::codec_test::run_codec_quick_find() {
                    Ok(r) => report.push_str(&r),
                    Err(e) => report.push_str(&format!("FAIL quick-find: {e}\n")),
                }

                report.push_str("\n===== ENCODER SMOKE TEST =====\n");
                match crate::jni_bridge::codec_test::run_codec_smoke_test() {
                    Ok(r) => report.push_str(&r),
                    Err(e) => report.push_str(&format!("FAIL smoke: {e}\n")),
                }

                let _ = ui_inner.upgrade_in_event_loop(move |ui| {
                    ui.global::<Bridge>().set_codec_test_log(report.into());
                    ui.global::<Bridge>().set_codec_test_running(false);
                });
            });
        }
    });

    ui.global::<Bridge>().on_run_codec_dump_only({
        let ui_weak = ui.as_weak();
        move || {
            let _ = ui_weak.upgrade_in_event_loop(|ui| {
                ui.global::<Bridge>().set_codec_test_running(true);
                ui.global::<Bridge>()
                    .set_codec_test_log("Dumping codecs…\n".into());
            });
            let ui_inner = ui_weak.clone();
            std::thread::spawn(move || {
                let report = match crate::jni_bridge::codec_test::run_codec_dump_all() {
                    Ok(r) => r,
                    Err(e) => format!("FAIL: {e}\n"),
                };
                let _ = ui_inner.upgrade_in_event_loop(move |ui| {
                    ui.global::<Bridge>().set_codec_test_log(report.into());
                    ui.global::<Bridge>().set_codec_test_running(false);
                });
            });
        }
    });

    ui.global::<Bridge>().on_run_codec_smoke_only({
        let ui_weak = ui.as_weak();
        move || {
            let _ = ui_weak.upgrade_in_event_loop(|ui| {
                ui.global::<Bridge>().set_codec_test_running(true);
                ui.global::<Bridge>()
                    .set_codec_test_log("Running encoder smoke test…\n".into());
            });
            let ui_inner = ui_weak.clone();
            std::thread::spawn(move || {
                let report = match crate::jni_bridge::codec_test::run_codec_smoke_test() {
                    Ok(r) => r,
                    Err(e) => format!("FAIL: {e}\n"),
                };
                let _ = ui_inner.upgrade_in_event_loop(move |ui| {
                    ui.global::<Bridge>().set_codec_test_log(report.into());
                    ui.global::<Bridge>().set_codec_test_running(false);
                });
            });
        }
    });
```

---

← [Step 5](step-5-mod-rs.md) · [Index](README.md) · Next → [Step 7](step-7-verification.md)
