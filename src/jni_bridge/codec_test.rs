//! JNI bridge — codec test upcalls into Kotlin CodecDump.

#[cfg(target_os = "android")]
use crate::jni_bridge::helpers::{load_app_class, vm};

#[cfg(target_os = "android")]
const CODEC_DUMP_CLASS: &str = "org/fcast/android/sender/codec/CodecDump";

#[cfg(target_os = "android")]
fn call_static_string_method(method_name: &str) -> Result<String, String> {
    let vm = vm();
    let mut env = vm
        .attach_current_thread()
        .map_err(|e| format!("attach: {e}"))?;

    let class = load_app_class(&mut env, CODEC_DUMP_CLASS)
        .map_err(|e| format!("load_app_class({CODEC_DUMP_CLASS}): {e}"))?;

    let result = env
        .call_static_method(class, method_name, "()Ljava/lang/String;", &[])
        .map_err(|e| format!("call_static_method {method_name}: {e}"))?;

    let jstr = result
        .l()
        .map_err(|e| format!("{method_name} result not an object: {e}"))?;

    if jstr.is_null() {
        return Ok(String::new());
    }

    let jstring = jni::objects::JString::from(jstr);
    let rust_str = env
        .get_string(&jstring)
        .map_err(|e| format!("get_string: {e}"))?
        .to_string_lossy()
        .to_string();

    Ok(rust_str)
}

#[cfg(target_os = "android")]
pub fn run_codec_dump_all() -> Result<String, String> {
    call_static_string_method("dumpAllCodecsToLog")
}

#[cfg(target_os = "android")]
pub fn run_codec_quick_find() -> Result<String, String> {
    call_static_string_method("quickFindCodecsForFormats")
}

#[cfg(target_os = "android")]
pub fn run_codec_smoke_test() -> Result<String, String> {
    call_static_string_method("smokeTestVideoEncoders")
}

// ── Non-Android stubs (host builds / tests) ─────────────────────────────

#[cfg(not(target_os = "android"))]
pub fn run_codec_dump_all() -> Result<String, String> {
    Ok("codec dump not available on this platform\n".into())
}

#[cfg(not(target_os = "android"))]
pub fn run_codec_quick_find() -> Result<String, String> {
    Ok("codec quick-find not available on this platform\n".into())
}

#[cfg(not(target_os = "android"))]
pub fn run_codec_smoke_test() -> Result<String, String> {
    Ok("codec smoke test not available on this platform\n".into())
}
