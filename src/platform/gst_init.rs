//! GStreamer initialisation helpers.

#[cfg(target_os = "android")]
pub(crate) fn ensure_gstreamer_initialized() -> std::result::Result<(), String> {
    use gst::prelude::GstObjectExt;
    use std::sync::OnceLock;
    use tracing::info;

    static GST_INIT: OnceLock<std::result::Result<(), String>> = OnceLock::new();
    GST_INIT
        .get_or_init(|| {
            gst::init().map_err(|err| format!("Failed to initialize GStreamer: {err}"))?;

            let amc_encoders = gst::ElementFactory::factories_with_type(
                gst::ElementFactoryType::ENCODER | gst::ElementFactoryType::MEDIA_VIDEO,
                gst::Rank::NONE,
            )
            .into_iter()
            .filter_map(|factory| {
                let name = factory.name();
                name.starts_with("amc").then(|| name.to_string())
            })
            .collect::<Vec<_>>();
            info!(
                amc_encoder_count = amc_encoders.len(),
                amc_encoders = %amc_encoders.join(", "),
                "Android MediaCodec encoder factories registered after GStreamer init"
            );

            Ok(())
        })
        .clone()
}

#[cfg(not(target_os = "android"))]
pub(crate) fn ensure_gstreamer_initialized() -> std::result::Result<(), String> {
    gst::init().map_err(|err| format!("Failed to initialize GStreamer: {err}"))
}
