//! Android entry point. Called by slint_android via JNI bootstrap.
//! Extracted from src/lib.rs as part of refactor step 07.7.

use migration_runtime::protocol::{Scene, SceneWidgetPlacement, Widget, WidgetLayout, WidgetType};
use parking_lot::Mutex;
use slint::ComponentHandle;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

use crate::application::defaults::{default_presets, default_quick_actions};
use crate::application::Application;
use crate::jni_bridge::helpers::{
    cache_app_class_loader, call_java_method_no_args, handle_back_request, init_vm,
    resolve_android_files_dir, JavaMethod,
};
use crate::platform::panel_stack::PanelStack;
use crate::platform::platform_app::{spawn_recording_ticker, PlatformApp, RecordingTickerState};
use crate::*;

use gstpop_runtime::GstPopClient;
use once_cell::sync::Lazy;
use std::sync::atomic::AtomicBool;

/// Set to true when `scan-rtmp-qr` is called so the next QrScanResult
/// is routed to RTMP auto-configuration rather than FCast device pairing.
pub(crate) static RTMP_QR_PENDING: AtomicBool = AtomicBool::new(false);

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
    let vm = unsafe { jni::JavaVM::from_raw(ptr).unwrap() };
    let vm = init_vm(vm);
    // Cache the app ClassLoader via the Activity instance (ANativeActivity::clazz).
    // NativeActivity native threads use the bootstrap classloader, so find_class
    // can't see app dex classes. Calling getClassLoader() on the live Activity
    // object gives the full app classloader and works from any thread thereafter.
    {
        let act_ptr = app.activity_as_ptr() as *mut jni::sys::_jobject;
        assert!(!act_ptr.is_null(), "Activity ptr is null");
        // SAFETY: PlatformApp holds the ANativeActivity, and activity_as_ptr()
        // returns ANativeActivity::clazz which is the live Activity instance for
        // the lifetime of the process.
        let activity = unsafe { jni::objects::JObject::from_raw(act_ptr) };
        let mut env = vm.get_env().expect("get_env for class loader cache");
        cache_app_class_loader(&mut env, &activity);
    }
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

    #[derive(Clone, Default)]
    struct SceneRegistryState {
        scenes: BTreeMap<String, Scene>,
        // User-facing display order (the BTreeMap above is keyed by id, so it
        // can't preserve order on its own). Tolerant of drift — see
        // `ordered_scene_ids`.
        scene_order: Vec<String>,
        widgets: BTreeMap<String, Widget>,
        current_scene_id: Option<String>,
    }

    /// Scene ids in user display order. Drops stale ids and appends any scene
    /// missing from `scene_order` (in stable BTreeMap order), so the order list
    /// never needs to be perfectly in sync with the map.
    fn ordered_scene_ids(reg: &SceneRegistryState) -> Vec<String> {
        let mut ids: Vec<String> = reg
            .scene_order
            .iter()
            .filter(|id| reg.scenes.contains_key(*id))
            .cloned()
            .collect();
        for id in reg.scenes.keys() {
            if !ids.contains(id) {
                ids.push(id.clone());
            }
        }
        ids
    }

    fn default_main_scene() -> Scene {
        Scene {
            id: "scene-main".to_owned(),
            name: "Main".to_owned(),
            enabled: true,
            widgets: vec![],
            quick_switch_group: None,
        }
    }

    fn widget_type_str(widget_type: &WidgetType) -> &'static str {
        match widget_type {
            WidgetType::Text { .. } => "text",
            WidgetType::Image { .. } => "image",
            WidgetType::Crop { .. } => "crop",
            WidgetType::Clock { .. } => "clock",
        }
    }

    fn placement_items(reg: &SceneRegistryState, scene_id: &str) -> Vec<ScenePlacementItem> {
        reg.scenes
            .get(scene_id)
            .map(|scene| {
                scene
                    .widgets
                    .iter()
                    .filter_map(|placement| {
                        reg.widgets
                            .get(&placement.widget_id)
                            .map(|widget| ScenePlacementItem {
                                widget_id: placement.widget_id.clone().into(),
                                name: widget.name.clone().into(),
                                widget_type: widget_type_str(&widget.widget_type).into(),
                                enabled: placement.enabled,
                                x: placement.layout.x as f32,
                                y: placement.layout.y as f32,
                                width: placement.layout.width as f32,
                                height: placement.layout.height as f32,
                                opacity: placement.layout.opacity as f32,
                                zorder: placement.zorder,
                            })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn update_editing_scene_bridge(ui: &MainWindow, reg: &SceneRegistryState, scene_id: &str) {
        let bridge = ui.global::<Bridge>();
        bridge.set_editing_scene_id(scene_id.into());

        if let Some(scene) = reg.scenes.get(scene_id) {
            bridge.set_editing_scene_name(scene.name.clone().into());
            bridge.set_editing_scene_quick_group(scene.quick_switch_group.unwrap_or(0) as i32);
            bridge.set_editing_scene_widgets(
                std::rc::Rc::new(slint::VecModel::from(placement_items(reg, scene_id))).into(),
            );
        } else {
            bridge.set_editing_scene_name("".into());
            bridge.set_editing_scene_quick_group(0);
            bridge
                .set_editing_scene_widgets(std::rc::Rc::new(slint::VecModel::from(vec![])).into());
        }
    }

    fn push_scene_models(ui_handle: slint::Weak<MainWindow>, reg: &SceneRegistryState) {
        let reg = reg.clone();
        let scenes = ordered_scene_ids(&reg)
            .iter()
            .filter_map(|id| reg.scenes.get(id))
            .map(|scene| SceneItem {
                id: scene.id.clone().into(),
                name: scene.name.clone().into(),
                enabled: scene.enabled,
                active: reg.current_scene_id.as_deref() == Some(scene.id.as_str()),
                widget_count: scene.widgets.len() as i32,
                quick_switch_group: scene.quick_switch_group.unwrap_or(0) as i32,
            })
            .collect::<Vec<_>>();
        let widgets = reg
            .widgets
            .values()
            .map(|widget| WidgetItem {
                id: widget.id.clone().into(),
                name: widget.name.clone().into(),
                widget_type: widget_type_str(&widget.widget_type).into(),
                enabled: widget.enabled,
            })
            .collect::<Vec<_>>();
        let current_scene_id = reg.current_scene_id.clone().unwrap_or_default();

        let _ = ui_handle.upgrade_in_event_loop(move |ui| {
            let bridge = ui.global::<Bridge>();
            bridge.set_scenes(std::rc::Rc::new(slint::VecModel::from(scenes)).into());
            bridge.set_widgets(std::rc::Rc::new(slint::VecModel::from(widgets)).into());
            bridge.set_current_scene_id(current_scene_id.into());

            let current_editing = bridge.get_editing_scene_id().to_string();
            let next_editing = if reg.scenes.contains_key(&current_editing) {
                current_editing
            } else if let Some(current_id) = reg.current_scene_id.clone() {
                current_id
            } else {
                reg.scenes.keys().next().cloned().unwrap_or_default()
            };
            update_editing_scene_bridge(&ui, &reg, &next_editing);
        });
    }

    fn persist_scene_registry(reg: &SceneRegistryState) {
        let scenes = ordered_scene_ids(reg)
            .iter()
            .filter_map(|id| reg.scenes.get(id).cloned())
            .collect::<Vec<_>>();
        let widgets = reg.widgets.values().cloned().collect::<Vec<_>>();
        let current_scene_id = reg.current_scene_id.clone();
        if let Err(err) = crate::config::update(move |cfg| {
            cfg.scenes = scenes;
            cfg.widgets = widgets;
            cfg.current_scene_id = current_scene_id;
        }) {
            tracing::error!(%err, "Failed to persist scene registry");
        }
    }

    fn dispatch_scene_command(command: migration_runtime::protocol::Command) {
        match migration_runtime::runtime::handle_command(command) {
            migration_runtime::protocol::CommandResult::Success
            | migration_runtime::protocol::CommandResult::Info(_) => {}
            migration_runtime::protocol::CommandResult::Error(err) => {
                tracing::warn!(%err, "Scene/widget runtime command failed");
            }
        }
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
    // Debug frame dumps: expose the dump dir to the migration-runtime crate too.
    // Toggle at runtime by creating/removing the marker file <files_dir>/dump/on
    // (see scripts below). Frames land in <files_dir>/dump/.
    std::env::set_var("FCAST_DUMP_DIR", files_dir.join("dump"));
    let backend_lifecycle =
        std::sync::Arc::new(backend::lifecycle::BackendLifecycle::new(files_dir));
    backend_lifecycle.register(&ui);

    // Hydrate the Bridge from saved configuration on startup
    let backend_cfg = crate::config::load();

    let mut initial_scene_registry = SceneRegistryState::default();
    for scene in backend_cfg.scenes.clone() {
        initial_scene_registry
            .scenes
            .insert(scene.id.clone(), scene);
    }
    // Preserve the saved display order (config `scenes` is an ordered Vec).
    initial_scene_registry.scene_order = backend_cfg.scenes.iter().map(|s| s.id.clone()).collect();
    for widget in backend_cfg.widgets.clone() {
        initial_scene_registry
            .widgets
            .insert(widget.id.clone(), widget);
    }
    let mut repaired_scene_registry = false;
    if initial_scene_registry.scenes.is_empty() {
        let scene = default_main_scene();
        initial_scene_registry
            .scenes
            .insert(scene.id.clone(), scene.clone());
        initial_scene_registry.scene_order = vec![scene.id.clone()];
        initial_scene_registry.current_scene_id = Some(scene.id);
        repaired_scene_registry = true;
    } else {
        initial_scene_registry.current_scene_id = backend_cfg
            .current_scene_id
            .clone()
            .filter(|scene_id| initial_scene_registry.scenes.contains_key(scene_id))
            .or_else(|| initial_scene_registry.scenes.keys().next().cloned());
        repaired_scene_registry =
            initial_scene_registry.current_scene_id != backend_cfg.current_scene_id;
    }
    let scene_registry = Arc::new(Mutex::new(initial_scene_registry));
    {
        let scene_snapshot = scene_registry.lock().clone();
        if repaired_scene_registry {
            persist_scene_registry(&scene_snapshot);
        }
        for widget in scene_snapshot.widgets.values().cloned() {
            dispatch_scene_command(migration_runtime::protocol::Command::CreateWidget { widget });
        }
        for scene in scene_snapshot.scenes.values().cloned() {
            dispatch_scene_command(migration_runtime::protocol::Command::CreateScene { scene });
        }
        if let Some(current_scene_id) = scene_snapshot.current_scene_id.clone() {
            dispatch_scene_command(migration_runtime::protocol::Command::SetScene {
                scene_id: current_scene_id,
            });
        }
        push_scene_models(ui.as_weak(), &scene_snapshot);
    }

    let rtmp_cfg = backend_cfg.camera_rtmp.unwrap_or_default();
    let global_cam_cfg = backend_cfg.global_camera.unwrap_or_default();
    let b = ui.global::<Bridge>();
    b.set_cam_rtmp_url(rtmp_cfg.url.into());
    // Lock the session pipeline mode to the launch value before the selector can
    // change it; start/stop read this, never the live selector idx (see config::*).
    crate::config::prime_session_android_camera_pipeline(backend_cfg.android_camera_pipeline);
    b.set_camera_pipeline_mode_idx(match backend_cfg.android_camera_pipeline {
        crate::config::AndroidCameraPipeline::LegacyRawI420Gstreamer => 0,
        crate::config::AndroidCameraPipeline::StreamPackDirectSrt => 1,
        crate::config::AndroidCameraPipeline::StreamPackEncodedToGstreamer => 2,
    });
    b.set_camera_idx(global_cam_cfg.camera_idx);
    b.set_resolution_idx(global_cam_cfg.resolution_idx);
    b.set_framerate_idx(global_cam_cfg.framerate_idx);
    b.set_camera_orientation_mode_idx(global_cam_cfg.orientation_mode_idx.clamp(0, 2));
    b.set_camera_mirror_front(global_cam_cfg.mirror_front);
    b.set_camera_stabilization(global_cam_cfg.stabilization);
    b.set_camera_zoom_level(global_cam_cfg.zoom_level.max(0.5));
    b.set_cam_rtmp_camera_permission(
        crate::jni_bridge::camera::upcall_probe_camera_permission().unwrap_or(false),
    );

    let saved_key = crate::secret::load("cam_rtmp_stream_key").ok().flatten();
    b.set_cam_rtmp_stream_key(saved_key.as_deref().unwrap_or("Byhag83gMx").into());

    // Hydrate the SRT destination from saved config (passphrase comes from the
    // secret store, never from backend.json).
    let srt_cfg = backend_cfg.srt_destination.clone().unwrap_or_default();
    {
        let mut srt = b.get_srt_destination();
        srt.uri = srt_cfg.url.clone().into();
        srt.latency_ms = srt_cfg.latency_ms;
        b.set_srt_destination(srt);
    }
    b.set_srt_destination_pbkeylen_idx(srt_cfg.pbkeylen_idx);
    let srt_pass = crate::secret::load("srt_destination_passphrase")
        .ok()
        .flatten();
    b.set_srt_destination_passphrase(srt_pass.as_deref().unwrap_or("").into());

    ui.global::<Bridge>().on_create_scene({
        let scene_registry = scene_registry.clone();
        let ui_weak = ui.as_weak();
        move |name| {
            let name = name.trim().to_string();
            if name.is_empty() {
                return;
            }

            let scene = Scene {
                id: uuid::Uuid::new_v4().to_string(),
                name,
                enabled: true,
                widgets: vec![],
                quick_switch_group: None,
            };

            let snapshot = {
                let mut reg = scene_registry.lock();
                if reg.current_scene_id.is_none() {
                    reg.current_scene_id = Some(scene.id.clone());
                }
                reg.scenes.insert(scene.id.clone(), scene.clone());
                reg.scene_order.push(scene.id.clone());
                let snapshot = reg.clone();
                persist_scene_registry(&snapshot);
                snapshot
            };

            dispatch_scene_command(migration_runtime::protocol::Command::CreateScene { scene });
            push_scene_models(ui_weak.clone(), &snapshot);
        }
    });

    ui.global::<Bridge>().on_set_scene({
        let scene_registry = scene_registry.clone();
        let ui_weak = ui.as_weak();
        move |scene_id| {
            let scene_id = scene_id.to_string();
            let snapshot = {
                let mut reg = scene_registry.lock();
                if !reg.scenes.contains_key(&scene_id) {
                    return;
                }
                reg.current_scene_id = Some(scene_id.clone());
                let snapshot = reg.clone();
                persist_scene_registry(&snapshot);
                snapshot
            };

            dispatch_scene_command(migration_runtime::protocol::Command::SetScene {
                scene_id: scene_id.clone(),
            });
            push_scene_models(ui_weak.clone(), &snapshot);
        }
    });

    ui.global::<Bridge>().on_rename_scene({
        let scene_registry = scene_registry.clone();
        let ui_weak = ui.as_weak();
        move |scene_id, name| {
            let scene_id = scene_id.to_string();
            let updated_scene = {
                let mut reg = scene_registry.lock();
                let Some(scene) = reg.scenes.get_mut(&scene_id) else {
                    return;
                };
                scene.name = name.to_string();
                let updated_scene = scene.clone();
                let snapshot = reg.clone();
                persist_scene_registry(&snapshot);
                push_scene_models(ui_weak.clone(), &snapshot);
                updated_scene
            };

            dispatch_scene_command(migration_runtime::protocol::Command::UpdateScene {
                scene: updated_scene,
            });
        }
    });

    ui.global::<Bridge>().on_remove_scene({
        let scene_registry = scene_registry.clone();
        let ui_weak = ui.as_weak();
        move |scene_id| {
            let scene_id = scene_id.to_string();
            let (snapshot, next_scene_id) = {
                let mut reg = scene_registry.lock();
                if reg.scenes.remove(&scene_id).is_none() {
                    return;
                }
                reg.scene_order.retain(|id| id != &scene_id);
                if reg.current_scene_id.as_deref() == Some(scene_id.as_str()) {
                    reg.current_scene_id = reg.scenes.keys().next().cloned();
                } else if reg
                    .current_scene_id
                    .as_ref()
                    .is_some_and(|current| !reg.scenes.contains_key(current))
                {
                    reg.current_scene_id = reg.scenes.keys().next().cloned();
                }
                let next_scene_id = reg.current_scene_id.clone();
                let snapshot = reg.clone();
                persist_scene_registry(&snapshot);
                (snapshot, next_scene_id)
            };

            dispatch_scene_command(migration_runtime::protocol::Command::RemoveScene {
                scene_id: scene_id.clone(),
            });
            if let Some(next_scene_id) = next_scene_id.clone() {
                dispatch_scene_command(migration_runtime::protocol::Command::SetScene {
                    scene_id: next_scene_id,
                });
            }
            push_scene_models(ui_weak.clone(), &snapshot);
        }
    });

    // Reordering is display/persistence-only — scenes are switched by id, not
    // position, so no runtime command is needed.
    ui.global::<Bridge>().on_reorder_scenes({
        let scene_registry = scene_registry.clone();
        let ui_weak = ui.as_weak();
        move |from, to| {
            let snapshot = {
                let mut reg = scene_registry.lock();
                // Canonicalize first so indices line up with what the UI rendered.
                reg.scene_order = ordered_scene_ids(&reg);
                let len = reg.scene_order.len() as i32;
                if from < 0 || to < 0 || from >= len || to >= len || from == to {
                    return;
                }
                let id = reg.scene_order.remove(from as usize);
                reg.scene_order.insert(to as usize, id);
                let snapshot = reg.clone();
                persist_scene_registry(&snapshot);
                snapshot
            };
            push_scene_models(ui_weak.clone(), &snapshot);
        }
    });

    ui.global::<Bridge>().on_set_scene_quick_group({
        let scene_registry = scene_registry.clone();
        let ui_weak = ui.as_weak();
        move |scene_id, group| {
            let scene_id = scene_id.to_string();
            let updated_scene = {
                let mut reg = scene_registry.lock();
                let Some(scene) = reg.scenes.get_mut(&scene_id) else {
                    return;
                };
                scene.quick_switch_group = if group <= 0 {
                    None
                } else {
                    Some(group.clamp(1, i32::from(u8::MAX)) as u8)
                };
                let updated_scene = scene.clone();
                let snapshot = reg.clone();
                persist_scene_registry(&snapshot);
                push_scene_models(ui_weak.clone(), &snapshot);
                updated_scene
            };

            dispatch_scene_command(migration_runtime::protocol::Command::UpdateScene {
                scene: updated_scene,
            });
        }
    });

    ui.global::<Bridge>().on_open_scene_edit({
        let scene_registry = scene_registry.clone();
        let ui_weak = ui.as_weak();
        move |scene_id| {
            let scene_id = scene_id.to_string();
            let snapshot = scene_registry.lock().clone();
            let Some(ui) = ui_weak.upgrade() else {
                return;
            };
            update_editing_scene_bridge(&ui, &snapshot, &scene_id);
            ui.global::<crate::PanelBridge>()
                .invoke_push(crate::Panel::SceneEdit);
        }
    });

    ui.global::<Bridge>().on_create_widget({
        let scene_registry = scene_registry.clone();
        let ui_weak = ui.as_weak();
        move || {
            let Some(ui) = ui_weak.upgrade() else {
                return;
            };
            let bridge = ui.global::<Bridge>();
            let scene_id = bridge.get_editing_scene_id().to_string();
            if scene_id.is_empty() {
                return;
            }

            let widget_type = match bridge.get_draft_widget_type() {
                WidgetTypeChoice::Text => WidgetType::Text {
                    format: bridge.get_draft_widget_text_format().to_string(),
                    font_size: Some(bridge.get_draft_widget_font_size().max(1) as u32),
                    color: None,
                },
                WidgetTypeChoice::Image => {
                    let asset_id = bridge.get_draft_widget_image_path().to_string();
                    if asset_id.is_empty() {
                        Application::flash_banner(
                            ui_weak.clone(),
                            "Pick an image first".into(),
                            BannerSeverity::Warning,
                            std::time::Duration::from_secs(2),
                        );
                        return;
                    }
                    WidgetType::Image {
                        asset_id,
                        scale_mode: Some(match bridge.get_draft_widget_scale_idx() {
                            1 => "fill".to_string(),
                            2 => "stretch".to_string(),
                            _ => "fit".to_string(),
                        }),
                    }
                }
                WidgetTypeChoice::Crop => WidgetType::Crop {
                    top: bridge.get_draft_crop_top() as f64,
                    bottom: bridge.get_draft_crop_bottom() as f64,
                    left: bridge.get_draft_crop_left() as f64,
                    right: bridge.get_draft_crop_right() as f64,
                },
                WidgetTypeChoice::Clock => WidgetType::Clock {
                    format: bridge.get_draft_widget_clock_format().to_string(),
                    font_size: Some(bridge.get_draft_widget_font_size().max(1) as u32),
                    color: None,
                },
            };

            let widget = Widget {
                id: uuid::Uuid::new_v4().to_string(),
                name: bridge.get_draft_widget_name().to_string(),
                widget_type,
                enabled: true,
            };

            let updated_scene = {
                let mut reg = scene_registry.lock();
                reg.widgets.insert(widget.id.clone(), widget.clone());
                let Some(scene) = reg.scenes.get_mut(&scene_id) else {
                    return;
                };
                let zorder = scene.widgets.iter().map(|p| p.zorder).max().unwrap_or(0) + 1;
                scene.widgets.push(SceneWidgetPlacement {
                    widget_id: widget.id.clone(),
                    layout: WidgetLayout::default(),
                    enabled: true,
                    zorder,
                });
                let updated_scene = scene.clone();
                let snapshot = reg.clone();
                persist_scene_registry(&snapshot);
                push_scene_models(ui_weak.clone(), &snapshot);
                updated_scene
            };

            dispatch_scene_command(migration_runtime::protocol::Command::CreateWidget { widget });
            dispatch_scene_command(migration_runtime::protocol::Command::UpdateScene {
                scene: updated_scene,
            });
        }
    });

    ui.global::<Bridge>().on_remove_widget({
        let scene_registry = scene_registry.clone();
        let ui_weak = ui.as_weak();
        move |widget_id| {
            let widget_id = widget_id.to_string();
            let changed_scenes = {
                let mut reg = scene_registry.lock();
                reg.widgets.remove(&widget_id);
                let mut changed = Vec::new();
                for scene in reg.scenes.values_mut() {
                    let before = scene.widgets.len();
                    scene
                        .widgets
                        .retain(|placement| placement.widget_id != widget_id);
                    if scene.widgets.len() != before {
                        changed.push(scene.clone());
                    }
                }
                let snapshot = reg.clone();
                persist_scene_registry(&snapshot);
                push_scene_models(ui_weak.clone(), &snapshot);
                changed
            };

            dispatch_scene_command(migration_runtime::protocol::Command::RemoveWidget {
                widget_id,
            });
            for scene in changed_scenes {
                dispatch_scene_command(migration_runtime::protocol::Command::UpdateScene { scene });
            }
        }
    });

    ui.global::<Bridge>().on_add_widget_to_scene({
        let scene_registry = scene_registry.clone();
        let ui_weak = ui.as_weak();
        move |scene_id, widget_id| {
            let scene_id = scene_id.to_string();
            let widget_id = widget_id.to_string();
            let updated_scene = {
                let mut reg = scene_registry.lock();
                if !reg.widgets.contains_key(&widget_id) {
                    return;
                }
                let Some(scene) = reg.scenes.get_mut(&scene_id) else {
                    return;
                };
                if scene
                    .widgets
                    .iter()
                    .any(|placement| placement.widget_id == widget_id)
                {
                    return;
                }
                let zorder = scene.widgets.iter().map(|p| p.zorder).max().unwrap_or(0) + 1;
                scene.widgets.push(SceneWidgetPlacement {
                    widget_id,
                    layout: WidgetLayout::default(),
                    enabled: true,
                    zorder,
                });
                let updated_scene = scene.clone();
                let snapshot = reg.clone();
                persist_scene_registry(&snapshot);
                push_scene_models(ui_weak.clone(), &snapshot);
                updated_scene
            };

            dispatch_scene_command(migration_runtime::protocol::Command::UpdateScene {
                scene: updated_scene,
            });
        }
    });

    ui.global::<Bridge>().on_remove_widget_from_scene({
        let scene_registry = scene_registry.clone();
        let ui_weak = ui.as_weak();
        move |scene_id, widget_id| {
            let scene_id = scene_id.to_string();
            let widget_id = widget_id.to_string();
            let updated_scene = {
                let mut reg = scene_registry.lock();
                let Some(scene) = reg.scenes.get_mut(&scene_id) else {
                    return;
                };
                let before = scene.widgets.len();
                scene
                    .widgets
                    .retain(|placement| placement.widget_id != widget_id);
                if scene.widgets.len() == before {
                    return;
                }
                let updated_scene = scene.clone();
                let snapshot = reg.clone();
                persist_scene_registry(&snapshot);
                push_scene_models(ui_weak.clone(), &snapshot);
                updated_scene
            };

            dispatch_scene_command(migration_runtime::protocol::Command::UpdateScene {
                scene: updated_scene,
            });
        }
    });

    ui.global::<Bridge>().on_set_placement_enabled({
        let scene_registry = scene_registry.clone();
        let ui_weak = ui.as_weak();
        move |scene_id, widget_id, enabled| {
            let scene_id = scene_id.to_string();
            let widget_id = widget_id.to_string();
            let updated_scene = {
                let mut reg = scene_registry.lock();
                let Some(scene) = reg.scenes.get_mut(&scene_id) else {
                    return;
                };
                let Some(placement) = scene
                    .widgets
                    .iter_mut()
                    .find(|placement| placement.widget_id == widget_id)
                else {
                    return;
                };
                placement.enabled = enabled;
                let updated_scene = scene.clone();
                let snapshot = reg.clone();
                persist_scene_registry(&snapshot);
                push_scene_models(ui_weak.clone(), &snapshot);
                updated_scene
            };

            dispatch_scene_command(migration_runtime::protocol::Command::UpdateScene {
                scene: updated_scene,
            });
        }
    });

    ui.global::<Bridge>().on_pick_widget_image({
        let ui_weak = ui.as_weak();
        move || {
            Application::flash_banner(
                ui_weak.clone(),
                "Image picker not wired yet".into(),
                BannerSeverity::Info,
                std::time::Duration::from_secs(2),
            );
        }
    });

    ui.global::<Bridge>().on_apply_widget_layout({
        let scene_registry = scene_registry.clone();
        let ui_weak = ui.as_weak();
        move |scene_id, widget_id, x, y, width, height, opacity| {
            let scene_id = scene_id.to_string();
            let widget_id = widget_id.to_string();
            let layout = WidgetLayout {
                x: x as f64,
                y: y as f64,
                width: width as f64,
                height: height as f64,
                rotation: 0.0,
                opacity: opacity as f64,
            };
            let snapshot = {
                let mut reg = scene_registry.lock();
                let Some(scene) = reg.scenes.get_mut(&scene_id) else {
                    return;
                };
                let Some(placement) = scene
                    .widgets
                    .iter_mut()
                    .find(|placement| placement.widget_id == widget_id)
                else {
                    return;
                };
                placement.layout = layout.clone();
                let snapshot = reg.clone();
                persist_scene_registry(&snapshot);
                snapshot
            };

            dispatch_scene_command(migration_runtime::protocol::Command::UpdateWidgetLayout {
                scene_id,
                widget_id,
                layout,
            });
            push_scene_models(ui_weak.clone(), &snapshot);
        }
    });

    ui.global::<Bridge>().on_set_camera_pipeline_mode({
        move |idx| {
            let mode = match idx {
                1 => crate::config::AndroidCameraPipeline::StreamPackDirectSrt,
                2 => crate::config::AndroidCameraPipeline::StreamPackEncodedToGstreamer,
                _ => crate::config::AndroidCameraPipeline::LegacyRawI420Gstreamer,
            };
            if let Err(e) = crate::config::update(|cfg| {
                cfg.android_camera_pipeline = mode;
            }) {
                tracing::error!("Failed to save camera pipeline mode: {e}");
            }
        }
    });

    ui.global::<Bridge>().on_save_cam_rtmp_config({
        let ui_weak = ui.as_weak();
        move || {
            let Some(ui) = ui_weak.upgrade() else { return };
            let b = ui.global::<Bridge>();

            // Save Stream Key securely
            let key = b.get_cam_rtmp_stream_key().to_string();
            if let Err(e) = crate::secret::store("cam_rtmp_stream_key", &key) {
                tracing::error!("Failed to store cam_rtmp_stream_key: {}", e);
            }

            // Save standard config (URL + Global Camera Settings)
            let url = b.get_cam_rtmp_url().to_string();
            let camera_idx = b.get_camera_idx();
            let resolution_idx = b.get_resolution_idx();
            let framerate_idx = b.get_framerate_idx();
            let orientation_mode_idx = b.get_camera_orientation_mode_idx();
            let mirror_front = b.get_camera_mirror_front();
            let stabilization = b.get_camera_stabilization();
            let zoom_level = b.get_camera_zoom_level();

            if let Err(e) = crate::config::update(|cfg| {
                let mut rtmp = cfg.camera_rtmp.clone().unwrap_or_default();
                rtmp.url = url;
                cfg.camera_rtmp = Some(rtmp);

                let mut cam = cfg.global_camera.clone().unwrap_or_default();
                cam.camera_idx = camera_idx;
                cam.resolution_idx = resolution_idx;
                cam.framerate_idx = framerate_idx;
                cam.orientation_mode_idx = orientation_mode_idx;
                cam.mirror_front = mirror_front;
                cam.stabilization = stabilization;
                cam.zoom_level = zoom_level;
                cfg.global_camera = Some(cam);
            }) {
                tracing::error!("Failed to save global config: {}", e);
            }

            tracing::info!("Camera and RTMP settings saved successfully");
        }
    });

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
            if p == Panel::CameraRtmpStream {
                ui.global::<Bridge>().invoke_start_camera_rtmp_preview();
            }
        }
    });

    ui.global::<PanelBridge>().on_pop({
        let stack = panel_stack.clone();
        let ui_weak = ui.as_weak();
        move || {
            let Some(ui) = ui_weak.upgrade() else { return };
            let pb = ui.global::<PanelBridge>();
            let next = stack.pop_panel();
            pb.set_active(next);
            pb.set_stack(stack.as_model());
            if next == Panel::CameraRtmpStream {
                ui.global::<Bridge>().invoke_start_camera_rtmp_preview();
            }
        }
    });

    ui.global::<PanelBridge>().on_replace({
        let ui_weak = ui.as_weak();
        move |p: Panel| {
            if let Some(ui) = ui_weak.upgrade() {
                ui.global::<PanelBridge>().set_active(p);
                if p == Panel::CameraRtmpStream {
                    ui.global::<Bridge>().invoke_start_camera_rtmp_preview();
                }
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
                _ => {}
            }
        }
    });

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

    // ── Codec test callbacks ─────────────────────────────────────────────
    // Publish a report both as the raw string and as a per-line model, so the
    // page can render it in a virtualised ListView (a single huge Text janks).
    fn set_codec_log(ui: &MainWindow, text: &str) {
        let lines: Vec<slint::SharedString> = text.lines().map(|l| l.into()).collect();
        ui.global::<Bridge>()
            .set_codec_test_log_lines(std::rc::Rc::new(slint::VecModel::from(lines)).into());
        ui.global::<Bridge>().set_codec_test_log(text.into());
    }

    // Write the current report to <files_dir>/codec-logs/codec-dump-<unixts>.log
    // (app-internal storage — no permission needed). Returns the absolute path.
    fn save_codec_log_to_file(text: &str) -> Result<String, String> {
        let dir = crate::config::get_files_dir()
            .ok_or_else(|| "files dir not initialised".to_string())?
            .join("codec-logs");
        std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir: {e}"))?;
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let path = dir.join(format!("codec-dump-{ts}.log"));
        std::fs::write(&path, text).map_err(|e| format!("write: {e}"))?;
        Ok(path.display().to_string())
    }

    ui.global::<Bridge>().on_run_codec_test({
        let ui_weak = ui.as_weak();
        move || {
            let _ = ui_weak.upgrade_in_event_loop(|ui| {
                ui.global::<Bridge>().set_codec_test_running(true);
                set_codec_log(&ui, "Running full codec test…");
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
                    set_codec_log(&ui, &report);
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
                set_codec_log(&ui, "Dumping codecs…");
            });
            let ui_inner = ui_weak.clone();
            std::thread::spawn(move || {
                let report = match crate::jni_bridge::codec_test::run_codec_dump_all() {
                    Ok(r) => r,
                    Err(e) => format!("FAIL: {e}\n"),
                };
                let _ = ui_inner.upgrade_in_event_loop(move |ui| {
                    set_codec_log(&ui, &report);
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
                set_codec_log(&ui, "Running encoder smoke test…");
            });
            let ui_inner = ui_weak.clone();
            std::thread::spawn(move || {
                let report = match crate::jni_bridge::codec_test::run_codec_smoke_test() {
                    Ok(r) => r,
                    Err(e) => format!("FAIL: {e}\n"),
                };
                let _ = ui_inner.upgrade_in_event_loop(move |ui| {
                    set_codec_log(&ui, &report);
                    ui.global::<Bridge>().set_codec_test_running(false);
                });
            });
        }
    });

    ui.global::<Bridge>().on_save_codec_log({
        let ui_weak = ui.as_weak();
        move || {
            let _ = ui_weak.upgrade_in_event_loop(|ui| {
                let bridge = ui.global::<Bridge>();
                let text = bridge.get_codec_test_log().to_string();
                let status = match save_codec_log_to_file(&text) {
                    Ok(path) => {
                        info!("codec log saved to {path}");
                        format!("Saved to {path}")
                    }
                    Err(e) => {
                        error!("codec log save failed: {e}");
                        format!("Save failed: {e}")
                    }
                };
                bridge.set_codec_test_save_status(status.into());
            });
        }
    });

    // ── Codec performance benchmark callbacks ────────────────────────────
    // Publish a report as both the raw string and a per-line model so the page
    // renders it in a virtualised ListView (mirrors set_codec_log).
    fn set_perf_log(ui: &MainWindow, text: &str) {
        let lines: Vec<slint::SharedString> = text.lines().map(|l| l.into()).collect();
        ui.global::<Bridge>()
            .set_perf_test_log_lines(std::rc::Rc::new(slint::VecModel::from(lines)).into());
        ui.global::<Bridge>().set_perf_test_log(text.into());
    }

    let _ = crate::PERF_UI_WEAK.set(ui.as_weak());

    ui.global::<Bridge>().on_run_perf_test({
        let ui_weak = ui.as_weak();
        move || {
            let _ = ui_weak.upgrade_in_event_loop(|ui| {
                ui.global::<Bridge>().set_perf_test_running(true);
                set_perf_log(
                    &ui,
                    "Running full benchmark in :codec_bench process…\nThis may take 1–2 minutes.",
                );
            });
            let req = crate::codec_bench_plan::CodecBenchRequest {
                include_factory_list: true,
                include_encode_perf: true,
                include_decode_perf: true,
                kill_process_after_decode: true,
                use_foreign_egl: true,
            };
            if let Err(e) = crate::codec_bench_jni::request_codec_benchmark(&req) {
                let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                    set_perf_log(&ui, &format!("Failed to start benchmark service: {e}"));
                    ui.global::<Bridge>().set_perf_test_running(false);
                });
            }
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
                set_perf_log(&ui, "Running decode benchmark in :codec_bench process…");
            });
            let req = crate::codec_bench_plan::CodecBenchRequest {
                include_factory_list: false,
                include_encode_perf: false,
                include_decode_perf: true,
                kill_process_after_decode: true,
                use_foreign_egl: true,
            };
            if let Err(e) = crate::codec_bench_jni::request_codec_benchmark(&req) {
                let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                    set_perf_log(&ui, &format!("Failed to start benchmark service: {e}"));
                    ui.global::<Bridge>().set_perf_test_running(false);
                });
            }
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

    use crate::jni_bridge::camera::{
        upcall_start_camera_capture, upcall_start_camera_preview, upcall_stop_camera_capture,
        upcall_stop_camera_preview,
    };
    use migration_runtime::protocol::{Command, CommandResult, DestinationFamily};

    fn tear_down_migration() {
        for id in ["cam-rtmp-src", "cam-rtmp-dest"] {
            let _ = migration_runtime::runtime::handle_command(Command::Remove { id: id.into() });
        }
    }

    fn tear_down_srt() {
        for id in ["srt-cam-src", "srt-dest"] {
            let _ = migration_runtime::runtime::handle_command(Command::Remove { id: id.into() });
        }
    }

    // SRT destination state lives in the `srt-destination` struct property, so
    // updates are get-modify-set (mirrors the cam-rtmp flat-property helpers).
    fn srt_set_state(ui: &slint::Weak<MainWindow>, state: MixerState) {
        let _ = ui.upgrade_in_event_loop(move |u| {
            let b = u.global::<Bridge>();
            let mut d = b.get_srt_destination();
            d.state = state;
            b.set_srt_destination(d);
        });
    }

    fn srt_fail(ui: &slint::Weak<MainWindow>, msg: impl Into<String>) {
        let msg: slint::SharedString = msg.into().into();
        let _ = ui.upgrade_in_event_loop(move |u| {
            let b = u.global::<Bridge>();
            let mut d = b.get_srt_destination();
            d.last_error = msg;
            d.state = MixerState::Error;
            b.set_srt_destination(d);
        });
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

    #[allow(clippy::too_many_arguments)] // start params form a single conceptual record
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

        // Ensure any stale Migration RTMP pipeline is torn down before starting
        // GstPop — otherwise the RTMP server rejects with "Already publishing".
        tear_down_migration();

        info!("gstpop rtmp: starting embedded server on port {CAM_RTMP_GSTPOP_PORT}");
        let status = gstpop_runtime::start_embedded(CAM_RTMP_GSTPOP_PORT).await;
        info!("gstpop rtmp: embedded server status = {:?}", status.state);
        if !matches!(status.state, EmbeddedState::Running) {
            let msg = format!("gst-pop daemon not running: {:?}", status.last_error);
            return fail(&ui, msg);
        }

        let ws_url = format!("ws://127.0.0.1:{}/", status.port);
        info!("gstpop rtmp: connecting client to {ws_url}");
        let client = match GstPopClient::connect(&ws_url, None).await {
            Ok(c) => {
                info!("gstpop rtmp: ws client connected");
                Arc::new(c)
            }
            Err(e) => return fail(&ui, format!("client connect: {e:#}")),
        };

        let _flip = if mirror {
            "videoflip method=horizontal-flip ! "
        } else {
            ""
        };

        let audiosrc_desc = if cfg!(target_os = "android") {
            "openslessrc ! audioconvert ! audioresample"
        } else {
            "audiotestsrc wave=silence is-live=true do-timestamp=true"
        };

        let pipeline_desc = format!(
            "flvmux name=mux streamable=true latency=1000000000 ! rtmp2sink location={url} \
             appsrc name=camera-src is-live=true format=time block=false do-timestamp=true \
             ! video/x-raw,format=I420,width={w},height={h},framerate=0/1 \
             ! queue leaky=downstream max-size-buffers=2 max-size-bytes=0 max-size-time=0 \
             ! videoconvert \
             ! videorate drop-only=false \
             ! video/x-raw,framerate=30/1 \
             ! x264enc bitrate=2000 speed-preset=ultrafast tune=zerolatency key-int-max=60 bframes=0 \
             ! h264parse config-interval=-1 \
             ! video/x-h264,stream-format=avc,alignment=au \
             ! queue leaky=downstream max-size-buffers=2 max-size-bytes=0 max-size-time=0 ! mux.video \
             {audiosrc_desc} \
             ! audio/x-raw,rate=44100,channels=1 \
             ! voaacenc ! queue ! mux.audio",
            url = full_url,
            w = width,
            h = height,
            audiosrc_desc = audiosrc_desc,
        );
        info!("gstpop rtmp: using sink element = rtmp2sink");
        info!("gstpop rtmp: pipeline = {pipeline_desc}");

        let pid_val = match client
            .call(
                "create_pipeline",
                serde_json::json!({ "description": pipeline_desc }),
            )
            .await
        {
            Ok(val) => {
                info!("gstpop rtmp: create_pipeline ok: {val}");
                val
            }
            Err(e) => return fail(&ui, format!("create_pipeline: {e:#}")),
        };
        let pid = match pid_val.get("pipeline_id").and_then(|v| v.as_str()) {
            Some(p) => p.to_string(),
            None => {
                return fail(
                    &ui,
                    "create_pipeline response missing pipeline_id".to_string(),
                )
            }
        };
        info!("gstpop rtmp: pipeline_id = {pid}");

        // Subscribe to pipeline events before issuing play so we don't miss a
        // fast state transition or an immediate connection error.
        let mut event_rx = client.subscribe();
        let pid_for_event = pid.clone();

        // Start the camera BEFORE calling play. With appsrc is-live=true and a
        // network sink (rtmpsink), flvmux won't produce FLV output until it has
        // a video frame, and rtmpsink won't complete its PLAYING state change
        // until it can write the first packet. Feeding camera data first
        // unblocks the state change so play() returns in <1s rather than after
        // the 30s gstpop timeout.
        if let Err(e) =
            upcall_start_camera_capture(cam_idx, width, height, fps, mirror, stab, zoom, 1)
        {
            let _ = client
                .call(
                    "remove_pipeline",
                    serde_json::json!({ "pipeline_id": &pid }),
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

        // Start the frame push loop BEFORE calling play so that flvmux has
        // video data and can preroll, allowing rtmpsink to connect without
        // hitting the 30s gstpop timeout. PTS is set to u64::MAX (NONE) so
        // appsrc's do-timestamp=true assigns the pipeline clock time, keeping
        // video timestamps in sync with audiotestsrc (both use pipeline clock).
        {
            let frame_pair = crate::FRAME_PAIR.clone();
            let client_clone = client.clone();
            let pid_clone = pid.clone();
            let active_flag_push = active_flag.clone();

            tokio::spawn(async move {
                let mut frame_count: u64 = 0;

                loop {
                    if !active_flag_push.load(Ordering::SeqCst) {
                        break;
                    }
                    if frame_count < 10 || frame_count % 30 == 0 {
                        info!("gstpop push: waiting for frame={frame_count}");
                    }
                    let frame_bytes = {
                        let mut frame_guard = frame_pair.frame.lock();
                        let mut waits: u32 = 0;
                        while (*frame_guard).is_none() {
                            if !active_flag_push.load(Ordering::SeqCst) {
                                return;
                            }
                            frame_pair
                                .cond
                                .wait_for(&mut frame_guard, std::time::Duration::from_millis(50));
                            waits += 1;
                            if waits % 20 == 0 {
                                info!(
                                    "gstpop push: still waiting frame={frame_count} waits={waits}"
                                );
                            }
                        }
                        let vframe = (*frame_guard).take().unwrap();
                        let buffer = vframe.buffer().to_owned();
                        let mapped = buffer.map_readable().unwrap();
                        mapped.to_vec()
                    };

                    // u64::MAX = PTS_NONE: appsrc do-timestamp=true assigns pipeline clock.
                    let pts_ns = u64::MAX;

                    // Log every frame for the first 30, then every 30 frames. Include avg_Y always.
                    if frame_count < 30 || frame_count % 30 == 0 {
                        let y_len = (width * height) as usize;
                        let avg_y: u32 = frame_bytes[..y_len.min(frame_bytes.len())]
                            .iter()
                            .map(|&b| b as u32)
                            .sum::<u32>()
                            / y_len as u32;
                        info!(
                            "gstpop push: frame={frame_count} avg_Y={avg_y} size={}",
                            frame_bytes.len()
                        );
                    }

                    if let Err(e) = client_clone
                        .push_buffer(&pid_clone, "camera-src", &frame_bytes, pts_ns)
                        .await
                    {
                        error!("gst-pop push_buffer failed at frame={frame_count}: {e}");
                        break;
                    }

                    if frame_count < 30 || frame_count % 30 == 0 {
                        info!("gstpop push: frame={frame_count} sent ok");
                    }

                    frame_count += 1;
                }
                info!("gstpop push_buffer loop ended after {frame_count} frames");
            });
        }

        // Wait for the first frame to arrive before play() so flvmux has data.
        info!("gstpop rtmp: waiting for first camera frame before play");
        {
            let frame_pair_wait = crate::FRAME_PAIR.clone();
            tokio::task::spawn_blocking(move || {
                let mut guard = frame_pair_wait.frame.lock();
                frame_pair_wait
                    .cond
                    .wait_for(&mut guard, std::time::Duration::from_secs(5));
            })
            .await
            .ok();
        }
        info!("gstpop rtmp: first frame ready, calling play");

        info!("gstpop rtmp: calling play for pipeline {pid}");
        let play_client = client.clone();
        let play_pid = pid.clone();
        let mut play_rpc = tokio::spawn(async move {
            play_client
                .call("play", serde_json::json!({ "pipeline_id": &play_pid }))
                .await
                .map(|_| ())
        });
        let mut play_rpc_done = false;
        let playing_deadline = tokio::time::sleep(std::time::Duration::from_secs(45));
        tokio::pin!(playing_deadline);

        let play_result: Result<(), anyhow::Error> = loop {
            tokio::select! {
                _ = &mut playing_deadline => {
                    break Err(anyhow::anyhow!("timed out waiting for pipeline to reach playing"));
                }
                rpc = &mut play_rpc, if !play_rpc_done => {
                    play_rpc_done = true;
                    match rpc {
                        Ok(Ok(())) => {
                            info!("gstpop rtmp: play rpc accepted, waiting for playing state");
                        }
                        Ok(Err(e)) => {
                            info!("gstpop rtmp: play rpc returned error: {e:#}");
                            break Err(e);
                        }
                        Err(e) => {
                            break Err(anyhow::anyhow!("play task failed: {e}"));
                        }
                    }
                }
                event = event_rx.recv() => {
                    match event {
                        Ok(event) => {
                            info!("gstpop rtmp: event = {event:?}");
                            match event {
                                gstpop_runtime::Event::StateChanged { ref pipeline_id, ref new_state, ref old_state }
                                    if *pipeline_id == pid_for_event =>
                                {
                                    info!("gstpop rtmp: pipeline {pipeline_id} state {old_state} -> {new_state}");
                                    if new_state == "playing" {
                                        break Ok(());
                                    }
                                }
                                gstpop_runtime::Event::Error { ref pipeline_id, ref message }
                                    if *pipeline_id == pid_for_event =>
                                {
                                    error!("gstpop rtmp: pipeline error: {message}");
                                    break Err(anyhow::anyhow!("{message}"));
                                }
                                _ => {}
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            warn!("gstpop rtmp: event channel lagged by {n}");
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            warn!("gstpop rtmp: event channel closed");
                            break Err(anyhow::anyhow!("event channel closed before playing"));
                        }
                    }
                }
            }
        };

        if let Err(e) = play_result {
            error!("gstpop rtmp: play failed: {e:#}");
            let _ = client
                .call("stop", serde_json::json!({ "pipeline_id": &pid }))
                .await;
            let _ = client
                .call(
                    "remove_pipeline",
                    serde_json::json!({ "pipeline_id": &pid }),
                )
                .await;
            // Stop camera too since we started it before play.
            let _ = upcall_stop_camera_capture();
            *CAM_RTMP_GSTPOP_SESSION.lock() = None;
            return fail(&ui, format!("play: {e:#}"));
        }
        info!("gstpop rtmp: pipeline is playing");
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

    ui.global::<Bridge>()
        .on_url_looks_valid(|url: slint::SharedString| {
            url.starts_with("rtmp://") || url.starts_with("rtmps://")
        });

    ui.global::<Bridge>().on_scan_rtmp_qr({
        let ui_weak = ui.as_weak();
        let app_clone = app_clone.clone();
        move || {
            RTMP_QR_PENDING.store(true, Ordering::Release);
            if let Some(u) = ui_weak.upgrade() {
                u.global::<Bridge>().set_rtmp_qr_scanning(true);
            }
            call_java_method_no_args(&app_clone, JavaMethod::ScanQr);
        }
    });

    ui.global::<Bridge>()
        .on_request_cam_rtmp_camera_permission(move || {
            // Fire-and-forget — the OS dialog is async. The actual grant/deny
            // result comes back via nativeCameraPermissionResult → CameraEvent::PermissionResult.
            let _ = crate::jni_bridge::camera::upcall_request_camera_permission();
        });

    ui.global::<Bridge>().on_start_camera_rtmp_preview({
        let ui_weak = ui.as_weak();
        move || {
            let Some(u) = ui_weak.upgrade() else { return };
            let b = u.global::<Bridge>();
            if !b.get_cam_rtmp_camera_permission() {
                return;
            }
            let (width, height) = match b.get_resolution_idx() {
                0 => (854, 480),
                1 => (1280, 720),
                3 => (3840, 2160),
                _ => (1920, 1080),
            };
            let fps = match b.get_framerate_idx() {
                0 => 24,
                2 => 60,
                _ => 30,
            };
            if let Err(err) = upcall_start_camera_preview(
                b.get_camera_idx() as u32,
                width,
                height,
                fps,
                b.get_camera_mirror_front(),
                b.get_camera_stabilization(),
                b.get_camera_zoom_level(),
                b.get_camera_orientation_mode_idx(),
            ) {
                warn!("startCameraPreview failed: {err}");
            }
        }
    });

    ui.global::<Bridge>().on_start_camera_rtmp_stream({
        let ui_weak = ui.as_weak();
        move || {
            let ui = ui_weak.clone();

            let (
                backend,
                url,
                key,
                cam_idx,
                res_idx,
                fps_idx,
                orientation_mode,
                mirror,
                stab,
                zoom,
            ) = match ui.upgrade() {
                Some(u) => {
                    let b = u.global::<Bridge>();
                    (
                        b.get_media_backend(),
                        b.get_cam_rtmp_url().to_string(),
                        b.get_cam_rtmp_stream_key().to_string(),
                        b.get_camera_idx(),
                        b.get_resolution_idx(),
                        b.get_framerate_idx(),
                        b.get_camera_orientation_mode_idx(),
                        b.get_camera_mirror_front(),
                        b.get_camera_stabilization(),
                        b.get_camera_zoom_level(),
                    )
                }
                None => return,
            };

            clear_error(&ui);
            let _ = ui.upgrade_in_event_loop(|u| {
                u.global::<Bridge>()
                    .set_cam_rtmp_state(MixerState::Starting);
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
            let url_base = url.trim_end_matches('/');
            let full_url = if key.is_empty() {
                url_base.to_string()
            } else {
                format!("{url_base}/{key}")
            };

            let ui_clone = ui.clone();
            match backend {
                MediaBackendKind::Migration => {
                    tokio::spawn(async move {
                        // Install runtime handles so CameraSourceNode can access FRAME_PAIR.
                        // start_graph_runtime is idempotent — safe to call on every Go Live tap.
                        if let Err(e) = migration_runtime::runtime::start_graph_runtime(
                            migration_runtime::runtime::RuntimeHandles {
                                frame_pair: crate::FRAME_PAIR.clone(),
                            },
                        ) {
                            return fail(&ui_clone, format!("start_graph_runtime: {e}"));
                        }

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
                                rotation_deg: None,
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
                            orientation_mode,
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
                u.global::<Bridge>()
                    .set_cam_rtmp_state(MixerState::Stopping);
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
                        // Always tear down Migration pipeline too — user may have
                        // switched backends while a Migration stream was still live.
                        let _ = upcall_stop_camera_capture();
                        tear_down_migration();
                        stop_camera_rtmp_via_gstpop(ui).await;
                    });
                }
            }
        }
    });

    // ── SRT destination: save / start / stop (camera → SRT, Migration only) ──
    ui.global::<Bridge>().on_save_srt_destination_config({
        let ui_weak = ui.as_weak();
        move || {
            let Some(ui) = ui_weak.upgrade() else { return };
            let b = ui.global::<Bridge>();

            // Passphrase goes to the secret store, never to backend.json.
            let pass = b.get_srt_destination_passphrase().to_string();
            if let Err(e) = crate::secret::store("srt_destination_passphrase", &pass) {
                tracing::error!("Failed to store srt_destination_passphrase: {e}");
            }

            let d = b.get_srt_destination();
            let url = d.uri.to_string();
            let latency_ms = d.latency_ms;
            let pbkeylen_idx = b.get_srt_destination_pbkeylen_idx();
            if let Err(e) = crate::config::update(move |cfg| {
                cfg.srt_destination = Some(crate::config::SrtDestinationConfig {
                    url,
                    latency_ms,
                    pbkeylen_idx,
                });
            }) {
                tracing::error!("Failed to save SRT config: {e}");
            }
            tracing::info!("SRT destination settings saved");
        }
    });

    ui.global::<Bridge>().on_start_srt_destination({
        let ui_weak = ui.as_weak();
        move || {
            let ui = ui_weak.clone();

            let (
                backend,
                uri,
                latency,
                pass,
                pbkeylen_idx,
                cam_idx,
                res_idx,
                fps_idx,
                orientation_mode,
                mirror,
                stab,
                zoom,
            ) = match ui.upgrade() {
                Some(u) => {
                    let b = u.global::<Bridge>();
                    let d = b.get_srt_destination();
                    (
                        b.get_media_backend(),
                        d.uri.to_string(),
                        d.latency_ms,
                        b.get_srt_destination_passphrase().to_string(),
                        b.get_srt_destination_pbkeylen_idx(),
                        b.get_camera_idx(),
                        b.get_resolution_idx(),
                        b.get_framerate_idx(),
                        b.get_camera_orientation_mode_idx(),
                        b.get_camera_mirror_front(),
                        b.get_camera_stabilization(),
                        b.get_camera_zoom_level(),
                    )
                }
                None => return,
            };

            // SRT destination only runs on the Migration backend (gst-pop's
            // camera path is RTMP-specific).
            if backend != MediaBackendKind::Migration {
                return srt_fail(&ui, "SRT destination requires the Migration backend");
            }
            if uri.trim().is_empty() {
                return srt_fail(&ui, "SRT URL is empty");
            }

            srt_set_state(&ui, MixerState::Starting);

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
            // Encryption index → SRT pbkeylen bytes (matches the UI cycler).
            let (passphrase, pbkeylen) = match pbkeylen_idx {
                1 => (Some(pass), Some(16)),
                2 => (Some(pass), Some(24)),
                3 => (Some(pass), Some(32)),
                _ => (None, None),
            };

            let ui_clone = ui.clone();
            // Launch-fixed mode (matches the Kotlin coordinator); NOT the live selector.
            let pipeline_mode = crate::config::session_android_camera_pipeline();
            tokio::spawn(async move {
                if matches!(
                    pipeline_mode,
                    crate::config::AndroidCameraPipeline::LegacyRawI420Gstreamer
                ) {
                    if let Err(e) = migration_runtime::runtime::start_graph_runtime(
                        migration_runtime::runtime::RuntimeHandles {
                            frame_pair: crate::FRAME_PAIR.clone(),
                        },
                    ) {
                        return srt_fail(&ui_clone, format!("start_graph_runtime: {e}"));
                    }
                }

                let commands = if matches!(
                    pipeline_mode,
                    crate::config::AndroidCameraPipeline::LegacyRawI420Gstreamer
                ) {
                    vec![
                        Command::CreateCameraSource {
                            id: "srt-cam-src".into(),
                            camera_idx: cam_idx as u32,
                            width,
                            height,
                            fps,
                            mirror,
                            stabilization: stab,
                            zoom,
                            rotation_deg: None,
                        },
                        Command::CreateDestination {
                            id: "srt-dest".into(),
                            family: DestinationFamily::Srt {
                                uri: uri.clone(),
                                latency,
                                passphrase: passphrase.clone(),
                                pbkeylen,
                            },
                            audio: false,
                            video: true,
                        },
                        Command::Connect {
                            link_id: "srt-link".into(),
                            src_id: "srt-cam-src".into(),
                            sink_id: "srt-dest".into(),
                            audio: false,
                            video: true,
                            config: None,
                        },
                        Command::Start {
                            id: "srt-dest".into(),
                            cue_time: None,
                            end_time: None,
                        },
                        Command::Start {
                            id: "srt-cam-src".into(),
                            cue_time: None,
                            end_time: None,
                        },
                    ]
                } else {
                    vec![]
                };
                for cmd in commands {
                    if let CommandResult::Error(err) =
                        migration_runtime::runtime::handle_command(cmd)
                    {
                        tear_down_srt();
                        return srt_fail(&ui_clone, err);
                    }
                }

                let start_res = match pipeline_mode {
                    crate::config::AndroidCameraPipeline::LegacyRawI420Gstreamer => {
                        upcall_start_camera_capture(
                            cam_idx as u32,
                            width,
                            height,
                            fps,
                            mirror,
                            stab,
                            zoom,
                            orientation_mode,
                        )
                    }
                    crate::config::AndroidCameraPipeline::StreamPackDirectSrt
                    | crate::config::AndroidCameraPipeline::StreamPackEncodedToGstreamer => {
                        if matches!(
                            pipeline_mode,
                            crate::config::AndroidCameraPipeline::StreamPackEncodedToGstreamer
                        ) {
                            tracing::warn!(
                                "StreamPackEncodedToGstreamer is not implemented yet \
                                 (Phase 2, steps 08-10); falling back to direct StreamPack SRT"
                            );
                        }
                        let orientation_mode = match orientation_mode {
                            0 => "PORTRAIT",
                            2 => "AUTO",
                            _ => "LANDSCAPE",
                        };
                        let config_json = serde_json::json!({
                            "cameraIdx": cam_idx,
                            "width": width,
                            "height": height,
                            "maxFps": fps,
                            "mirror": mirror,
                            "stabilization": stab,
                            "zoom": zoom,
                            "orientationMode": orientation_mode,
                            "srtUrl": uri,
                        })
                        .to_string();
                        crate::jni_bridge::camera::upcall_start_streampack_camera(&config_json)
                    }
                };
                if let Err(e) = start_res {
                    tear_down_srt();
                    return srt_fail(&ui_clone, format!("start camera: {e}"));
                }
                // State flips to Running on CameraEvent::Started.
            });
        }
    });

    ui.global::<Bridge>().on_stop_srt_destination({
        let ui_weak = ui.as_weak();
        move || {
            let ui = ui_weak.clone();
            srt_set_state(&ui, MixerState::Stopping);
            tokio::spawn(async move {
                if let Err(err) = upcall_stop_camera_capture() {
                    srt_fail(&ui, format!("stopCameraCapture: {err}"));
                    return;
                }

                // Only the legacy path builds GStreamer SRT graph nodes; tearing them
                // down in StreamPack mode would touch a runtime that was never started.
                // Use the launch-fixed session mode so this can't diverge from start().
                if matches!(
                    crate::config::session_android_camera_pipeline(),
                    crate::config::AndroidCameraPipeline::LegacyRawI420Gstreamer
                ) {
                    tear_down_srt();
                }

                let _ = ui.upgrade_in_event_loop(|u| {
                    let b = u.global::<Bridge>();
                    let mut d = b.get_srt_destination();
                    d.state = MixerState::Idle;
                    d.last_error = "".into();
                    b.set_srt_destination(d);
                });
            });
        }
    });

    // ── Listen to camera lifecycle events from Kotlin via JNI ──
    let cam_event_ui_weak = ui.as_weak();
    std::thread::spawn(move || loop {
        match crate::GLOB_CAMERA_EVENT_CHAN.1.recv() {
            Ok(event) => match event {
                crate::jni_bridge::main_activity::CameraEvent::Started {
                    width,
                    height,
                    rotation_deg,
                } => {
                    tracing::info!("camera capture started {width}x{height} rot={rotation_deg}°");
                    migration_runtime::runtime::update_camera_source_rotation(rotation_deg);
                    let _ = cam_event_ui_weak.upgrade_in_event_loop(|u| {
                        let b = u.global::<Bridge>();
                        b.set_cam_rtmp_camera_permission(true);
                        let mut srt = b.get_srt_destination();
                        if srt.state == MixerState::Starting {
                            srt.state = MixerState::Running;
                            b.set_srt_destination(srt);
                        } else {
                            b.set_cam_rtmp_state(MixerState::Running);
                        }
                    });
                }
                crate::jni_bridge::main_activity::CameraEvent::Stopped => {
                    tracing::info!("camera capture stopped");
                }
                crate::jni_bridge::main_activity::CameraEvent::PermissionResult { granted } => {
                    tracing::info!("camera permission result: granted={granted}");
                    let _ = cam_event_ui_weak.upgrade_in_event_loop(move |u| {
                        let b = u.global::<Bridge>();
                        b.set_cam_rtmp_camera_permission(granted);
                        if granted
                            && u.global::<PanelBridge>().get_active() == Panel::CameraRtmpStream
                        {
                            b.invoke_start_camera_rtmp_preview();
                        }
                    });
                }
                crate::jni_bridge::main_activity::CameraEvent::Failed { reason } => {
                    tracing::warn!("camera capture failed: {reason}");

                    let srt_active = cam_event_ui_weak.upgrade().map(|u| {
                        let st = u.global::<Bridge>().get_srt_destination().state;
                        st == MixerState::Starting || st == MixerState::Running
                    });

                    if srt_active == Some(true) {
                        tear_down_srt();
                        srt_fail(&cam_event_ui_weak, reason.clone());
                    } else {
                        let backend = cam_event_ui_weak
                            .upgrade()
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

                        fail(&cam_event_ui_weak, reason.clone());
                    }

                    if reason == "Camera permission denied" {
                        let _ = cam_event_ui_weak.upgrade_in_event_loop(|u| {
                            u.global::<Bridge>().set_cam_rtmp_camera_permission(false);
                        });
                    }
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
