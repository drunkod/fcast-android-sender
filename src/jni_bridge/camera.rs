//! JNI bridge — camera upcalls.

#[cfg(target_os = "android")]
use jni::objects::{JString, JValue};

#[cfg(target_os = "android")]
pub fn upcall_start_camera_capture(
    camera_idx: u32,
    w: u32,
    h: u32,
    fps: u32,
    mirror: bool,
    stabilization: bool,
    zoom: f32,
    orientation_mode: i32,
) -> Result<(), String> {
    let ctx = crate::android_context().map_err(|e| e.to_string())?;
    let mut env = ctx.vm.attach_current_thread().map_err(|e| e.to_string())?;
    env.call_method(
        &ctx.activity,
        "startCameraCapture",
        "(IIIIZZFI)V",
        &[
            JValue::Int(camera_idx as i32),
            JValue::Int(w as i32),
            JValue::Int(h as i32),
            JValue::Int(fps as i32),
            JValue::Bool(u8::from(mirror)),
            JValue::Bool(u8::from(stabilization)),
            JValue::Float(zoom),
            JValue::Int(orientation_mode),
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(not(target_os = "android"))]
pub fn upcall_start_camera_capture(
    _camera_idx: u32,
    _w: u32,
    _h: u32,
    _fps: u32,
    _mirror: bool,
    _stabilization: bool,
    _zoom: f32,
    _orientation_mode: i32,
) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "android")]
pub fn upcall_start_streampack_camera(config_json: &str) -> Result<(), String> {
    let ctx = crate::android_context().map_err(|e| e.to_string())?;
    let mut env = ctx.vm.attach_current_thread().map_err(|e| e.to_string())?;
    let j: JString = env.new_string(config_json).map_err(|e| e.to_string())?;
    env.call_method(
        &ctx.activity,
        "startStreamPackCamera",
        "(Ljava/lang/String;)V",
        &[JValue::Object(&j.into())],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(not(target_os = "android"))]
pub fn upcall_start_streampack_camera(_config_json: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "android")]
pub fn upcall_stop_camera_capture() -> Result<(), String> {
    let ctx = crate::android_context().map_err(|e| e.to_string())?;
    let mut env = ctx.vm.attach_current_thread().map_err(|e| e.to_string())?;
    env.call_method(&ctx.activity, "stopCameraCapture", "()V", &[])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(not(target_os = "android"))]
pub fn upcall_stop_camera_capture() -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "android")]
pub fn upcall_start_camera_preview(
    camera_idx: u32,
    w: u32,
    h: u32,
    fps: u32,
    mirror: bool,
    stabilization: bool,
    zoom: f32,
    orientation_mode: i32,
) -> Result<(), String> {
    let ctx = crate::android_context().map_err(|e| e.to_string())?;
    let mut env = ctx.vm.attach_current_thread().map_err(|e| e.to_string())?;
    env.call_method(
        &ctx.activity,
        "startCameraPreview",
        "(IIIIZZFI)V",
        &[
            JValue::Int(camera_idx as i32),
            JValue::Int(w as i32),
            JValue::Int(h as i32),
            JValue::Int(fps as i32),
            JValue::Bool(u8::from(mirror)),
            JValue::Bool(u8::from(stabilization)),
            JValue::Float(zoom),
            JValue::Int(orientation_mode),
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(not(target_os = "android"))]
pub fn upcall_start_camera_preview(
    _camera_idx: u32,
    _w: u32,
    _h: u32,
    _fps: u32,
    _mirror: bool,
    _stabilization: bool,
    _zoom: f32,
    _orientation_mode: i32,
) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "android")]
pub fn upcall_stop_camera_preview() -> Result<(), String> {
    let ctx = crate::android_context().map_err(|e| e.to_string())?;
    let mut env = ctx.vm.attach_current_thread().map_err(|e| e.to_string())?;
    env.call_method(&ctx.activity, "stopCameraPreview", "()V", &[])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(not(target_os = "android"))]
pub fn upcall_stop_camera_preview() -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "android")]
pub fn upcall_probe_camera_permission() -> Result<bool, String> {
    let ctx = crate::android_context().map_err(|e| e.to_string())?;
    let mut env = ctx.vm.attach_current_thread().map_err(|e| e.to_string())?;
    let res = env
        .call_method(&ctx.activity, "probeCameraPermission", "()Z", &[])
        .map_err(|e| e.to_string())?;
    res.z().map_err(|e| e.to_string())
}

#[cfg(not(target_os = "android"))]
pub fn upcall_probe_camera_permission() -> Result<bool, String> {
    Ok(true)
}

#[cfg(target_os = "android")]
pub fn upcall_request_camera_permission() -> Result<(), String> {
    // Delegates to the Kotlin `requestCameraPermission()` helper which uses
    // the canonical `MainActivity.REQ_CAMERA_PERM` constant — keeps the
    // request code defined in exactly one place.
    let ctx = crate::android_context().map_err(|e| e.to_string())?;
    let mut env = ctx.vm.attach_current_thread().map_err(|e| e.to_string())?;
    env.call_method(&ctx.activity, "requestCameraPermission", "()V", &[])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(not(target_os = "android"))]
pub fn upcall_request_camera_permission() -> Result<(), String> {
    Ok(())
}
