//! In-process gst-pop daemon host + JSON-RPC client.
//!
//! Extracted from `android-sender`. See
//! `docs/gstpop-runtime-crate-extraction/` for the extraction plan and
//! `docs/gstpop-service-architecture.md` for the runtime architecture.

pub mod client;
pub mod embedded;
pub mod protocol;

#[cfg(test)]
mod protocol_tests;

pub use client::GstPopClient;
pub use embedded::{
    embedded_status, is_localhost, start_embedded, start_embedded_with_config,
    stop_embedded, url_port, EmbeddedConfig, EmbeddedState, EmbeddedStatus,
};
pub use protocol::{classify, ClassifiedFrame, Event, Request, Response};

#[cfg(feature = "typed-client")]
pub mod typed_client;

#[cfg(feature = "typed-client")]
pub use typed_client::{
    PipelineSummary, PositionInfo, TypedGstPopClient,
};

#[cfg(feature = "media-tools")]
pub mod media;

#[cfg(feature = "media-tools")]
pub use media::{
    build_playbin_description, discover, normalise_media_input, AudioStreamInfo, ContainerInfo,
    DiscoverResult, SubtitleStreamInfo, TagsInfo, VideoStreamInfo,
};
