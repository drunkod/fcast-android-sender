use crate::{
    frame_pair::FramePair,
    protocol::{NodeInfo, SourceInfo, State},
};
use chrono::{DateTime, Duration, Utc};
use gst::prelude::*;
use gst_app::{AppSink, AppSrc};
use std::collections::BTreeSet;
use std::sync::Arc;

const PREROLL_LEAD_TIME_SECONDS: i64 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraPipelineStage {
    Idle,
    Prerolling,
    Playing,
}

#[derive(Debug, Clone)]
pub struct LiveCameraPipeline {
    pub pipeline: gst::Pipeline,
    pub video_appsink: AppSink,
}

#[derive(Debug, Clone)]
pub struct CameraSourceNode {
    pub frame_pair: Arc<FramePair>,
    pub id: String,
    pub camera_idx: u32,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub mirror: bool,
    pub stabilization: bool,
    pub zoom: f32,
    /// Clockwise rotation in degrees (0/90/180/270) applied via GStreamer videoflip.
    /// Computed by Kotlin from SENSOR_ORIENTATION + device physical rotation and
    /// reported back via nativeCameraCaptureStarted. Mirrors Moblin's setVideoOrientation().
    pub rotation_deg: u32,
    pub video_consumer_slot_ids: BTreeSet<String>,
    pub cue_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub state: State,
    pub stage: CameraPipelineStage,
    pub live_pipeline: Option<LiveCameraPipeline>,
    pub last_error: Option<String>,
}

/// Maps (rotation_deg, mirror) → a GStreamer `videoflip` method string.
///
/// Encodes both transforms into a single element to avoid chaining two
/// `videoflip` instances. Equivalent to Moblin's combined
/// `isLandscapeStreamAndPortraitUi` + `AVCaptureVideoOrientation` logic.
///
/// Valid `rotation_deg` values: 0, 90, 180, 270. Others are treated as 0.
pub fn videoflip_method(rotation_deg: u32, mirror: bool) -> &'static str {
    match (rotation_deg, mirror) {
        (0, false) => "none",
        (0, true) => "horizontal-flip",
        (90, false) => "clockwise",
        (90, true) => "upper-right-diagonal",
        (180, false) => "rotate-180",
        (180, true) => "vertical-flip",
        (270, false) => "counterclockwise",
        (270, true) => "upper-left-diagonal",
        _ => "none",
    }
}

impl CameraSourceNode {
    fn gst_initialized() -> bool {
        unsafe { gst::ffi::gst_is_initialized() != 0 }
    }

    pub fn new(
        frame_pair: Arc<FramePair>,
        id: String,
        camera_idx: u32,
        width: u32,
        height: u32,
        fps: u32,
        mirror: bool,
        stabilization: bool,
        zoom: f32,
        rotation_deg: u32,
    ) -> Self {
        Self {
            frame_pair,
            id,
            camera_idx,
            width,
            height,
            fps,
            mirror,
            stabilization,
            zoom,
            rotation_deg,
            video_consumer_slot_ids: BTreeSet::new(),
            cue_time: None,
            end_time: None,
            state: State::Initial,
            stage: CameraPipelineStage::Idle,
            live_pipeline: None,
            last_error: None,
        }
    }

    pub fn as_info(&self) -> NodeInfo {
        NodeInfo::Source(SourceInfo {
            uri: format!("camera://{}x{}@{}fps", self.width, self.height, self.fps),
            video_consumer_slot_ids: Some(self.video_consumer_slot_ids.iter().cloned().collect()),
            audio_consumer_slot_ids: None,
            cue_time: self.cue_time,
            end_time: self.end_time,
            state: self.state,
        })
    }

    pub fn schedule(
        &mut self,
        cue_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    ) -> Result<(), String> {
        self.cue_time = cue_time;
        self.end_time = end_time;
        Ok(())
    }

    pub fn add_consumer_link(&mut self, link_id: &str, _audio: bool, video: bool) {
        if video {
            self.video_consumer_slot_ids.insert(link_id.to_string());
        }
    }

    pub fn remove_consumer_link(&mut self, link_id: &str) {
        self.video_consumer_slot_ids.remove(link_id);
    }

    pub fn live_video_appsink(&self) -> Option<AppSink> {
        self.live_pipeline
            .as_ref()
            .map(|live| live.video_appsink.clone())
    }

    pub fn stop(&mut self) {
        self.state = State::Stopped;
        self.stage = CameraPipelineStage::Idle;
        self.teardown_live_pipeline();
    }

    pub fn mark_error(&mut self, message: String) {
        self.last_error = Some(message);
        self.stop();
    }

    pub fn refresh(&mut self) -> Result<(), String> {
        self.advance_schedule(Utc::now());
        self.sync_live_pipeline()
    }

    fn teardown_live_pipeline(&mut self) {
        if let Some(live) = self.live_pipeline.take() {
            let _ = live.pipeline.set_state(gst::State::Null);
        }
    }

    fn build_live_pipeline(&self) -> Result<LiveCameraPipeline, String> {
        let pipeline = gst::Pipeline::with_name(&format!("migration-camera-{}", self.id));
        let appsrc = AppSrc::builder()
            .name(format!("camera-appsrc-{}", self.id))
            .format(gst::Format::Time)
            .is_live(true)
            .do_timestamp(true)
            .stream_type(gst_app::AppStreamType::Stream)
            .caps(
                &gst::Caps::builder("video/x-raw")
                    .field("format", "I420")
                    .field("width", self.width as i32)
                    .field("height", self.height as i32)
                    .field("framerate", gst::Fraction::new(self.fps as i32, 1))
                    .build(),
            )
            .build();

        let videoconvert = gst::ElementFactory::make("videoconvert")
            .name(format!("camera-videoconvert-{}", self.id))
            .build()
            .map_err(|err| format!("Failed to create videoconvert: {}", err.message))?;

        // Always-present videocrop (passthrough at 0 crop). Crop widgets in the
        // active scene set its top/bottom/left/right via `set_crop`. Placed
        // before videoflip so crop percentages map to the sensor-oriented
        // self.width/self.height coordinate space.
        let videocrop = gst::ElementFactory::make("videocrop")
            .name(format!("camera-videocrop-{}", self.id))
            .build()
            .map_err(|err| format!("Failed to create videocrop: {}", err.message))?;

        // Single videoflip encodes both rotation and mirror into one method string.
        // This avoids double-transformation from chaining two elements.
        let flip_method = videoflip_method(self.rotation_deg, self.mirror);
        let flip_elem = if flip_method != "none" {
            Some(
                gst::ElementFactory::make("videoflip")
                    .name(format!("camera-videoflip-{}", self.id))
                    .property_from_str("method", flip_method)
                    .build()
                    .map_err(|err| format!("Failed to create videoflip: {}", err.message))?,
            )
        } else {
            None
        };

        let appsink = gst::ElementFactory::make("appsink")
            .name(format!("camera-video-appsink-{}", self.id))
            .property("sync", false)
            .build()
            .map_err(|err| format!("Failed to create appsink: {}", err.message))?
            .downcast::<AppSink>()
            .map_err(|_| "Failed to downcast camera appsink".to_string())?;

        // H.264 macroblock alignment. amcvidenc codes to 16x16 macroblocks, so a
        // non-16-aligned height (1080 -> coded 1088) leaves padding rows the
        // encoder fills as a green edge at the receiver. Verified via frame dumps:
        // the camera frames themselves are clean — the green is purely this encoder
        // padding. Crop the unavoidable remainder from the FINAL (post-flip) frame
        // so the encoder always receives 16-aligned dimensions. Post-flip width/
        // height swap for 90/270 rotation, so compute against the output geometry.
        let (out_w, out_h) = if matches!(self.rotation_deg, 90 | 270) {
            (self.height, self.width)
        } else {
            (self.width, self.height)
        };
        let align_crop = gst::ElementFactory::make("videocrop")
            .name(format!("camera-aligncrop-{}", self.id))
            .build()
            .map_err(|err| format!("Failed to create alignment videocrop: {}", err.message))?;
        let align_right = (out_w % 16) as i32;
        let align_bottom = (out_h % 16) as i32;
        if align_right != 0 {
            align_crop.set_property("right", align_right);
        }
        if align_bottom != 0 {
            align_crop.set_property("bottom", align_bottom);
        }

        let mut elements: Vec<&gst::Element> = vec![appsrc.upcast_ref(), &videoconvert, &videocrop];
        if let Some(ref f) = flip_elem {
            elements.push(f);
        }
        elements.push(&align_crop);
        elements.push(appsink.upcast_ref());

        pipeline
            .add_many(&elements)
            .map_err(|err| format!("Failed to add camera pipeline elements: {err:?}"))?;
        gst::Element::link_many(&elements)
            .map_err(|err| format!("Failed to link camera pipeline elements: {err:?}"))?;

        // ── Debug: log the actual pipeline geometry/crop/flip at build time ──
        tracing::info!(
            id = %self.id,
            width = self.width,
            height = self.height,
            rotation_deg = self.rotation_deg,
            mirror = self.mirror,
            flip = %flip_method,
            crop_top = videocrop.property::<i32>("top"),
            crop_bottom = videocrop.property::<i32>("bottom"),
            crop_left = videocrop.property::<i32>("left"),
            crop_right = videocrop.property::<i32>("right"),
            out_w,
            out_h,
            align_right,
            align_bottom,
            encoded_w = out_w as i32 - align_right,
            encoded_h = out_h as i32 - align_bottom,
            "camera pipeline built"
        );

        // ── Debug: dump the encoder-INPUT frame (after videoconvert/crop/flip) ──
        // Toggle at runtime (no rebuild) by creating the marker file
        // <FCAST_DUMP_DIR>/on. Compare these `pipe_*` dumps with the `cam_*` dumps
        // (raw GL output, before crop/flip) to localise the green line. The probe
        // is cheap when disabled (an atomic + a stat only on throttled frames).
        if let (Ok(dump_dir), Some(sink_pad)) =
            (std::env::var("FCAST_DUMP_DIR"), appsink.static_pad("sink"))
        {
            let id = self.id.clone();
            let count = Arc::new(std::sync::atomic::AtomicU64::new(0));
            sink_pad.add_probe(gst::PadProbeType::BUFFER, move |pad, info| {
                let Some(buffer) = info.buffer() else {
                    return gst::PadProbeReturn::Ok;
                };
                let n = count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if n >= 5 && n % 120 != 0 {
                    return gst::PadProbeReturn::Ok;
                }
                let dir = std::path::Path::new(&dump_dir);
                if !dir.join("on").exists() {
                    return gst::PadProbeReturn::Ok; // marker absent → disabled
                }
                let (w, h) = pad
                    .current_caps()
                    .and_then(|caps| {
                        let s = caps.structure(0)?;
                        Some((s.get::<i32>("width").ok()?, s.get::<i32>("height").ok()?))
                    })
                    .unwrap_or((0, 0));
                if let Ok(map) = buffer.map_readable() {
                    let path = dir.join(format!("pipe_{w}x{h}_{n:05}.i420"));
                    if let Ok(mut f) = std::fs::File::create(&path) {
                        use std::io::Write;
                        let _ = f.write_all(map.as_slice());
                        tracing::info!(id = %id, path = %path.display(), "dumped pipeline frame");
                    }
                }
                gst::PadProbeReturn::Ok
            });
        }

        Self::wire_need_data(&appsrc, self.frame_pair.clone());

        Ok(LiveCameraPipeline {
            pipeline,
            video_appsink: appsink,
        })
    }

    fn ensure_live_pipeline(&mut self) -> Result<(), String> {
        if self.live_pipeline.is_none() {
            self.live_pipeline = Some(self.build_live_pipeline()?);
        }
        Ok(())
    }

    /// Dynamically updates the `videoflip` element's rotation after the pipeline
    /// is already running. Called when Kotlin reports the actual rotation via
    /// `nativeCameraCaptureStarted`. Equivalent to Moblin's setVideoOrientation().
    pub fn update_rotation(&mut self, deg: u32) {
        self.rotation_deg = deg;
        let Some(live) = self.live_pipeline.as_ref() else {
            return;
        };
        let flip_name = format!("camera-videoflip-{}", self.id);
        if let Some(elem) = live.pipeline.by_name(&flip_name) {
            let method = videoflip_method(deg, self.mirror);
            elem.set_property_from_str("method", method);
        }
    }

    /// Applies a Crop widget's margins to the camera `videocrop` element.
    /// `top`/`bottom`/`left`/`right` are percentages (0.0–100.0) of the
    /// sensor frame; converted to pixels against `self.width`/`self.height`.
    /// A no-op (0,0,0,0) resets the crop. Mirrors `update_rotation`'s by-name
    /// lookup so it works on the already-running pipeline.
    pub fn set_crop(&self, top: f64, bottom: f64, left: f64, right: f64) {
        let Some(live) = self.live_pipeline.as_ref() else {
            return;
        };
        let name = format!("camera-videocrop-{}", self.id);
        let Some(vc) = live.pipeline.by_name(&name) else {
            return;
        };
        let w = self.width as f64;
        let h = self.height as f64;
        // Clamp so opposite margins can't exceed the frame.
        let crop_top = (top.clamp(0.0, 100.0) * h / 100.0) as i32;
        let crop_bottom = (bottom.clamp(0.0, 100.0) * h / 100.0) as i32;
        let crop_left = (left.clamp(0.0, 100.0) * w / 100.0) as i32;
        let crop_right = (right.clamp(0.0, 100.0) * w / 100.0) as i32;
        vc.set_property("top", crop_top);
        vc.set_property("bottom", crop_bottom);
        vc.set_property("left", crop_left);
        vc.set_property("right", crop_right);
        tracing::info!(
            id = %self.id,
            crop_top, crop_bottom, crop_left, crop_right,
            "set_crop applied"
        );
    }

    fn poll_bus_messages(&mut self) -> Result<(), String> {
        let Some(live) = self.live_pipeline.as_ref() else {
            return Ok(());
        };
        let Some(bus) = live.pipeline.bus() else {
            return Ok(());
        };

        let mut saw_eos = false;
        let mut last_error = None;
        while let Some(message) = bus.timed_pop_filtered(
            gst::ClockTime::ZERO,
            &[gst::MessageType::Error, gst::MessageType::Eos],
        ) {
            match message.view() {
                gst::MessageView::Eos(..) => saw_eos = true,
                gst::MessageView::Error(err) => {
                    last_error = Some(format!(
                        "CameraSource {} pipeline error from {:?}: {} ({:?})",
                        self.id,
                        err.src().map(|src| src.path_string()),
                        err.error(),
                        err.debug()
                    ));
                }
                _ => {}
            }
        }

        if let Some(err) = last_error {
            self.last_error = Some(err.clone());
            self.stage = CameraPipelineStage::Idle;
            self.state = State::Stopped;
            self.teardown_live_pipeline();
            return Err(err);
        }

        if saw_eos {
            self.stage = CameraPipelineStage::Idle;
            self.state = State::Stopped;
            self.teardown_live_pipeline();
        }

        Ok(())
    }

    fn sync_live_pipeline(&mut self) -> Result<(), String> {
        if !Self::gst_initialized() {
            return Ok(());
        }

        self.poll_bus_messages()?;

        match self.stage {
            CameraPipelineStage::Idle => {
                self.teardown_live_pipeline();
                Ok(())
            }
            CameraPipelineStage::Prerolling | CameraPipelineStage::Playing => {
                self.ensure_live_pipeline()?;

                let target_state = if self.stage == CameraPipelineStage::Prerolling {
                    gst::State::Paused
                } else {
                    gst::State::Playing
                };

                if let Some(live) = self.live_pipeline.as_ref() {
                    live.pipeline.set_state(target_state).map_err(|err| {
                        format!("Failed to set camera pipeline state to {target_state:?}: {err:?}")
                    })?;
                }

                self.poll_bus_messages()
            }
        }
    }

    fn schedule_transition_due(&self, now: DateTime<Utc>) -> Option<State> {
        match self.state {
            State::Initial => match self.cue_time {
                Some(cue) => {
                    let preroll_at = cue - Duration::seconds(PREROLL_LEAD_TIME_SECONDS);
                    if now >= preroll_at {
                        Some(State::Starting)
                    } else {
                        None
                    }
                }
                None => Some(State::Started),
            },
            State::Starting => {
                if self.cue_time.is_none_or(|cue| now >= cue) {
                    Some(State::Started)
                } else {
                    None
                }
            }
            State::Started => {
                if self.end_time.is_some_and(|end| now >= end) {
                    Some(State::Stopping)
                } else {
                    None
                }
            }
            State::Stopping => Some(State::Stopped),
            State::Stopped => None,
        }
    }

    fn apply_state_to_stage(&mut self) {
        self.stage = match self.state {
            State::Initial | State::Stopping | State::Stopped => CameraPipelineStage::Idle,
            State::Starting => CameraPipelineStage::Prerolling,
            State::Started => CameraPipelineStage::Playing,
        };
    }

    fn advance_schedule(&mut self, now: DateTime<Utc>) -> bool {
        let mut changed = false;
        while let Some(next_state) = self.schedule_transition_due(now) {
            if next_state == self.state {
                break;
            }
            self.state = next_state;
            changed = true;
        }

        let old_stage = self.stage;
        self.apply_state_to_stage();
        changed || old_stage != self.stage
    }

    fn wire_need_data(appsrc: &AppSrc, frame_pair: Arc<FramePair>) {
        let mut caps = None::<gst::Caps>;
        appsrc.set_callbacks(
            gst_app::AppSrcCallbacks::builder()
                .need_data(move |appsrc, _| {
                    let frame = {
                        let mut frame = frame_pair.frame.lock();
                        while (*frame).is_none() {
                            frame_pair
                                .cond
                                .wait_for(&mut frame, std::time::Duration::from_millis(100));
                        }
                        (*frame).take()
                    };

                    let Some(frame) = frame else {
                        return;
                    };

                    use gst_video::prelude::*;

                    let now_caps = gst_video::VideoInfo::builder(
                        frame.format(),
                        frame.width(),
                        frame.height(),
                    )
                    .build()
                    .unwrap()
                    .to_caps()
                    .unwrap();

                    match &caps {
                        Some(old_caps) if *old_caps == now_caps => {}
                        _ => {
                            appsrc.set_caps(Some(&now_caps));
                            caps = Some(now_caps);
                        }
                    }

                    let _ = appsrc.push_buffer(frame.into_buffer());
                })
                .build(),
        );
    }
}
