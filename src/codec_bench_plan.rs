//! JSON benchmark plan executed inside the :codec_bench service process.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodecBenchRequest {
    pub include_factory_list: bool,
    pub include_encode_perf: bool,
    pub include_decode_perf: bool,
    /// If true, the caller should kill :codec_bench after the result is delivered
    /// (releases all HW codec + EGL state torn up by the decode tests).
    pub kill_process_after_decode: bool,
    /// Optional GStreamer 1.26+ foreign-EGL guard (Step 9). Ignored if unsupported.
    pub use_foreign_egl: bool,
}

impl Default for CodecBenchRequest {
    fn default() -> Self {
        Self {
            include_factory_list: true,
            include_encode_perf: true,
            include_decode_perf: false,
            kill_process_after_decode: true,
            use_foreign_egl: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodecBenchResponse {
    pub ok: bool,
    pub report: String,
    pub ran_decode: bool,
    pub should_kill_process: bool,
    pub error: Option<String>,
}

impl CodecBenchResponse {
    pub fn ok(report: String, ran_decode: bool, should_kill_process: bool) -> Self {
        Self {
            ok: true,
            report,
            ran_decode,
            should_kill_process,
            error: None,
        }
    }

    pub fn error(error: impl Into<String>) -> Self {
        Self {
            ok: false,
            report: String::new(),
            ran_decode: false,
            should_kill_process: false,
            error: Some(error.into()),
        }
    }
}

/// Entry point called from JNI (Step 3). Never panics across the FFI boundary.
pub fn run_benchmark_plan_json(request_json: &str) -> String {
    let request: CodecBenchRequest = match serde_json::from_str(request_json) {
        Ok(v) => v,
        Err(e) => {
            let resp = CodecBenchResponse::error(format!("Invalid request JSON: {e}"));
            return serde_json::to_string(&resp).unwrap_or_else(|_| "{}".into());
        }
    };

    let response = match std::panic::catch_unwind(|| run_benchmark_plan(request)) {
        Ok(resp) => resp,
        Err(_) => CodecBenchResponse::error("Rust benchmark panicked"),
    };

    serde_json::to_string(&response).unwrap_or_else(|e| {
        format!(
            "{{\"ok\":false,\"report\":\"\",\"ranDecode\":false,\"shouldKillProcess\":false,\"error\":\"serialize failed: {e}\"}}"
        )
    })
}

pub fn run_benchmark_plan(request: CodecBenchRequest) -> CodecBenchResponse {
    use crate::codec_perf::{self, ProcessSafety};

    let mut report = String::new();

    if request.include_factory_list {
        report.push_str(&codec_perf::list_codec_factories());
        report.push('\n');
    }
    if request.include_encode_perf {
        report.push_str(&codec_perf::run_encode_benchmarks());
        report.push('\n');
    }

    // Decode LAST: if amcviddec damages EGL state we already have factory + encode.
    if request.include_decode_perf {
        let _ = request.use_foreign_egl;

        report.push_str(&codec_perf::run_decode_benchmarks_checked(
            ProcessSafety::IsolatedBenchmarkProcess,
        ));
        report.push('\n');
    }

    report.push_str(&codec_perf::encoder_recommendation());

    CodecBenchResponse::ok(
        report,
        request.include_decode_perf,
        request.include_decode_perf && request.kill_process_after_decode,
    )
}
