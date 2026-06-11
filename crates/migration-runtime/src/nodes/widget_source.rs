use crate::protocol::{NodeInfo, SourceInfo, State, Widget, WidgetType};
use ab_glyph::{point, Font, FontRef, ScaleFont};
use chrono::{DateTime, Local, Utc};
use gst::prelude::*;
use gst_app::{AppSink, AppSrc};
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const DEFAULT_WIDGET_WIDTH: u32 = 512;
const DEFAULT_WIDGET_HEIGHT: u32 = 128;
static FONT_BYTES: &[u8] =
    include_bytes!("../../../../src/main/assets/fontconfig/fonts/truetype/Ubuntu-R.ttf");

#[derive(Debug, Clone)]
pub struct LiveWidgetSourcePipeline {
    pub pipeline: gst::Pipeline,
    pub appsrc: AppSrc,
    pub appsink: AppSink,
    pub ticker_active: Option<Arc<AtomicBool>>,
}

#[derive(Debug, Clone)]
pub struct WidgetSourceNode {
    pub id: String,
    pub widget: Widget,
    pub video_consumer_slot_ids: BTreeSet<String>,
    pub cue_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub state: State,
    pub width: u32,
    pub height: u32,
    pub live_pipeline: Option<LiveWidgetSourcePipeline>,
    pub last_error: Option<String>,
}

impl WidgetSourceNode {
    fn gst_initialized() -> bool {
        unsafe { gst::ffi::gst_is_initialized() != 0 }
    }

    pub fn new(id: String, widget: Widget) -> Self {
        Self {
            id,
            widget,
            video_consumer_slot_ids: BTreeSet::new(),
            cue_time: None,
            end_time: None,
            state: State::Started,
            width: DEFAULT_WIDGET_WIDTH,
            height: DEFAULT_WIDGET_HEIGHT,
            live_pipeline: None,
            last_error: None,
        }
    }

    fn make_element(element: &str, name: &str) -> Result<gst::Element, String> {
        gst::ElementFactory::make(element)
            .name(name)
            .build()
            .map_err(|err| format!("Failed to create element `{element}`: {}", &*err.message))
    }

    fn make_appsrc(id: &str, width: u32, height: u32) -> Result<AppSrc, String> {
        let element = Self::make_element("appsrc", &format!("widget-appsrc-{id}"))?;
        let appsrc = element
            .downcast::<AppSrc>()
            .map_err(|_| format!("Failed to downcast widget appsrc for `{id}`"))?;
        appsrc.set_property("is-live", true);
        appsrc.set_property("do-timestamp", true);
        appsrc.set_property_from_str("format", "time");
        appsrc.set_caps(Some(
            &gst::Caps::builder("video/x-raw")
                .field("format", "RGBA")
                .field("width", width as i32)
                .field("height", height as i32)
                .field("framerate", gst::Fraction::new(30, 1))
                .build(),
        ));
        Ok(appsrc)
    }

    fn render_image_widget(
        asset_path: &str,
        w: u32,
        h: u32,
        scale_mode: &str,
    ) -> Result<Vec<u8>, String> {
        let img = image::open(asset_path).map_err(|e| format!("open image: {e}"))?;
        let resized = match scale_mode {
            "stretch" => img.resize_exact(w, h, image::imageops::FilterType::Triangle),
            "fill" => img.resize_to_fill(w, h, image::imageops::FilterType::Triangle),
            _ => img.resize(w, h, image::imageops::FilterType::Triangle),
        };
        let mut canvas = image::RgbaImage::from_pixel(w, h, image::Rgba([0, 0, 0, 0]));
        let resized = resized.to_rgba8();
        let (rw, rh) = resized.dimensions();
        image::imageops::overlay(
            &mut canvas,
            &resized,
            ((w.saturating_sub(rw)) / 2) as i64,
            ((h.saturating_sub(rh)) / 2) as i64,
        );
        Ok(canvas.into_raw())
    }

    fn parse_rgba_color(input: Option<&str>) -> [u8; 4] {
        let Some(input) = input.map(str::trim) else {
            return [255, 255, 255, 255];
        };
        let hex = input.strip_prefix('#').unwrap_or(input);
        match hex.len() {
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok();
                let g = u8::from_str_radix(&hex[2..4], 16).ok();
                let b = u8::from_str_radix(&hex[4..6], 16).ok();
                match (r, g, b) {
                    (Some(r), Some(g), Some(b)) => [r, g, b, 255],
                    _ => [255, 255, 255, 255],
                }
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok();
                let g = u8::from_str_radix(&hex[2..4], 16).ok();
                let b = u8::from_str_radix(&hex[4..6], 16).ok();
                let a = u8::from_str_radix(&hex[6..8], 16).ok();
                match (r, g, b, a) {
                    (Some(r), Some(g), Some(b), Some(a)) => [r, g, b, a],
                    _ => [255, 255, 255, 255],
                }
            }
            _ => [255, 255, 255, 255],
        }
    }

    fn render_text_widget(
        text: &str,
        w: u32,
        h: u32,
        px: f32,
        rgba_color: [u8; 4],
    ) -> Result<Vec<u8>, String> {
        let font = FontRef::try_from_slice(FONT_BYTES).map_err(|_| "load font".to_string())?;
        let scaled = font.as_scaled(px);
        let mut canvas = vec![0u8; (w * h * 4) as usize];
        let mut caret = point(4.0, px.max(1.0));

        for ch in text.chars() {
            let glyph = scaled.scaled_glyph(ch);
            let glyph_id = glyph.id;
            if let Some(outline) = font.outline_glyph(glyph) {
                let bounds = outline.px_bounds();
                outline.draw(|gx, gy, cov| {
                    let x = bounds.min.x as i32 + gx as i32 + caret.x as i32;
                    let y = bounds.min.y as i32 + gy as i32 + caret.y as i32 - px as i32;
                    if x >= 0 && y >= 0 && (x as u32) < w && (y as u32) < h {
                        let i = ((y as u32 * w + x as u32) * 4) as usize;
                        let a = (cov * rgba_color[3] as f32) as u8;
                        canvas[i] = rgba_color[0];
                        canvas[i + 1] = rgba_color[1];
                        canvas[i + 2] = rgba_color[2];
                        canvas[i + 3] = canvas[i + 3].max(a);
                    }
                });
            }
            caret.x += scaled.h_advance(glyph_id);
        }
        Ok(canvas)
    }

    fn render_current_rgba(&self) -> Result<Vec<u8>, String> {
        match &self.widget.widget_type {
            WidgetType::Image {
                asset_id,
                scale_mode,
            } => Self::render_image_widget(
                asset_id,
                self.width,
                self.height,
                scale_mode.as_deref().unwrap_or("fit"),
            ),
            WidgetType::Text {
                format,
                font_size,
                color,
            } => Self::render_text_widget(
                format,
                self.width,
                self.height,
                font_size.unwrap_or(32) as f32,
                Self::parse_rgba_color(color.as_deref()),
            ),
            WidgetType::Clock {
                format,
                font_size,
                color,
            } => Self::render_text_widget(
                &Local::now().format(format).to_string(),
                self.width,
                self.height,
                font_size.unwrap_or(32) as f32,
                Self::parse_rgba_color(color.as_deref()),
            ),
            WidgetType::Crop { .. } => Ok(vec![0u8; (self.width * self.height * 4) as usize]),
        }
    }

    fn push_rgba(appsrc: &AppSrc, rgba: &[u8]) -> Result<(), String> {
        let mut buffer =
            gst::Buffer::with_size(rgba.len()).map_err(|_| "alloc buffer".to_string())?;
        {
            let buf = buffer.get_mut().ok_or_else(|| "buffer mut".to_string())?;
            let mut map = buf.map_writable().map_err(|_| "map buffer".to_string())?;
            map.copy_from_slice(rgba);
        }
        appsrc
            .push_buffer(buffer)
            .map(|_| ())
            .map_err(|e| format!("push: {e:?}"))
    }

    fn build_live_pipeline(&self) -> Result<LiveWidgetSourcePipeline, String> {
        let pipeline = gst::Pipeline::with_name(&format!("migration-widget-{}", self.id));
        let appsrc = Self::make_appsrc(&self.id, self.width, self.height)?;
        let queue = Self::make_element("queue", &format!("widget-queue-{}", self.id))?;
        let appsink = Self::make_element("appsink", &format!("widget-appsink-{}", self.id))?
            .downcast::<AppSink>()
            .map_err(|_| format!("Failed to downcast widget appsink for `{}`", self.id))?;

        pipeline
            .add(appsrc.upcast_ref::<gst::Element>())
            .map_err(|err| format!("Failed to add widget appsrc to pipeline: {err:?}"))?;
        pipeline
            .add(&queue)
            .map_err(|err| format!("Failed to add widget queue to pipeline: {err:?}"))?;
        pipeline
            .add(appsink.upcast_ref::<gst::Element>())
            .map_err(|err| format!("Failed to add widget appsink to pipeline: {err:?}"))?;

        gst::Element::link_many(
            [
                appsrc.upcast_ref::<gst::Element>(),
                &queue,
                appsink.upcast_ref::<gst::Element>(),
            ]
            .as_slice(),
        )
        .map_err(|err| format!("Failed to link widget pipeline: {err:?}"))?;

        let rgba = self.render_current_rgba()?;
        Self::push_rgba(&appsrc, &rgba)?;

        let ticker_active = if matches!(self.widget.widget_type, WidgetType::Clock { .. }) {
            let active = Arc::new(AtomicBool::new(true));
            let active_for_thread = Arc::clone(&active);
            let appsrc_for_thread = appsrc.clone();
            let width = self.width;
            let height = self.height;
            let widget = self.widget.clone();
            std::thread::spawn(move || {
                while active_for_thread.load(Ordering::SeqCst) {
                    let WidgetType::Clock {
                        format,
                        font_size,
                        color,
                    } = &widget.widget_type
                    else {
                        break;
                    };
                    let text = Local::now().format(format).to_string();
                    if let Ok(rgba) = WidgetSourceNode::render_text_widget(
                        &text,
                        width,
                        height,
                        font_size.unwrap_or(32) as f32,
                        WidgetSourceNode::parse_rgba_color(color.as_deref()),
                    ) {
                        if WidgetSourceNode::push_rgba(&appsrc_for_thread, &rgba).is_err() {
                            break;
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
            });
            Some(active)
        } else {
            None
        };

        Ok(LiveWidgetSourcePipeline {
            pipeline,
            appsrc,
            appsink,
            ticker_active,
        })
    }

    fn teardown_live_pipeline(&mut self) {
        if let Some(live) = self.live_pipeline.take() {
            if let Some(active) = live.ticker_active {
                active.store(false, Ordering::SeqCst);
            }
            let _ = live.pipeline.set_state(gst::State::Null);
        }
    }

    fn ensure_live_pipeline(&mut self) -> Result<(), String> {
        if self.live_pipeline.is_none() {
            self.live_pipeline = Some(self.build_live_pipeline()?);
        }
        Ok(())
    }

    pub fn refresh(&mut self) -> Result<(), String> {
        if !Self::gst_initialized() {
            return Ok(());
        }
        self.ensure_live_pipeline()?;
        if let Some(live) = self.live_pipeline.as_ref() {
            live.pipeline
                .set_state(gst::State::Playing)
                .map_err(|err| {
                    format!("Failed to set widget pipeline state to Playing: {err:?}")
                })?;
        }
        Ok(())
    }

    pub fn stop(&mut self) {
        self.teardown_live_pipeline();
        self.state = State::Stopped;
    }

    pub fn mark_error(&mut self, message: String) {
        self.last_error = Some(message);
        self.state = State::Stopped;
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
        self.live_pipeline.as_ref().map(|live| live.appsink.clone())
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

    pub fn as_compatible_source_info(&self) -> NodeInfo {
        NodeInfo::Source(SourceInfo {
            uri: format!("widget://{}", self.widget.id),
            video_consumer_slot_ids: Some(self.video_consumer_slot_ids.iter().cloned().collect()),
            audio_consumer_slot_ids: None,
            cue_time: self.cue_time,
            end_time: self.end_time,
            state: self.state,
        })
    }
}
