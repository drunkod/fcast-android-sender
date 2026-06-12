//! Shared helpers used by every jni_bridge::* module.
//! Extracted from src/lib.rs as part of refactor step 07.

#[cfg(target_os = "android")]
use anyhow::{bail, Result};
#[cfg(target_os = "android")]
use gst::prelude::{BufferPoolExt, BufferPoolExtManual};
#[cfg(target_os = "android")]
use gst_video::{VideoColorimetry, VideoFrameExt};
#[cfg(target_os = "android")]
use jni::{
    objects::{GlobalRef, JByteBuffer, JClass, JObject, JString, JValue},
    JavaVM,
};
#[cfg(target_os = "android")]
use once_cell::sync::OnceCell;
#[cfg(target_os = "android")]
use slint::ComponentHandle;
#[cfg(target_os = "android")]
use std::path::PathBuf;
#[cfg(target_os = "android")]
use std::sync::Arc;
#[cfg(target_os = "android")]
use tracing::{error, info, warn};

use crate::platform::platform_app::PlatformApp;
#[cfg(target_os = "android")]
use mcore::Event;

#[cfg(target_os = "android")]
pub(crate) fn jstring_to_string<'local>(
    env: &mut jni::JNIEnv<'local>,
    s: &JString<'local>,
) -> Result<String> {
    Ok(env.get_string(s)?.to_string_lossy().to_string())
}

#[cfg(target_os = "android")]
static VM: OnceCell<Arc<JavaVM>> = OnceCell::new();

#[cfg(target_os = "android")]
pub(crate) fn init_vm(vm: JavaVM) -> Arc<JavaVM> {
    let vm = Arc::new(vm);
    if VM.set(vm.clone()).is_err() {
        warn!("init_vm called twice; keeping the first JavaVM handle");
    }
    VM.get()
        .expect("JavaVM missing immediately after init_vm")
        .clone()
}

#[cfg(target_os = "android")]
pub(crate) fn vm() -> Arc<JavaVM> {
    VM.get()
        .expect("JavaVM not initialised; call init_vm() from android_main")
        .clone()
}

// Cached ClassLoader from the main thread so background-thread JNI can find
// app classes. Android's bootstrap classloader (used on attached native
// threads) can only see system classes; app dex classes must be loaded via
// the app ClassLoader obtained on the UI thread.
#[cfg(target_os = "android")]
static APP_CLASS_LOADER: OnceCell<GlobalRef> = OnceCell::new();

/// Cache the app ClassLoader using the live Activity instance.
///
/// NativeActivity exposes the Activity object via `ANativeActivity::clazz`
/// (despite the name it is the *instance*, not the class). Calling
/// `getClassLoader()` on it gives the full app dex classloader, which works
/// even from JNI-attached native threads where `env.find_class()` only sees
/// the bootstrap loader.
///
/// Must be called early in `android_main` before any background JNI work.
#[cfg(target_os = "android")]
pub(crate) fn cache_app_class_loader(env: &mut jni::JNIEnv<'_>, activity: &JObject<'_>) {
    let Ok(loader_val) =
        env.call_method(activity, "getClassLoader", "()Ljava/lang/ClassLoader;", &[])
    else {
        warn!("cache_app_class_loader: getClassLoader failed on activity — app class lookup will fail on native threads");
        return;
    };
    let Ok(loader_obj) = loader_val.l() else {
        warn!("cache_app_class_loader: loader is not an object");
        return;
    };
    match env.new_global_ref(&loader_obj) {
        Ok(global) => {
            let _ = APP_CLASS_LOADER.set(global);
        }
        Err(e) => warn!(?e, "cache_app_class_loader: new_global_ref failed"),
    }
}

/// Find an app class by its JNI slash-separated name (e.g.
/// `"org/fcast/android/sender/data/SecretStoreBridge"`).
/// Works on any thread, including JNI-attached native threads.
#[cfg(target_os = "android")]
pub(crate) fn load_app_class<'local>(
    env: &mut jni::JNIEnv<'local>,
    name: &str,
) -> jni::errors::Result<JClass<'local>> {
    if let Some(loader) = APP_CLASS_LOADER.get() {
        let dot_name = name.replace('/', ".");
        let dot_name_j = env.new_string(&dot_name)?;
        let class_obj = env
            .call_method(
                loader.as_obj(),
                "loadClass",
                "(Ljava/lang/String;)Ljava/lang/Class;",
                &[JValue::Object(&dot_name_j.into())],
            )?
            .l()?;
        Ok(JClass::from(class_obj))
    } else {
        // Fallback (works only on the main thread)
        env.find_class(name)
    }
}

#[derive(Debug)]
pub(crate) enum JavaMethod {
    StopCapture,
    ScanQr,
    FinishApp,
}

#[cfg(target_os = "android")]
pub(crate) fn call_java_method_no_args(app: &PlatformApp, method: JavaMethod) {
    let vm = vm();
    let ptr = app.activity_as_ptr() as *mut jni::sys::_jobject;
    assert!(!ptr.is_null(), "Activity ptr is null");
    // SAFETY: PlatformApp owns the Android activity handle for the lifetime of
    // the Slint Android runtime. This helper only creates a local wrapper for
    // the immediate call on the current UI callback.
    let activity = unsafe { JObject::from_raw(ptr) };

    let method_name = match method {
        JavaMethod::StopCapture => "stopCapture",
        JavaMethod::ScanQr => "scanQr",
        JavaMethod::FinishApp => "finishApp",
    };

    match vm.get_env() {
        Ok(mut env) => match env.call_method(activity, method_name, "()V", &[]) {
            Ok(_) => (),
            Err(err) => error!(?err, ?method, "Failed to call java method"),
        },
        Err(err) => error!(?err, "Failed to get env from VM"),
    }
}

#[cfg(not(target_os = "android"))]
pub(crate) fn call_java_method_no_args(_app: &PlatformApp, _method: JavaMethod) {}

#[cfg(target_os = "android")]
pub(crate) fn handle_back_request(ui: &crate::MainWindow, app: Option<&PlatformApp>) {
    let bridge = ui.global::<crate::Bridge>();

    if ui.global::<crate::PanelBridge>().get_active() != crate::Panel::None {
        ui.global::<crate::PanelBridge>().invoke_pop();
        return;
    }

    if bridge.get_lifecycle() != crate::LifecycleMode::Normal {
        bridge.set_lifecycle(crate::LifecycleMode::Normal);
        return;
    }

    match bridge.get_app_state() {
        crate::AppState::Disconnected => {
            if let Some(app) = app {
                call_java_method_no_args(app, JavaMethod::FinishApp);
            } else {
                warn!("Ignoring back press in disconnected state without Android app handle");
            }
        }
        crate::AppState::Connecting | crate::AppState::SelectingSettings => {
            bridge.invoke_change_state(crate::AppState::Disconnected);
        }
        crate::AppState::WaitingForMedia | crate::AppState::Casting => {
            if let Err(err) = crate::GLOB_EVENT_CHAN
                .0
                .send(Event::EndSession { disconnect: true })
            {
                error!(?err, "Failed to send back-requested end-session event");
            }
        }
    }
}

#[cfg(target_os = "android")]
pub(crate) fn resolve_android_files_dir(app: &PlatformApp) -> Result<PathBuf> {
    let vm = vm();
    let ptr = app.activity_as_ptr() as *mut jni::sys::_jobject;
    assert!(!ptr.is_null(), "Activity ptr is null");
    // SAFETY: PlatformApp exposes the live Activity object owned by the Slint
    // Android runtime. The wrapper is used only while resolving the files dir
    // on the current thread and is not retained.
    let activity = unsafe { JObject::from_raw(ptr) };

    let mut env = vm.get_env()?;
    let files_dir = env
        .call_method(&activity, "getFilesDir", "()Ljava/io/File;", &[])?
        .l()?;
    let absolute_path = env
        .call_method(files_dir, "getAbsolutePath", "()Ljava/lang/String;", &[])?
        .l()?;
    let absolute_path = JString::from(absolute_path);
    let absolute_path = env
        .get_string(&absolute_path)?
        .to_string_lossy()
        .to_string();

    Ok(PathBuf::from(absolute_path))
}

#[cfg(target_os = "android")]
static PROCESS_FRAME_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Debug: dump a raw I420 frame to `<files_dir>/dump/`. Toggle at runtime (no
/// rebuild) by creating the marker file `<files_dir>/dump/on`. Captures the
/// first 5 frames + one every ~4s so the dump stays small. This is the camera →
/// GL → native output, *before* any GStreamer videocrop/videoflip, so it
/// isolates the GL YUV conversion from later pipeline stages.
///
/// View on host (after `adb pull`):
///   gst-launch-1.0 filesrc location=cam_1920x1080_00003.i420 \
///     ! rawvideoparse width=1920 height=1080 format=i420 \
///     ! videoconvert ! pngenc ! filesink location=cam.png
#[cfg(target_os = "android")]
fn maybe_dump_i420(tag: &str, count: u64, width: usize, height: usize, y: &[u8], u: &[u8], v: &[u8]) {
    if count >= 5 && count % 120 != 0 {
        return;
    }
    let Some(dir) = crate::config::get_files_dir() else {
        return;
    };
    let dump_dir = dir.join("dump");
    if !dump_dir.join("on").exists() {
        return; // marker absent → disabled
    }
    if std::fs::create_dir_all(&dump_dir).is_err() {
        return;
    }
    let path = dump_dir.join(format!("{tag}_{width}x{height}_{count:05}.i420"));
    if let Ok(mut f) = std::fs::File::create(&path) {
        use std::io::Write;
        let _ = f.write_all(y);
        let _ = f.write_all(u);
        let _ = f.write_all(v);
        info!("dumped camera frame to {}", path.display());
    }
}

#[cfg(target_os = "android")]
pub(crate) fn process_frame<'local>(
    env: jni::JNIEnv<'local>,
    width: jni::sys::jint,
    height: jni::sys::jint,
    timestamp_ns: jni::sys::jlong,
    buffer_y: JByteBuffer<'local>,
    buffer_u: JByteBuffer<'local>,
    buffer_v: JByteBuffer<'local>,
) -> Result<()> {
    let width = width as usize;
    let height = height as usize;

    let pf_count = PROCESS_FRAME_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if pf_count < 10 || pf_count % 30 == 0 {
        info!("process_frame: count={pf_count} {width}x{height}");
    }

    fn buffer_as_slice<'local>(
        env: &jni::JNIEnv<'local>,
        buffer: &JByteBuffer<'local>,
        size: usize,
    ) -> Result<&'local [u8]> {
        let buffer_cap = match env.get_direct_buffer_capacity(&buffer) {
            Ok(cap) => cap,
            Err(err) => {
                bail!("Failed to get capacity of the byte buffer: {err}");
            }
        };

        if buffer_cap < size {
            bail!("buffer_cap < size: {buffer_cap} < {size}");
        }

        let buffer_ptr = match env.get_direct_buffer_address(&buffer) {
            Ok(ptr) => {
                assert!(!ptr.is_null());
                ptr
            }
            Err(err) => {
                bail!("Failed to get buffer address: {err}");
            }
        };

        // SAFETY: get_direct_buffer_address/capacity came from the same live
        // DirectByteBuffer local reference, and callers pass the buffer through
        // JNI for the duration of this native frame callback.
        unsafe { Ok(std::slice::from_raw_parts(buffer_ptr, buffer_cap)) }
    }

    let slice_y = buffer_as_slice(&env, &buffer_y, width * height)?;
    let slice_u = buffer_as_slice(&env, &buffer_u, (width / 2) * (height / 2))?;
    let slice_v = buffer_as_slice(&env, &buffer_v, (width / 2) * (height / 2))?;

    // Debug: dump the raw camera frame (GL conversion output) before videocrop.
    maybe_dump_i420("cam", pf_count, width, height, slice_y, slice_u, slice_v);

    let info = match gst_video::VideoInfo::builder(
        gst_video::VideoFormat::I420,
        width as u32,
        height as u32,
    )
    .colorimetry(&VideoColorimetry::new(
        gst_video::VideoColorRange::Range0_255,
        gst_video::VideoColorMatrix::Bt709,
        gst_video::VideoTransferFunction::Bt709,
        gst_video::VideoColorPrimaries::Bt709,
    ))
    .build()
    {
        Ok(info) => info,
        Err(err) => {
            bail!("Failed to crate video info: {err}");
        }
    };

    let new_caps = match info.to_caps() {
        Ok(caps) => caps,
        Err(err) => {
            bail!("Failed to create caps from video info: {err}");
        }
    };

    fn init_frame_pool(
        pool: &gst_video::VideoBufferPool,
        mut old_config: gst::BufferPoolConfig,
        new_caps: &gst::Caps,
        frame_size: u32,
    ) -> Result<()> {
        pool.set_config({
            old_config.set_params(Some(&new_caps), frame_size, 1, 30);
            old_config
        })?;
        pool.set_active(true)?;
        Ok(())
    }

    let frame_pool = crate::FRAME_POOL.lock();
    let frame_size = width * height + 2 * ((width / 2) * (height / 2));
    let needs_reconfigure = if !frame_pool.is_active() {
        true
    } else {
        match frame_pool.config().params() {
            Some((caps, size, _, _)) => {
                caps.as_ref() != Some(&new_caps) || size != frame_size as u32
            }
            None => true,
        }
    };
    if needs_reconfigure {
        let old_config = frame_pool.config();
        if frame_pool.is_active() {
            let _ = frame_pool.set_active(false);
        }
        init_frame_pool(&frame_pool, old_config, &new_caps, frame_size as u32)?;
    }

    let buffer = match frame_pool.acquire_buffer(None) {
        Ok(buffer) => buffer,
        Err(err) => {
            error!("process_frame: acquire_buffer failed at count={pf_count}: {err}");
            bail!("Failed to acquire buffer from pool: {err}");
        }
    };
    let mut buffer = buffer;
    if timestamp_ns > 0 {
        if let Some(buffer_mut) = buffer.get_mut() {
            buffer_mut.set_pts(gst::ClockTime::from_nseconds(timestamp_ns as u64));
        } else {
            warn!("process_frame: acquired buffer was not writable for pts assignment");
        }
    }
    let Ok(mut vframe) = gst_video::VideoFrame::from_buffer_writable(buffer, &info) else {
        bail!("Failed to crate VideoFrame from buffer");
    };

    fn copy(
        vframe: &mut gst_video::VideoFrame<gst_video::video_frame::Writable>,
        plane_idx: u32,
        src_plane: &[u8],
    ) -> Result<()> {
        let dest_y_stride = *vframe
            .plane_stride()
            .get(plane_idx as usize)
            .ok_or(anyhow::anyhow!("Could not get plane stride"))?
            as usize;
        let dest_y = vframe.plane_data_mut(plane_idx)?;
        for (dest, src) in dest_y
            .chunks_exact_mut(dest_y_stride)
            .zip(src_plane.chunks_exact(dest_y_stride))
        {
            dest[..dest_y_stride].copy_from_slice(&src[..dest_y_stride]);
        }

        Ok(())
    }

    copy(&mut vframe, 0, slice_y)?;
    copy(&mut vframe, 1, slice_u)?;
    copy(&mut vframe, 2, slice_v)?;

    let mut frame = crate::FRAME_PAIR.frame.lock();
    *frame = Some(vframe);
    crate::FRAME_PAIR.cond.notify_one();

    Ok(())
}
