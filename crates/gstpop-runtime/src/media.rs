//! Android-safe media path & playbin description helpers.
//!
//! Upstream `gstpop::gst::discoverer::normalize_uri` resolves bare paths
//! against `std::env::current_dir()`. That is correct for desktop CLI use
//! but wrong for Android, where the working directory is typically `/` and
//! app-owned files live under `Context.filesDir` /
//! `Context.getExternalFilesDir(null)`. This module requires the caller to
//! supply an explicit base directory on Android.

use anyhow::{bail, Result};
use std::path::{Path, PathBuf};

/// Normalise a user-provided media input into a `file://` or pass-through URI.
///
/// - If `input` already contains `://`, returned as-is.
/// - If `input` is absolute, used directly.
/// - If `input` is relative and `base_dir` is `Some`, joined onto it.
/// - If `input` is relative and `base_dir` is `None`, falls back to
///   `std::env::current_dir()` (desktop only; never pass `None` from Android).
pub fn normalise_media_input(input: &str, base_dir: Option<&Path>) -> Result<String> {
    let input = input.trim();
    if input.is_empty() {
        bail!("empty media input");
    }
    if input.contains("://") {
        return Ok(input.to_owned());
    }

    let path = Path::new(input);
    let absolute: PathBuf = if path.is_absolute() {
        path.to_path_buf()
    } else if let Some(base) = base_dir {
        base.join(path)
    } else {
        #[cfg(target_os = "android")]
        {
            bail!(
                "relative media path {input:?} requires an explicit base_dir on Android"
            );
        }
        #[cfg(not(target_os = "android"))]
        {
            std::env::current_dir()?.join(path)
        }
    };

    // canonicalize is best-effort: paths that don't exist yet (e.g. about to
    // be written) should still produce a valid file:// URI.
    let absolute = absolute.canonicalize().unwrap_or(absolute);
    let path_str = absolute.to_string_lossy().replace('\\', "/");

    if path_str.starts_with('/') {
        Ok(format!("file://{path_str}"))
    } else {
        // Windows drive-letter form, e.g. C:/Users/...
        Ok(format!("file:///{path_str}"))
    }
}

/// Build a `playbin` / `playbin3` description string for `gst-launch`-style
/// pipeline creation. Validates sink element names to avoid shell-style
/// injection into the pipeline description grammar.
pub fn build_playbin_description(
    input: &str,
    base_dir: Option<&Path>,
    video_sink: Option<&str>,
    audio_sink: Option<&str>,
    use_legacy_playbin: bool,
) -> Result<String> {
    let uri = normalise_media_input(input, base_dir)?;
    if let Some(vs) = video_sink {
        validate_sink_name(vs)?;
    }
    if let Some(asink) = audio_sink {
        validate_sink_name(asink)?;
    }

    let element = if use_legacy_playbin { "playbin" } else { "playbin3" };
    let escaped_uri = uri.replace('"', r#"\""#);

    let mut desc = format!(r#"{element} uri="{escaped_uri}""#);
    if let Some(vs) = video_sink {
        desc.push_str(&format!(" video-sink={vs}"));
    }
    if let Some(asink) = audio_sink {
        desc.push_str(&format!(" audio-sink={asink}"));
    }
    Ok(desc)
}

fn validate_sink_name(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("empty sink element name");
    }
    const FORBIDDEN: &[char] = &['!', '"', '\'', '`', '\\', ' ', '\t', '\n'];
    if name.contains(FORBIDDEN) {
        bail!("invalid sink element name: {name}");
    }
    Ok(())
}

use std::time::Duration;

pub use gstpop::gst::discoverer::{
    AudioStreamInfo, ContainerInfo, DiscoverResult, SubtitleStreamInfo, TagsInfo, VideoStreamInfo,
};

/// Discover media metadata for `input`, normalising the path first.
///
/// `base_dir` follows the same rule as
/// [`normalise_media_input`]: `Some(...)` is required on Android.
/// `timeout` defaults to 5 seconds when `None`.
pub async fn discover(
    input: &str,
    base_dir: Option<&Path>,
    timeout: Option<Duration>,
) -> Result<DiscoverResult> {
    let uri = normalise_media_input(input, base_dir)?;
    let timeout_secs = timeout.map(|t| t.as_secs() as u32);

    // Run the synchronous gstreamer discoverer on a blocking thread so we
    // don't stall the tokio runtime.
    let uri_for_blocking = uri.clone();
    let info = tokio::task::spawn_blocking(move || -> Result<DiscoverResult> {
        gstpop::gst::discoverer::discover_uri(&uri_for_blocking, timeout_secs)
            .map_err(|e| anyhow::anyhow!("discover failed: {e:#}"))
    })
    .await
    .map_err(|e| anyhow::anyhow!("discoverer join failed: {e}"))??;

    Ok(info)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn passthrough_uri() {
        assert_eq!(
            normalise_media_input("https://example.com/a.mp4", None).unwrap(),
            "https://example.com/a.mp4",
        );
    }

    #[test]
    fn absolute_unix_path() {
        let out = normalise_media_input("/tmp/x.mp4", None).unwrap();
        assert!(out.starts_with("file:///") || out.starts_with("file://"), "got {out}");
        assert!(out.ends_with("/x.mp4"), "got {out}");
    }

    #[test]
    fn relative_with_base() {
        let base = PathBuf::from("/data/user/0/org.fcast.sender/files");
        let out = normalise_media_input("clip.mp4", Some(&base)).unwrap();
        assert_eq!(out, "file:///data/user/0/org.fcast.sender/files/clip.mp4");
    }

    #[test]
    #[cfg(target_os = "android")]
    fn relative_without_base_errors_on_android() {
        assert!(normalise_media_input("clip.mp4", None).is_err());
    }

    #[test]
    fn empty_input_errors() {
        assert!(normalise_media_input("   ", None).is_err());
    }

    #[test]
    fn rejects_bad_sink_names() {
        assert!(build_playbin_description(
            "/tmp/a.mp4", None, Some("bad sink"), None, false
        ).is_err());
        assert!(build_playbin_description(
            "/tmp/a.mp4", None, Some("with\"quote"), None, false
        ).is_err());
    }

    #[test]
    fn builds_playbin3_description() {
        let desc = build_playbin_description(
            "/tmp/a.mp4",
            None,
            Some("autovideosink"),
            Some("autoaudiosink"),
            false,
        )
        .unwrap();
        assert!(desc.starts_with("playbin3 uri=\"file://"), "got {desc}");
        assert!(desc.contains("video-sink=autovideosink"));
        assert!(desc.contains("audio-sink=autoaudiosink"));
    }

    #[test]
    fn various_uri_schemes_pass_through() {
        for uri in [
            "rtsp://192.168.1.1/stream",
            "rtp://239.0.0.1:5004",
            "srt://host:1234",
            "http://example.com/video.mp4",
            "ftp://files.example.com/a.mkv",
        ] {
            assert_eq!(normalise_media_input(uri, None).unwrap(), uri, "scheme {uri} must pass through");
        }
    }

    #[test]
    fn whitespace_around_uri_is_trimmed_before_passthrough() {
        let out = normalise_media_input("  https://example.com/a.mp4  ", None).unwrap();
        assert_eq!(out, "https://example.com/a.mp4");
    }

    #[test]
    fn relative_with_deep_base_dir() {
        let base = PathBuf::from("/data/user/0/org.fcast.sender/files");
        let out = normalise_media_input("subdir/clip.mp4", Some(&base)).unwrap();
        assert_eq!(out, "file:///data/user/0/org.fcast.sender/files/subdir/clip.mp4");
    }

    #[test]
    fn build_playbin_no_sinks_produces_minimal_description() {
        let desc = build_playbin_description("/tmp/a.mp4", None, None, None, false).unwrap();
        assert!(desc.starts_with("playbin3 uri=\"file://"), "got {desc}");
        assert!(!desc.contains("video-sink"), "got {desc}");
        assert!(!desc.contains("audio-sink"), "got {desc}");
    }

    #[test]
    fn build_playbin_legacy_uses_playbin_element() {
        let desc = build_playbin_description("/tmp/a.mp4", None, None, None, true).unwrap();
        assert!(desc.starts_with("playbin uri="), "got {desc}");
        assert!(!desc.contains("playbin3"), "got {desc}");
    }

    #[test]
    fn build_playbin_only_video_sink() {
        let desc = build_playbin_description("/tmp/a.mp4", None, Some("autovideosink"), None, false).unwrap();
        assert!(desc.contains("video-sink=autovideosink"));
        assert!(!desc.contains("audio-sink"), "got {desc}");
    }

    #[test]
    fn build_playbin_only_audio_sink() {
        let desc = build_playbin_description("/tmp/a.mp4", None, None, Some("autoaudiosink"), false).unwrap();
        assert!(desc.contains("audio-sink=autoaudiosink"));
        assert!(!desc.contains("video-sink"), "got {desc}");
    }

    #[test]
    fn build_playbin_escapes_double_quote_in_uri() {
        // Construct a URI that contains double quotes (passes through normalise unchanged).
        let input = r#"rtsp://host/path"with"quotes"#;
        let desc = build_playbin_description(input, None, None, None, false).unwrap();
        // Each quote must be escaped as \" — no bare unescaped " should remain
        // outside the surrounding uri="..." wrapper quotes.
        assert!(desc.contains(r#"\""#), "quotes must be backslash-escaped: {desc}");
        // The original unescaped sequence should not appear literally.
        assert!(!desc.contains(r#"path"with"#), "raw unescaped quote sequence present: {desc}");
    }

    #[test]
    fn rejects_all_forbidden_sink_chars() {
        let bad = ['!', '\'', '`', ' ', '\t', '\n'];
        for ch in bad {
            let name = format!("sink{ch}name");
            assert!(
                build_playbin_description("/tmp/a.mp4", None, Some(&name), None, false).is_err(),
                "char {ch:?} should be rejected",
            );
        }
    }

    #[test]
    fn rejects_empty_sink_name() {
        assert!(build_playbin_description("/tmp/a.mp4", None, Some(""), None, false).is_err());
        assert!(build_playbin_description("/tmp/a.mp4", None, None, Some(""), false).is_err());
    }

    #[tokio::test]
    #[ignore = "requires gstreamer plugins on host; run locally with --ignored"]
    async fn discover_resolves_local_audio_file() {
        let fixture = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/silence.wav");
        if !fixture.exists() {
            eprintln!("skip: fixture {} missing", fixture.display());
            return;
        }
        let info = discover(
            fixture.to_str().unwrap(),
            None,
            Some(std::time::Duration::from_secs(2)),
        )
        .await
        .expect("discover");
        assert!(info.uri.starts_with("file://"));
        assert!(info.audio_streams.len() >= 1);
    }
}
