//! Android entry point. Called by slint_android via JNI bootstrap.
//! Extracted from src/lib.rs as part of refactor step 07.7.

use parking_lot::Mutex;
use slint::ComponentHandle;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing::{debug, error};

use crate::*;
use crate::application::Application;
use crate::application::defaults::{default_presets, default_quick_actions};
use crate::command::http_runner::start_migrated_command_server;
use crate::command::legacy_tests::{
    log_ui_test_status, run_graph_smoke_test, run_legacy_http_crossfade_test,
    run_legacy_http_getinfo_test,
};
use crate::command::legacy_tests::migration_test_log_name;
use crate::jni_bridge::helpers::{
    call_java_method_no_args, handle_back_request, init_vm, resolve_android_files_dir, JavaMethod,
};
use crate::platform::panel_stack::PanelStack;
use crate::platform::platform_app::{spawn_recording_ticker, PlatformApp, RecordingTickerState};

const LEGACY_COMMAND_BIND_ADDR: &str = "0.0.0.0:8080";

use gstpop_runtime::GstPopClient;
use once_cell::sync::Lazy;
use std::sync::atomic::AtomicBool;

/// Embedded gst-pop daemon WebSocket port for the camera RTMP path.
const CAM_RTMP_GSTPOP_PORT: u16 = 9000;

struct CamRtmpGstPopSession {
    pid: String,
    client: Arc<GstPopClient>,
    active: Arc<AtomicBool>,
}

static CAM_RTMP_GSTPOP_SESSION: Lazy<Mutex<Option<CamRtmpGstPopSession>>> =
    Lazy::new(|| Mutex::new(None));

// TODO: handle errs
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: PlatformApp) {
    let ptr = app.vm_as_ptr() as *mut jni::sys::JavaVM;
    assert!(!ptr.is_null(), "JavaVM ptr is null");
    // SAFETY: PlatformApp is provided by Slint's Android bootstrap and exposes
    // the process JavaVM pointer while android_main is starting.
    let vm = unsafe {
        jni::JavaVM::from_raw(ptr).unwrap()
    };
    let vm = init_vm(vm);
    crate::app::init(crate::app::App::production(vm.clone()));
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Debug),
    );

    let app_clone = app.clone();

    if let Ok(files_dir) = resolve_android_files_dir(&app_clone) {
        if let Err(e) = crate::config::migration::migrate_config_file(&files_dir) {
            tracing::warn!("migration config migrate failed: {e}");
        }
    }

    slint::android::init(app).unwrap();

    let ui = MainWindow::new().unwrap();
    *ANDROID_UI.lock() = Some(ui.as_weak());
    *ANDROID_APP.lock() = Some(app_clone.clone());

    // Cached snapshot. Re-pushed in full whenever any signal changes.
    #[derive(Clone, Default)]
    struct StatusSnapshot {
        network_label: String,
        thermal_label: String,
        battery_pct: i32,
        charging: bool,
    }

    fn push_status(ui_handle: slint::Weak<MainWindow>, snap: StatusSnapshot) {
        let _ = ui_handle.upgrade_in_event_loop(move |ui| {
            let bridge = ui.global::<Bridge>();
            let items = vec![
                StatusItem {
                    label: "network".into(),
                    value: snap.network_label.into(),
                    severity: StatusSeverity::Info,
                    icon_glyph: "📶".into(),
                },
                StatusItem {
                    label: "thermal".into(),
                    value: snap.thermal_label.clone().into(),
                    severity: match snap.thermal_label.as_str() {
                        "Critical" => StatusSeverity::Error,
                        "Serious" => StatusSeverity::Warning,
                        _ => StatusSeverity::Info,
                    },
                    icon_glyph: if snap.thermal_label == "Critical" {
                        "🔥".into()
                    } else {
                        "🌡".into()
                    },
                },
                StatusItem {
                    label: "battery".into(),
                    value: format!("{}%", snap.battery_pct).into(),
                    severity: if snap.battery_pct < 20 {
                        StatusSeverity::Error
                    } else {
                        StatusSeverity::Info
                    },
                    icon_glyph: if snap.charging {
                        "⚡".into()
                    } else {
                        "🔋".into()
                    },
                },
            ];
            let model: slint::ModelRc<StatusItem> =
                std::rc::Rc::new(slint::VecModel::from(items)).into();
            bridge.set_status_items(model);
        });
    }

    let show_debug = cfg!(debug_assertions);
    ui.global::<Bridge>().set_show_debug(show_debug);
    ui.global::<Bridge>()
        .set_app_version(env!("CARGO_PKG_VERSION").into());

    let bar_actions: Arc<Mutex<Vec<QuickAction>>> = Arc::new(Mutex::new(default_quick_actions()));
    // Initial push is synchronous — we still hold the strong `ui` handle,
    // so the control bar is populated before `ui.run()` paints the first
    // frame. Subsequent mutations from callbacks use `push_bar()` which
    // hops through `upgrade_in_event_loop` because they only have a weak.
    {
        let snapshot = bar_actions.lock().clone();
        ui.global::<Bridge>()
            .set_quick_actions(std::rc::Rc::new(slint::VecModel::from(snapshot)).into());
    }
    let push_bar = {
        let bar_actions = bar_actions.clone();
        let ui_weak = ui.as_weak();
        move || {
            let snapshot = bar_actions.lock().clone();
            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                ui.global::<Bridge>()
                    .set_quick_actions(std::rc::Rc::new(slint::VecModel::from(snapshot)).into());
            });
        }
    };

    ui.global::<Bridge>().on_move_bar_action({
        let bar_actions = bar_actions.clone();
        let push = push_bar.clone();
        move |from, to| {
            let mut g = bar_actions.lock();
            if let (Ok(from_u), Ok(to_u)) = (usize::try_from(from), usize::try_from(to)) {
                if from_u < g.len() && to_u < g.len() && from_u != to_u {
                    let item = g.remove(from_u);
                    g.insert(to_u, item);
                }
            }
            drop(g);
            push();
        }
    });

    ui.global::<Bridge>().on_set_bar_action_enabled({
        let bar_actions = bar_actions.clone();
        let push = push_bar.clone();
        move |idx, enabled| {
            let mut g = bar_actions.lock();
            if let Ok(i) = usize::try_from(idx) {
                if let Some(a) = g.get_mut(i) {
                    a.enabled = enabled;
                }
            }
            drop(g);
            push();
        }
    });

    ui.global::<Bridge>().on_save_bar_actions({
        let _bar_actions = bar_actions.clone();
        let push = push_bar.clone();
        move || {
            // Phase 11: persist to DataStore via JNI here.
            // For now, just re-push the in-memory state.
            push();
        }
    });

    let history: Arc<Mutex<Vec<crate::CastHistoryEntry>>> = Arc::new(Mutex::new(vec![
        crate::CastHistoryEntry {
            id: "h1".into(),
            receiver: "Living Room TV".into(),
            started_at: "Today 12:34".into(),
            duration_s: 765,
            status: "Completed".into(),
        },
        crate::CastHistoryEntry {
            id: "h2".into(),
            receiver: "Bedroom TV".into(),
            started_at: "Yesterday 22:10".into(),
            duration_s: 68,
            status: "Cancelled".into(),
        },
        crate::CastHistoryEntry {
            id: "h3".into(),
            receiver: "Office Mac".into(),
            started_at: "Yesterday 09:00".into(),
            duration_s: 1920,
            status: "Completed".into(),
        },
    ]));

    use slint::Model;
    use std::sync::atomic::AtomicUsize;
    let macros: Arc<std::sync::Mutex<Vec<Macro>>> = Arc::new(std::sync::Mutex::new(vec![]));
    let draft_macro_steps: Arc<std::sync::Mutex<Vec<MacroStep>>> =
        Arc::new(std::sync::Mutex::new(vec![]));
    let next_macro_id = Arc::new(AtomicUsize::new(0));

    // Both push_* helpers apply synchronously via `upgrade()` rather than
    // `upgrade_in_event_loop()`. Every caller is a Slint callback (e.g.
    // on_save_macro, on_draft_move_step) that already runs on the UI
    // thread, so deferral is unnecessary — and would let the consumer
    // page render one frame with stale data when a panel switch happens
    // immediately after the callback (see on_load_draft_macro for the
    // same rationale).
    let push_macros = {
        let macros = macros.clone();
        let ui_weak = ui.as_weak();
        move || {
            let snap = macros.lock().unwrap().clone();
            if let Some(ui) = ui_weak.upgrade() {
                ui.global::<Bridge>()
                    .set_macros(std::rc::Rc::new(slint::VecModel::from(snap)).into());
            }
        }
    };
    push_macros();

    let push_draft_steps = {
        let draft_macro_steps = draft_macro_steps.clone();
        let ui_weak = ui.as_weak();
        move || {
            let snap = draft_macro_steps.lock().unwrap().clone();
            if let Some(ui) = ui_weak.upgrade() {
                ui.global::<Bridge>()
                    .set_draft_macro_steps(std::rc::Rc::new(slint::VecModel::from(snap)).into());
            }
        }
    };
    push_draft_steps();

    ui.global::<Bridge>().on_save_macro({
        let macros = macros.clone();
        let next_id = next_macro_id.clone();
        let push = push_macros.clone();
        move |id, name, steps, enabled| {
            let steps_vec: Vec<MacroStep> = steps.iter().collect();
            let mut g = macros.lock().unwrap();
            if id.is_empty() {
                let new_id = format!("macro-{}", next_id.fetch_add(1, Ordering::Relaxed));
                g.push(Macro {
                    id: new_id.into(),
                    name: name.into(),
                    steps: std::rc::Rc::new(slint::VecModel::from(steps_vec)).into(),
                    enabled,
                });
            } else if let Some(m) = g.iter_mut().find(|m| m.id == id) {
                m.name = name.into();
                m.enabled = enabled;
                m.steps = std::rc::Rc::new(slint::VecModel::from(steps_vec)).into();
            }
            drop(g);
            push();
        }
    });

    ui.global::<Bridge>().on_delete_macro({
        let macros = macros.clone();
        let push = push_macros.clone();
        move |id| {
            macros.lock().unwrap().retain(|m| m.id != id);
            push();
        }
    });

    ui.global::<Bridge>().on_run_macro({
        let macros = macros.clone();
        let ui_weak = ui.as_weak();
        move |id| {
            let snap = macros.lock().unwrap().iter().find(|m| m.id == id).cloned();
            let Some(m) = snap else {
                Application::flash_banner(
                    ui_weak.clone(),
                    format!("Macro {} not found", id),
                    BannerSeverity::Error,
                    std::time::Duration::from_secs(3),
                );
                return;
            };
            // Phase 11: real macro engine (iterate m.steps, dispatch each via on_invoke_action).
            Application::flash_banner(
                ui_weak.clone(),
                format!("Ran macro: {}", m.name),
                BannerSeverity::Success,
                std::time::Duration::from_secs(2),
            );
        }
    });

    ui.global::<Bridge>().on_load_draft_macro({
        let macros = macros.clone();
        let draft_macro_steps = draft_macro_steps.clone();
        let ui_weak = ui.as_weak();
        move |id| {
            let mut draft_name = "".to_string();
            let mut draft_enabled = true;
            let steps_snap: Vec<MacroStep> = {
                let mut draft_g = draft_macro_steps.lock().unwrap();
                if id.is_empty() {
                    draft_g.clear();
                } else {
                    let mg = macros.lock().unwrap();
                    if let Some(m) = mg.iter().find(|m| m.id == id) {
                        *draft_g = m.steps.iter().collect();
                        draft_name = m.name.to_string();
                        draft_enabled = m.enabled;
                    } else {
                        draft_g.clear();
                    }
                }
                draft_g.clone()
            };
            // Slint callbacks run on the UI thread, so we can apply the
            // draft state synchronously. This matters because callers
            // (macros_page.slint) switch to Panel.macro-edit immediately
            // after this callback returns — a deferred upgrade_in_event_loop
            // would let MacroEditPage render one frame with stale values.
            if let Some(ui) = ui_weak.upgrade() {
                let bridge = ui.global::<Bridge>();
                bridge.set_draft_macro_name(draft_name.into());
                bridge.set_draft_macro_enabled(draft_enabled);
                bridge.set_draft_macro_steps(
                    std::rc::Rc::new(slint::VecModel::from(steps_snap)).into(),
                );
            }
        }
    });

    ui.global::<Bridge>().on_draft_add_step({
        let draft_macro_steps = draft_macro_steps.clone();
        let push = push_draft_steps.clone();
        move |kind| {
            let mut g = draft_macro_steps.lock().unwrap();
            let label = match kind {
                QuickActionKind::ScanQr => "Scan QR",
                QuickActionKind::OpenAudio => "Open Audio",
                QuickActionKind::OpenCamera => "Open Camera",
                QuickActionKind::StartRecord => "Start Recording",
                QuickActionKind::StopRecord => "Stop Recording",
                QuickActionKind::StopCast => "Stop Cast",
                _ => "",
            };
            g.push(MacroStep {
                kind,
                label: label.into(),
            });
            drop(g);
            push();
        }
    });

    ui.global::<Bridge>().on_draft_remove_step({
        let draft_macro_steps = draft_macro_steps.clone();
        let push = push_draft_steps.clone();
        move |idx| {
            let mut g = draft_macro_steps.lock().unwrap();
            if let Ok(i) = usize::try_from(idx) {
                if i < g.len() {
                    g.remove(i);
                }
            }
            drop(g);
            push();
        }
    });

    ui.global::<Bridge>().on_draft_move_step({
        let draft_macro_steps = draft_macro_steps.clone();
        let push = push_draft_steps.clone();
        move |from, to| {
            let mut g = draft_macro_steps.lock().unwrap();
            if let (Ok(from_u), Ok(to_u)) = (usize::try_from(from), usize::try_from(to)) {
                if from_u < g.len() && to_u < g.len() && from_u != to_u {
                    let s = g.remove(from_u);
                    g.insert(to_u, s);
                }
            }
            drop(g);
            push();
        }
    });

    let push_history = {
        let history = history.clone();
        let ui_weak = ui.as_weak();
        move || {
            let snap = history.lock().clone();
            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                ui.global::<Bridge>()
                    .set_history(std::rc::Rc::new(slint::VecModel::from(snap)).into());
            });
        }
    };
    push_history();

    // ── Bitrate presets (Phase 8 / Cluster C1) — data + pusher ──────────
    // Created here (above the D1 handlers) so `on_reset_settings` can
    // restore the factory presets list. The C1 callback registrations
    // (save / delete / set-active) live further down and capture these
    // same handles by clone.
    //
    // The factory-default literal lives in `default_presets()` so init
    // and reset share a single source of truth — same pattern as
    // `default_quick_actions()`.
    let presets: Arc<Mutex<Vec<BitratePreset>>> = Arc::new(Mutex::new(default_presets()));
    // Monotonic id source for user-created presets. Never use `g.len()`:
    // after a delete-then-add cycle len() can collide with a previously
    // issued id.
    let next_preset_id: Arc<AtomicU64> = Arc::new(AtomicU64::new(0));
    let push_presets = {
        let presets = presets.clone();
        let ui_weak = ui.as_weak();
        move || {
            let snapshot = presets.lock().clone();
            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                ui.global::<Bridge>()
                    .set_presets(std::rc::Rc::new(slint::VecModel::from(snapshot)).into());
            });
        }
    };
    push_presets();

    // Create the tokio runtime *before* registering Slint callbacks that
    // call `tokio::spawn` (directly or via `Application::flash_banner`).
    // Slint callbacks run on the UI thread during `ui.run()`, which has no
    // tokio context by default — `tokio::spawn` would panic with "there is
    // no reactor running". The `_runtime_guard` registers this thread as a
    // runtime context for the lifetime of the guard. It MUST be dropped
    // before `runtime.block_on(...)` later in this function, otherwise
    // `block_on` panics ("Cannot start a runtime from within a runtime").
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _runtime_guard = runtime.enter();

    let files_dir = resolve_android_files_dir(&app_clone).unwrap_or_else(|err| {
        error!(
            ?err,
            "Failed to resolve Android files dir for backend settings"
        );
        std::env::temp_dir()
    });
    crate::config::init(files_dir.clone());
    let backend_lifecycle =
        std::sync::Arc::new(backend::lifecycle::BackendLifecycle::new(files_dir));
    backend_lifecycle.register(&ui);

    // Hydrate the Bridge from saved CameraRtmpConfig on startup
    let cfg = crate::config::load().camera_rtmp.unwrap_or_default();
    let b = ui.global::<Bridge>();
    b.set_cam_rtmp_url(cfg.url.into());
    b.set_cam_rtmp_camera_idx(cfg.camera_idx as i32);
    b.set_cam_rtmp_resolution_idx(cfg.resolution_idx as i32);
    b.set_cam_rtmp_framerate_idx(cfg.framerate_idx as i32);
    b.set_cam_rtmp_mirror(cfg.mirror);
    b.set_cam_rtmp_stabilization(cfg.stabilization);
    b.set_cam_rtmp_zoom(cfg.zoom.max(0.5));

    if let Ok(Some(key)) = crate::secret::load("cam_rtmp_stream_key") {
        b.set_cam_rtmp_stream_key(key.into());
    }

    // ── Phase 8 / Cluster D1 — Backup / reset handlers ──────────────────
    ui.global::<Bridge>().on_export_settings({
        let ui_weak = ui.as_weak();
        move || {
            let ui_weak = ui_weak.clone();
            tokio::spawn(async move {
                // Phase 11: ACTION_CREATE_DOCUMENT via JNI; serialise + write.
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                Application::flash_banner(
                    ui_weak,
                    "Settings exported (placeholder).".into(),
                    BannerSeverity::Success,
                    std::time::Duration::from_secs(3),
                );
            });
        }
    });

    ui.global::<Bridge>().on_import_settings({
        let ui_weak = ui.as_weak();
        move || {
            let ui_weak = ui_weak.clone();
            tokio::spawn(async move {
                // Phase 11: ACTION_OPEN_DOCUMENT, parse JSON, write to DataStore.
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                Application::flash_banner(
                    ui_weak,
                    "Settings imported (placeholder).".into(),
                    BannerSeverity::Success,
                    std::time::Duration::from_secs(3),
                );
            });
        }
    });

    ui.global::<Bridge>().on_reset_settings({
        let bar_actions = bar_actions.clone();
        let history = history.clone();
        let presets = presets.clone();
        let next_preset_id = next_preset_id.clone();
        let macros = macros.clone();
        let next_macro_id = next_macro_id.clone();
        let push_bar = push_bar.clone();
        let push_history = push_history.clone();
        let push_presets = push_presets.clone();
        let push_macros = push_macros.clone();
        let ui_weak = ui.as_weak();
        move || {
            // Reset every Cluster-C/D model owned by Rust to factory
            // defaults.
            //
            // `next_preset_id` / `next_macro_id` are also rewound so
            // user-created ids restart at `custom-0` / `macro-0` after
            // a reset, matching the factory state. Without this, a
            // freshly-reset device would still hand out `custom-N` /
            // `macro-N` for some N > 0 the moment the user added an
            // entry.
            *bar_actions.lock() = default_quick_actions();
            *presets.lock() = default_presets();
            next_preset_id.store(0, Ordering::Relaxed);
            history.lock().clear();
            macros.lock().unwrap().clear();
            next_macro_id.store(0, Ordering::Relaxed);

            push_bar();
            push_presets();
            push_history();
            push_macros();

            // Phase 11: also clear DataStore / SharedPreferences via JNI.

            Application::flash_banner(
                ui_weak.clone(),
                "Settings reset to defaults".into(),
                BannerSeverity::Success,
                std::time::Duration::from_secs(3),
            );
        }
    });

    ui.global::<Bridge>().on_clear_cast_history({
        let history = history.clone();
        let push_history = push_history.clone();
        let ui_weak = ui.as_weak();
        move || {
            history.lock().clear();
            push_history();

            Application::flash_banner(
                ui_weak.clone(),
                "Cast history cleared".into(),
                BannerSeverity::Success,
                std::time::Duration::from_secs(2),
            );
        }
    });

    ui.global::<Bridge>().on_clear_known_receivers({
        let ui_weak = ui.as_weak();
        move || {
            // Phase 11: clear known-receivers DataStore. For now, announce.
            Application::flash_banner(
                ui_weak.clone(),
                "Known receivers cleared".into(),
                BannerSeverity::Success,
                std::time::Duration::from_secs(2),
            );
        }
    });

    // ── Phase 8 / Cluster D2 — Cast history handlers ────────────────────
    ui.global::<Bridge>().on_clear_history({
        let history = history.clone();
        let push_history = push_history.clone();
        move || {
            history.lock().clear();
            push_history();
        }
    });

    ui.global::<Bridge>().on_delete_history_entry({
        let history = history.clone();
        let push_history = push_history.clone();
        move |id| {
            let id = id.to_string();
            history.lock().retain(|e| e.id != id);
            push_history();
        }
    });

    ui.global::<Bridge>().on_recast({
        let history = history.clone();
        let ui_weak = ui.as_weak();
        move |id| {
            let id = id.to_string();
            let entry_opt = history.lock().iter().find(|e| e.id == id).cloned();
            let Some(entry) = entry_opt else {
                return;
            };
            // Phase 11: trigger reconnection + start_casting with the same receiver.
            Application::flash_banner(
                ui_weak.clone(),
                format!("Recasting to {}", entry.receiver),
                BannerSeverity::Info,
                std::time::Duration::from_secs(2),
            );
        }
    });

    // Push selected-history-entry when a row is tapped. Uses the explicit
    // open-history-detail callback — no `changed` re-emit needed.
    //
    // Called synchronously (Slint UI thread) so the detail page always
    // renders with fresh data on the same frame it becomes visible.
    ui.global::<Bridge>().on_open_history_detail({
        let history = history.clone();
        let ui_weak = ui.as_weak();
        move |entry_id: slint::SharedString| {
            let id = entry_id.to_string();
            let entry = history.lock().iter().find(|e| e.id == id).cloned();
            let Some(entry) = entry else {
                return;
            };
            if let Some(ui) = ui_weak.upgrade() {
                ui.global::<Bridge>().set_selected_history_id(entry_id);
                ui.global::<Bridge>().set_selected_history_entry(entry);
            }
        }
    });

    let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();

    let ui_handle = ui.as_weak();
    // NOTE: The shared `Arc<Mutex<StatusSnapshot>>` cache that producers
    // (battery / thermal / network listeners) will update lands with
    // Cluster B (Phase 8 Section 3). For now the ticker just rebuilds a
    // hardcoded snapshot on every tick — no shared state needed yet.
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(5));
        loop {
            tick.tick().await;
            let snap_now = StatusSnapshot {
                network_label: "Wi-Fi".into(),
                thermal_label: "Nominal".into(),
                battery_pct: 87,
                charging: false,
            };
            push_status(ui_handle.clone(), snap_now);
        }
    });
    fn enumerate_interfaces() -> Vec<NetworkInterface> {
        vec![
            NetworkInterface {
                name: "wlan0".into(),
                kind: NetworkKind::Wifi,
                address_v4: "192.168.1.42".into(),
                address_v6: "fe80::1234".into(),
                enabled: true,
            },
            NetworkInterface {
                name: "rmnet0".into(),
                kind: NetworkKind::Cellular,
                address_v4: "10.20.30.40".into(),
                address_v6: "".into(),
                enabled: false,
            },
            NetworkInterface {
                name: "lo".into(),
                kind: NetworkKind::Loopback,
                address_v4: "127.0.0.1".into(),
                address_v6: "::1".into(),
                enabled: true,
            },
        ]
    }
    fn push_interfaces(ui_handle: slint::Weak<MainWindow>, list: Vec<NetworkInterface>) {
        let _ = ui_handle.upgrade_in_event_loop(move |ui| {
            let model: slint::ModelRc<NetworkInterface> =
                std::rc::Rc::new(slint::VecModel::from(list)).into();
            ui.global::<Bridge>().set_network_interfaces(model);
        });
    }
    push_interfaces(ui.as_weak(), enumerate_interfaces());
    let interfaces = std::sync::Arc::new(tokio::sync::Mutex::new(enumerate_interfaces()));
    let interfaces_for_callback = interfaces.clone();
    let ui_for_callback = ui.as_weak();
    ui.global::<Bridge>()
        .on_set_interface_enabled(move |name, value| {
            let interfaces = interfaces_for_callback.clone();
            let ui_handle = ui_for_callback.clone();
            tokio::spawn(async move {
                let mut list = interfaces.lock().await;
                if let Some(iface) = list.iter_mut().find(|i| i.name == name.as_str()) {
                    iface.enabled = value;
                }
                push_interfaces(ui_handle, list.clone());
            });
        });
    let log_ring = log_ring::LogRing::new(ui.as_weak());
    let log_ring_for_clear = log_ring.clone();
    use tracing_subscriber::filter::LevelFilter;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::Layer;
    // Cap the LogRing layer at DEBUG so the firehose of GStreamer `Fixme`
    // / TRACE events forwarded by `tracing_gstreamer::integrate_events`
    // (see `Application::run_event_loop`) never reaches the ring buffer.
    // Without this filter, an active media pipeline can produce thousands
    // of TRACE events per second, each one mutating the ring and dirtying
    // the UI pusher — pointless for a human-readable debug log.
    //
    // `try_init` (not `init`) so re-entries of `android_main` (Android can
    // trigger on activity destroy/recreate) don't panic from
    // `set_global_default()` being called twice. Mirrors `init_once` above.
    if let Err(err) = tracing_subscriber::registry()
        .with(log_ring.clone().with_filter(LevelFilter::DEBUG))
        .try_init()
    {
        debug!(
            ?err,
            "tracing subscriber already initialised — re-entry of android_main"
        );
    }
    ui.global::<Bridge>().on_clear_log_entries(move || {
        log_ring_for_clear.clear();
    });
    // ── Bitrate presets (Phase 8 / Cluster C1) — callbacks ──────────────
    // The shared `presets` Arc + `push_presets` closure are declared
    // above (next to `history`) so the D1 `on_reset_settings` handler
    // can also restore the factory list.
    ui.global::<Bridge>().on_save_preset({
        let presets = presets.clone();
        let next_id = next_preset_id.clone();
        let push = push_presets.clone();
        move |id, name, kbps| {
            let mut g = presets.lock();
            if id.is_empty() {
                let new_id = format!("custom-{}", next_id.fetch_add(1, Ordering::Relaxed));
                g.push(BitratePreset {
                    id: new_id.into(),
                    name: name.into(),
                    bitrate_kbps: kbps,
                    active: false,
                });
            } else if let Some(p) = g.iter_mut().find(|p| p.id == id) {
                p.name = name.into();
                p.bitrate_kbps = kbps;
            }
            drop(g);
            push();
        }
    });
    ui.global::<Bridge>().on_delete_preset({
        let presets = presets.clone();
        let push = push_presets.clone();
        move |id| {
            let mut g = presets.lock();
            g.retain(|p| p.id != id);
            // If the deleted preset was the active one, promote the first
            // remaining preset to active so the user is never left without
            // a selection.
            if !g.iter().any(|p| p.active) {
                if let Some(first) = g.first_mut() {
                    first.active = true;
                }
            }
            drop(g);
            push();
        }
    });
    ui.global::<Bridge>().on_set_active_preset({
        let presets = presets.clone();
        let push = push_presets.clone();
        move |id| {
            let mut g = presets.lock();
            for p in g.iter_mut() {
                p.active = p.id == id;
            }
            drop(g);
            push();
        }
    });

    ui.global::<Bridge>().on_connect_receiver({
        let event_tx = event_tx.clone();
        move |device_name| {
            event_tx
                .send(Event::ConnectToDevice(device_name.to_string()))
                .unwrap();
        }
    });

    let panel_stack = std::rc::Rc::new(PanelStack::new());

    ui.global::<PanelBridge>().on_push({
        let stack = panel_stack.clone();
        let ui_weak = ui.as_weak();
        move |p: Panel| {
            let Some(ui) = ui_weak.upgrade() else { return };
            let pb = ui.global::<PanelBridge>();
            let current = pb.get_active();
            if current == p {
                return;
            }
            stack.push_panel(current);
            pb.set_active(p);
            pb.set_stack(stack.as_model());
        }
    });

    ui.global::<PanelBridge>().on_pop({
        let stack = panel_stack.clone();
        let ui_weak = ui.as_weak();
        move || {
            let Some(ui) = ui_weak.upgrade() else { return };
            let pb = ui.global::<PanelBridge>();
            pb.set_active(stack.pop_panel());
            pb.set_stack(stack.as_model());
        }
    });

    ui.global::<PanelBridge>().on_replace({
        let ui_weak = ui.as_weak();
        move |p: Panel| {
            if let Some(ui) = ui_weak.upgrade() {
                ui.global::<PanelBridge>().set_active(p);
            }
        }
    });

    ui.global::<PanelBridge>().on_close_all({
        let stack = panel_stack.clone();
        let ui_weak = ui.as_weak();
        move || {
            let Some(ui) = ui_weak.upgrade() else { return };
            stack.clear();
            let pb = ui.global::<PanelBridge>();
            pb.set_active(Panel::None);
            pb.set_stack(stack.as_model());
        }
    });

    ui.global::<Bridge>().on_back_requested({
        let ui_weak = ui.as_weak();
        let app_clone = app_clone.clone();
        move || {
            let Some(ui) = ui_weak.upgrade() else {
                return;
            };
            handle_back_request(&ui, Some(&app_clone));
        }
    });

    ui.global::<Bridge>().on_start_casting({
        let event_tx = event_tx.clone();
        move |scale_width: i32, scale_height: i32, max_framerate: i32| {
            event_tx
                .send(Event::StartCast {
                    scale_width: scale_width as u32,
                    scale_height: scale_height as u32,
                    max_framerate: max_framerate as u32,
                })
                .unwrap();
        }
    });

    ui.global::<Bridge>().on_stop_casting({
        let event_tx = event_tx.clone();
        move || {
            event_tx
                .send(Event::EndSession { disconnect: true })
                .unwrap();
        }
    });

    let recorder_state = Arc::new(tokio::sync::Mutex::new(RecordingTickerState::default()));

    ui.global::<Recording>().on_start({
        let recorder_state = recorder_state.clone();
        let ui_handle = ui.as_weak();
        move || {
            let recorder_state = recorder_state.clone();
            let ui_handle = ui_handle.clone();
            tokio::spawn(async move {
                let mut s = recorder_state.lock().await;
                s.started_at = Some(std::time::Instant::now());
                s.paused_for = std::time::Duration::ZERO;
                s.pause_started = None;
                s.state = RecordingState::Recording;
                let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                    ui.global::<Recording>()
                        .set_state(RecordingState::Recording);
                    ui.global::<Recording>().set_elapsed_s(0);
                });
            });
        }
    });

    ui.global::<Recording>().on_pause({
        let recorder_state = recorder_state.clone();
        let ui_handle = ui.as_weak();
        move || {
            let recorder_state = recorder_state.clone();
            let ui_handle = ui_handle.clone();
            tokio::spawn(async move {
                let mut s = recorder_state.lock().await;
                if s.state != RecordingState::Recording {
                    return;
                }
                s.pause_started = Some(std::time::Instant::now());
                s.state = RecordingState::Paused;
                let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                    ui.global::<Recording>().set_state(RecordingState::Paused);
                });
            });
        }
    });

    ui.global::<Recording>().on_resume({
        let recorder_state = recorder_state.clone();
        let ui_handle = ui.as_weak();
        move || {
            let recorder_state = recorder_state.clone();
            let ui_handle = ui_handle.clone();
            tokio::spawn(async move {
                let mut s = recorder_state.lock().await;
                if s.state != RecordingState::Paused {
                    return;
                }
                if let Some(started) = s.pause_started.take() {
                    s.paused_for += started.elapsed();
                }
                s.state = RecordingState::Recording;
                let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                    ui.global::<Recording>()
                        .set_state(RecordingState::Recording);
                });
            });
        }
    });

    ui.global::<Recording>().on_stop({
        let recorder_state = recorder_state.clone();
        let ui_handle = ui.as_weak();
        move || {
            let recorder_state = recorder_state.clone();
            let ui_handle = ui_handle.clone();
            tokio::spawn(async move {
                {
                    let mut s = recorder_state.lock().await;
                    s.state = RecordingState::Finalizing;
                }
                let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                    ui.global::<Recording>()
                        .set_state(RecordingState::Finalizing);
                });

                let mut s = recorder_state.lock().await;
                s.started_at = None;
                s.paused_for = std::time::Duration::ZERO;
                s.pause_started = None;
                s.state = RecordingState::Idle;
                let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                    let rec = ui.global::<Recording>();
                    rec.set_state(RecordingState::Idle);
                    rec.set_elapsed_s(0);
                });
            });
        }
    });

    spawn_recording_ticker(ui.as_weak(), recorder_state.clone());

    ui.global::<Bridge>().on_engage_lock({
        let ui_handle = ui.as_weak();
        move || {
            let _ = ui_handle.upgrade_in_event_loop(|ui| {
                ui.global::<Bridge>()
                    .set_lifecycle(LifecycleMode::LockScreen);
            });
        }
    });

    ui.global::<Bridge>().on_engage_stealth({
        let ui_handle = ui.as_weak();
        move || {
            let _ = ui_handle.upgrade_in_event_loop(|ui| {
                ui.global::<Bridge>().set_lifecycle(LifecycleMode::Stealth);
            });
        }
    });

    ui.global::<Bridge>().on_start_snapshot_countdown({
        let ui_handle = ui.as_weak();
        // Monotonic generation counter. Each new countdown bumps it and
        // captures the new value; the spawned timer only resets lifecycle
        // if its captured generation is still current. This makes any
        // older, still-sleeping timer a no-op when a newer countdown is
        // triggered. Mirrors `Application::banner_generation`.
        static SNAPSHOT_GEN: AtomicU64 = AtomicU64::new(0);
        move |seconds: i32| {
            let ui_handle = ui_handle.clone();
            let gen = SNAPSHOT_GEN.fetch_add(1, Ordering::SeqCst) + 1;
            tokio::spawn(async move {
                let _ = ui_handle.upgrade_in_event_loop(|ui| {
                    ui.global::<Bridge>()
                        .set_lifecycle(LifecycleMode::SnapshotCountdown);
                });
                tokio::time::sleep(std::time::Duration::from_secs(seconds.max(0) as u64)).await;
                // Only reset to Normal if (a) no newer countdown has started
                // (otherwise we'd cut the new one short) and (b) we are still
                // in SnapshotCountdown (otherwise the user cancelled or
                // engaged lock/stealth and we must not clobber their choice).
                if SNAPSHOT_GEN.load(Ordering::SeqCst) != gen {
                    return;
                }
                let _ = ui_handle.upgrade_in_event_loop(|ui| {
                    let bridge = ui.global::<Bridge>();
                    if bridge.get_lifecycle() == LifecycleMode::SnapshotCountdown {
                        bridge.set_lifecycle(LifecycleMode::Normal);
                    }
                });
            });
        }
    });

    ui.global::<Bridge>().on_exit_lifecycle({
        let ui_handle = ui.as_weak();
        move || {
            let _ = ui_handle.upgrade_in_event_loop(|ui| {
                ui.global::<Bridge>().set_lifecycle(LifecycleMode::Normal);
            });
        }
    });

    ui.global::<Bridge>().on_set_wifi_aware({
        let ui_handle = ui.as_weak();
        move |enabled| {
            let ui_handle = ui_handle.clone();
            tokio::spawn(async move {
                let success = true;
                let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                    let bridge = ui.global::<Bridge>();
                    bridge.set_wifi_aware_enabled(enabled && success);
                });

                Application::flash_banner(
                    ui_handle,
                    if enabled {
                        "Wi-Fi Aware enabled (placeholder — no permission requested).".into()
                    } else {
                        "Wi-Fi Aware disabled.".into()
                    },
                    BannerSeverity::Info,
                    std::time::Duration::from_secs(3),
                );
            });
        }
    });

    ui.global::<Bridge>().on_invoke_action({
        let app_clone = app_clone.clone();
        let ui_weak = ui.as_weak();
        move |id| {
            let id_str = id.as_str();
            match id_str {
                "scan-qr" => {
                    call_java_method_no_args(&app_clone, JavaMethod::ScanQr);
                }
                "migrated-server" => {
                    let _ = ui_weak.upgrade_in_event_loop(|ui| {
                        ui.global::<Bridge>()
                            .invoke_start_migration_server(LEGACY_COMMAND_BIND_ADDR.into());
                    });
                }
                "test-getinfo" => {
                    let _ = ui_weak.upgrade_in_event_loop(|ui| {
                        ui.global::<Bridge>()
                            .invoke_run_migration_test("getinfo".into());
                    });
                }
                "test-crossfade" => {
                    let _ = ui_weak.upgrade_in_event_loop(|ui| {
                        ui.global::<Bridge>()
                            .invoke_run_migration_test("crossfade".into());
                    });
                }
                "test-smoke" => {
                    let _ = ui_weak.upgrade_in_event_loop(|ui| {
                        ui.global::<Bridge>()
                            .invoke_run_migration_test("smoke".into());
                    });
                }
                _ => {}
            }
        }
    });

    #[cfg(debug_assertions)]
    {
        ui.global::<Bridge>().on_start_migration_server({
            let ui_weak = ui.as_weak();
            move |bind_addr| {
                let status = match start_migrated_command_server(bind_addr.as_str()) {
                    Ok(message) => format!("PASS {message}"),
                    Err(err) => format!("FAIL {err}"),
                };
                log_ui_test_status("start-migration-server", &status);
                let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                    ui.global::<Bridge>().set_test_status(status.into());
                });
            }
        });

        ui.global::<Bridge>().on_run_migration_test({
            let ui_weak = ui.as_weak();
            move |test_id| {
                let test_id = test_id.to_string();
                let _ = ui_weak.upgrade_in_event_loop({
                    let test_id = test_id.clone();
                    move |ui| {
                        ui.global::<Bridge>().set_test_status(
                            format!("Running migration test '{test_id}'...").into(),
                        );
                    }
                });
                let ui_weak_clone = ui_weak.clone();
                std::thread::spawn(move || {
                    let status = match test_id.as_str() {
                        "getinfo" => run_legacy_http_getinfo_test(LEGACY_COMMAND_BIND_ADDR),
                        "crossfade" => run_legacy_http_crossfade_test(LEGACY_COMMAND_BIND_ADDR),
                        "smoke" => run_graph_smoke_test(),
                        other => format!("FAIL unknown migration-test id: {other}"),
                    };
                    log_ui_test_status(migration_test_log_name(&test_id), &status);
                    let _ = ui_weak_clone.upgrade_in_event_loop(move |ui| {
                        ui.global::<Bridge>().set_test_status(status.into());
                    });
                });
            }
        });

        ui.global::<Bridge>().on_stop_migration_server({
            let ui_weak = ui.as_weak();
            move || {
                let status = match migration_runtime::runtime::shutdown_graph_runtime() {
                    Ok(()) => "PASS migration server stopped".to_string(),
                    Err(err) => format!("FAIL migration server stop: {err}"),
                };
                log_ui_test_status("stop-migration-server", &status);
                let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                    ui.global::<Bridge>().set_test_status(status.into());
                });
            }
        });
    }

    // ── Test functionality callbacks ─────────────────────────────────────
    ui.global::<Bridge>().on_start_test({
        let ui_weak = ui.as_weak();
        move || {
            let _ = ui_weak.upgrade_in_event_loop(|ui| {
                ui.global::<Bridge>().set_test_state(MixerState::Starting);
            });
            // TODO: build and start GStreamer test pipeline.
            // Pattern: see crates/migration-runtime/src/nodes/mixer.rs for compositor pad setup.
            // On success call set_test_state(MixerState::Running).
            // On error call set_test_state(MixerState::Error) + set_test_error_text.
            log::info!("start-test: stub — pipeline not yet implemented");
        }
    });

    ui.global::<Bridge>().on_stop_test({
        let ui_weak = ui.as_weak();
        move || {
            // TODO: stop GStreamer test pipeline and release resources.
            let _ = ui_weak.upgrade_in_event_loop(|ui| {
                ui.global::<Bridge>().set_test_state(MixerState::Idle);
                ui.global::<Bridge>().set_test_error_text("".into());
            });
            log::info!("stop-test: stub — pipeline not yet implemented");
        }
    });

    ui.global::<Bridge>().on_pick_test_overlay_image(|| {
        // TODO(android): launch ACTION_GET_CONTENT intent via JNI, write
        //   result back with set_test_overlay_image_path.
        // TODO(desktop): use rfd::FileDialog to pick a file.
        log::info!("pick-test-overlay-image: stub — file picker not yet implemented");
    });

    use migration_runtime::protocol::{Command, CommandResult, DestinationFamily};
    use crate::jni_bridge::camera::{upcall_start_camera_capture, upcall_stop_camera_capture};

    fn tear_down_migration() {
        for id in ["cam-rtmp-src", "cam-rtmp-dest"] {
            let _ = migration_runtime::runtime::handle_command(Command::Remove { id: id.into() });
        }
    }

    fn fail(ui: &slint::Weak<MainWindow>, msg: impl Into<String>) {
        let msg: slint::SharedString = msg.into().into();
        let _ = ui.upgrade_in_event_loop(move |u| {
            let b = u.global::<Bridge>();
            b.set_cam_rtmp_error_text(msg);
            b.set_cam_rtmp_state(MixerState::Error);
        });
    }

    fn clear_error(ui: &slint::Weak<MainWindow>) {
        let _ = ui.upgrade_in_event_loop(|u| {
            u.global::<Bridge>().set_cam_rtmp_error_text("".into());
        });
    }

    async fn teardown_gstpop_session(session: CamRtmpGstPopSession) {
        session.active.store(false, Ordering::SeqCst);
        crate::FRAME_PAIR.cond.notify_all();
        let _ = session
            .client
            .call("stop", serde_json::json!({ "pipeline_id": session.pid }))
            .await;
        let _ = session
            .client
            .call(
                "remove_pipeline",
                serde_json::json!({ "pipeline_id": session.pid }),
            )
            .await;
    }

    async fn start_camera_rtmp_via_gstpop(
        ui: slint::Weak<MainWindow>,
        cam_idx: u32,
        width: u32,
        height: u32,
        fps: u32,
        full_url: String,
        mirror: bool,
        stab: bool,
        zoom: f32,
    ) {
        use gstpop_runtime::EmbeddedState;

        let status = gstpop_runtime::start_embedded(CAM_RTMP_GSTPOP_PORT).await;
        if !matches!(status.state, EmbeddedState::Running) {
            let msg = format!("gst-pop daemon not running: {:?}", status.last_error);
            return fail(&ui, msg);
        }

        let ws_url = format!("ws://127.0.0.1:{}/", status.port);
        let client = match GstPopClient::connect(&ws_url, None).await {
            Ok(c) => Arc::new(c),
            Err(e) => return fail(&ui, format!("client connect: {e:#}")),
        };

        let sink = migration_runtime::nodes::destination::pick_rtmp_sink();
        let flip = if mirror { "videoflip method=horizontal-flip ! " } else { "" };

        let pipeline_desc = format!(
            "appsrc name=camera-src format=time is-live=true do-timestamp=true \
             caps=video/x-raw,format=I420,width={w},height={h},framerate={fps}/1 \
             ! videoconvert ! {flip}\
             x264enc bitrate=4000 speed-preset=ultrafast tune=zerolatency \
             ! h264parse ! queue ! flvmux streamable=true \
             ! {sink} location={url}",
            w = width, h = height, fps = fps, url = full_url, sink = sink, flip = flip,
        );

        let pid_val = match client
            .call(
                "create_pipeline",
                serde_json::json!({ "description": pipeline_desc }),
            )
            .await
        {
            Ok(val) => val,
            Err(e) => return fail(&ui, format!("create_pipeline: {e:#}")),
        };
        let pid = match pid_val.get("pipeline_id").and_then(|v| v.as_str()) {
            Some(p) => p.to_string(),
            None => return fail(&ui, "create_pipeline response missing pipeline_id".to_string()),
        };

        if let Err(e) = client
            .call("play", serde_json::json!({ "pipeline_id": pid }))
            .await
        {
            let _ = client
                .call(
                    "remove_pipeline",
                    serde_json::json!({ "pipeline_id": pid }),
                )
                .await;
            return fail(&ui, format!("play: {e:#}"));
        }

        // Open Camera2 via the JNI upcall.
        if let Err(e) =
            upcall_start_camera_capture(cam_idx, width, height, fps, mirror, stab, zoom)
        {
            let _ = client
                .call("stop", serde_json::json!({ "pipeline_id": pid }))
                .await;
            let _ = client
                .call(
                    "remove_pipeline",
                    serde_json::json!({ "pipeline_id": pid }),
                )
                .await;
            return fail(&ui, format!("startCameraCapture: {e}"));
        }

        let active_flag = Arc::new(AtomicBool::new(true));
        *CAM_RTMP_GSTPOP_SESSION.lock() = Some(CamRtmpGstPopSession {
            pid: pid.clone(),
            client: client.clone(),
            active: active_flag.clone(),
        });

        let frame_pair = crate::FRAME_PAIR.clone();
        let client_clone = client.clone();
        let pid_clone = pid.clone();

        tokio::spawn(async move {
            loop {
                if !active_flag.load(Ordering::SeqCst) {
                    break;
                }
                let buffer_data = {
                    let mut frame_guard = frame_pair.frame.lock();
                    while (*frame_guard).is_none() {
                        if !active_flag.load(Ordering::SeqCst) {
                            return;
                        }
                        frame_pair
                            .cond
                            .wait_for(&mut frame_guard, std::time::Duration::from_millis(50));
                    }
                    let vframe = (*frame_guard).take().unwrap();
                    let buffer = vframe.buffer().to_owned();
                    let pts_ns = buffer.pts().map(|p| p.nseconds()).unwrap_or(0);
                    let mapped = buffer.map_readable().unwrap();
                    (mapped.to_vec(), pts_ns)
                };

                if let Err(e) = client_clone
                    .push_buffer(&pid_clone, "camera-src", &buffer_data.0, buffer_data.1)
                    .await
                {
                    tracing::error!("gst-pop push_buffer failed: {}", e);
                    break;
                }
            }
        });
    }

    async fn stop_camera_rtmp_via_gstpop(ui: slint::Weak<MainWindow>) {
        let _ = upcall_stop_camera_capture();

        let session = CAM_RTMP_GSTPOP_SESSION.lock().take();
        if let Some(session) = session {
            teardown_gstpop_session(session).await;
        }

        let _ = ui.upgrade_in_event_loop(|u| {
            let b = u.global::<Bridge>();
            b.set_cam_rtmp_state(MixerState::Idle);
            b.set_cam_rtmp_error_text("".into());
        });
    }

    ui.global::<Bridge>().on_url_looks_valid(|url: slint::SharedString| {
        url.starts_with("rtmp://") || url.starts_with("rtmps://")
    });

    ui.global::<Bridge>().on_start_camera_rtmp_stream({
        let ui_weak = ui.as_weak();
        move || {
            let ui = ui_weak.clone();

            let (backend, url, key, cam_idx, res_idx, fps_idx, mirror, stab, zoom) =
                match ui.upgrade() {
                    Some(u) => {
                        let b = u.global::<Bridge>();
                        (
                            b.get_media_backend(),
                            b.get_cam_rtmp_url().to_string(),
                            b.get_cam_rtmp_stream_key().to_string(),
                            b.get_cam_rtmp_camera_idx(),
                            b.get_cam_rtmp_resolution_idx(),
                            b.get_cam_rtmp_framerate_idx(),
                            b.get_cam_rtmp_mirror(),
                            b.get_cam_rtmp_stabilization(),
                            b.get_cam_rtmp_zoom(),
                        )
                    }
                    None => return,
                };

            let url_c = url.clone();
            let key_c = key.clone();
            tokio::spawn(async move {
                let _ = crate::config::update(|c| {
                    c.camera_rtmp = Some(crate::config::CameraRtmpConfig {
                        url: url_c,
                        camera_idx: cam_idx as u32,
                        resolution_idx: res_idx as u32,
                        framerate_idx: fps_idx as u32,
                        mirror,
                        stabilization: stab,
                        zoom,
                    });
                });
                let _ = crate::secret::store("cam_rtmp_stream_key", &key_c);
            });

            clear_error(&ui);
            let _ = ui.upgrade_in_event_loop(|u| {
                u.global::<Bridge>().set_cam_rtmp_state(MixerState::Starting);
            });

            let (width, height) = match res_idx {
                0 => (854, 480),
                1 => (1280, 720),
                3 => (3840, 2160),
                _ => (1920, 1080),
            };
            let fps = match fps_idx {
                0 => 24,
                2 => 60,
                _ => 30,
            };
            let full_url = if key.is_empty() { url } else { format!("{url}/{key}") };

            let ui_clone = ui.clone();
            match backend {
                MediaBackendKind::Migration => {
                    tokio::spawn(async move {
                        // ── Build the GStreamer graph first. It runs idle until frames arrive. ──
                        let commands = vec![
                            Command::CreateCameraSource {
                                id: "cam-rtmp-src".into(),
                                camera_idx: cam_idx as u32,
                                width,
                                height,
                                fps,
                                mirror,
                                stabilization: stab,
                                zoom,
                            },
                            Command::CreateDestination {
                                id: "cam-rtmp-dest".into(),
                                family: DestinationFamily::Rtmp { uri: full_url },
                                audio: false,
                                video: true,
                            },
                            Command::Connect {
                                link_id: "cam-rtmp-link".into(),
                                src_id: "cam-rtmp-src".into(),
                                sink_id: "cam-rtmp-dest".into(),
                                audio: false,
                                video: true,
                                config: None,
                            },
                            Command::Start {
                                id: "cam-rtmp-dest".into(),
                                cue_time: None,
                                end_time: None,
                            },
                            Command::Start {
                                id: "cam-rtmp-src".into(),
                                cue_time: None,
                                end_time: None,
                            },
                        ];
                        for cmd in commands {
                            if let CommandResult::Error(err) =
                                migration_runtime::runtime::handle_command(cmd)
                            {
                                return fail(&ui_clone, err);
                            }
                        }

                        // ── Ask Kotlin to open the camera. ──
                        if let Err(e) = upcall_start_camera_capture(
                            cam_idx as u32,
                            width,
                            height,
                            fps,
                            mirror,
                            stab,
                            zoom,
                        ) {
                            tear_down_migration();
                            return fail(&ui_clone, format!("startCameraCapture: {e}"));
                        }
                    });
                }
                MediaBackendKind::GstPop => {
                    tokio::spawn(async move {
                        start_camera_rtmp_via_gstpop(
                            ui_clone,
                            cam_idx as u32,
                            width,
                            height,
                            fps,
                            full_url,
                            mirror,
                            stab,
                            zoom,
                        )
                        .await;
                    });
                }
            }
        }
    });

    ui.global::<Bridge>().on_stop_camera_rtmp_stream({
        let ui_weak = ui.as_weak();
        move || {
            let ui = ui_weak.clone();
            let backend = match ui.upgrade() {
                Some(u) => u.global::<Bridge>().get_media_backend(),
                None => return,
            };
            let _ = ui.upgrade_in_event_loop(|u| {
                u.global::<Bridge>().set_cam_rtmp_state(MixerState::Stopping);
            });
            match backend {
                MediaBackendKind::Migration => {
                    tokio::spawn(async move {
                        let _ = upcall_stop_camera_capture();
                        tear_down_migration();
                        let _ = ui.upgrade_in_event_loop(|u| {
                            let b = u.global::<Bridge>();
                            b.set_cam_rtmp_state(MixerState::Idle);
                            b.set_cam_rtmp_error_text("".into());
                        });
                    });
                }
                MediaBackendKind::GstPop => {
                    tokio::spawn(async move {
                        stop_camera_rtmp_via_gstpop(ui).await;
                    });
                }
            }
        }
    });

    // ── Listen to camera lifecycle events from Kotlin via JNI ──
    let cam_event_ui_weak = ui.as_weak();
    std::thread::spawn(move || loop {
        match crate::GLOB_CAMERA_EVENT_CHAN.1.recv() {
            Ok(event) => match event {
                crate::jni_bridge::main_activity::CameraEvent::Started { width, height } => {
                    tracing::info!("camera capture started {width}x{height}");
                    let _ = cam_event_ui_weak.upgrade_in_event_loop(|u| {
                        u.global::<Bridge>().set_cam_rtmp_state(MixerState::Running);
                    });
                }
                crate::jni_bridge::main_activity::CameraEvent::Stopped => {
                    tracing::info!("camera capture stopped");
                }
                crate::jni_bridge::main_activity::CameraEvent::Failed { reason } => {
                    tracing::warn!("camera capture failed: {reason}");
                    
                    let backend = cam_event_ui_weak.upgrade()
                        .map(|u| u.global::<Bridge>().get_media_backend());
                    
                    match backend {
                        Some(MediaBackendKind::Migration) => {
                            tear_down_migration();
                        }
                        Some(MediaBackendKind::GstPop) => {
                            let session = CAM_RTMP_GSTPOP_SESSION.lock().take();
                            if let Some(session) = session {
                                tokio::spawn(async move {
                                    teardown_gstpop_session(session).await;
                                });
                            }
                        }
                        None => {}
                    }
                    
                    fail(&cam_event_ui_weak, reason);
                }
            },
            Err(_) => break,
        }
    });

    let ui_weak = ui.as_weak();

    let event_tx_clone = event_tx.clone();
    let app_jh = runtime.spawn(async move {
        Application::new(ui_weak, event_tx_clone, app_clone)
            .await
            .unwrap()
            .run_event_loop(event_rx)
            .await
            .unwrap();
    });

    ui.run().unwrap();

    // Drop the runtime context guard before `block_on`, which panics if
    // called from within a tokio runtime context.
    drop(_runtime_guard);

    runtime.block_on(async move {
        if let Err(err) = event_tx.send(Event::Quit) {
            error!(?err, "Failed to send quit event");
        }
        if let Err(err) = app_jh.await {
            error!(?err, "Android application task join failed");
        }
    });

    debug!("Finished");
}
