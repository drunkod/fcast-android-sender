use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

#[derive(Clone, Debug, Serialize)]
pub struct Request {
    pub id: String,
    pub method: String,
    pub params: Value,
}

impl Request {
    pub fn new(method: impl Into<String>, params: Value) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            method: method.into(),
            params,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Response {
    #[serde(default)]
    pub id: Value,
    #[serde(default)]
    pub result: Option<Value>,
    #[serde(default)]
    pub error: Option<RpcError>,
}

impl Response {
    pub fn id_as_str(&self) -> Option<String> {
        match &self.id {
            Value::String(value) => Some(value.clone()),
            Value::Null => None,
            other => Some(other.to_string()),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
}

impl fmt::Display for RpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "gst-pop error ({}): {}", self.code, self.message)
    }
}

impl std::error::Error for RpcError {}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "event", content = "data", rename_all = "snake_case")]
pub enum Event {
    StateChanged {
        pipeline_id: String,
        old_state: String,
        new_state: String,
    },
    Error {
        pipeline_id: String,
        message: String,
    },
    Unsupported {
        pipeline_id: String,
        message: String,
    },
    Eos {
        pipeline_id: String,
    },
    PipelineAdded {
        pipeline_id: String,
        description: String,
    },
    PipelineUpdated {
        pipeline_id: String,
        description: String,
    },
    PipelineRemoved {
        pipeline_id: String,
    },
    #[serde(other)]
    Other,
}

#[derive(Clone, Debug)]
pub enum ClassifiedFrame {
    Response(Response),
    Event(Event),
    Garbage,
}

pub fn classify(text: &str) -> ClassifiedFrame {
    let value: Value = match serde_json::from_str(text) {
        Ok(value) => value,
        Err(_) => return ClassifiedFrame::Garbage,
    };

    if value.get("event").is_some() {
        match serde_json::from_value::<Event>(value) {
            Ok(event) => ClassifiedFrame::Event(event),
            Err(_) => ClassifiedFrame::Event(Event::Other),
        }
    } else if value.get("id").is_some() {
        match serde_json::from_value::<Response>(value) {
            Ok(response) => ClassifiedFrame::Response(response),
            Err(_) => ClassifiedFrame::Garbage,
        }
    } else {
        ClassifiedFrame::Garbage
    }
}

/// Mirror of `gstpop::gst::event::PipelineState`. Unknown server values fall
/// through to `Other` so clients survive server upgrades.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineState {
    Null,
    Ready,
    Paused,
    Playing,
    #[serde(other)]
    Other,
}

impl PipelineState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Ready => "ready",
            Self::Paused => "paused",
            Self::Playing => "playing",
            Self::Other => "other",
        }
    }
}

/// Mirror of the daemon's `PipelineEvent` discriminant. Kept opaque on
/// payload to avoid a second migration step if the server adds fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineEventKind {
    StateChanged,
    Position,
    Eos,
    Error,
    Warning,
    Info,
    StreamStart,
    AsyncDone,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineStateExt {
    Known(PipelineState),
    Unknown(String),
}

impl<'de> Deserialize<'de> for PipelineStateExt {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        let known = match s.as_str() {
            "null" => Some(PipelineState::Null),
            "ready" => Some(PipelineState::Ready),
            "paused" => Some(PipelineState::Paused),
            "playing" => Some(PipelineState::Playing),
            _ => None,
        };
        Ok(match known {
            Some(k) => PipelineStateExt::Known(k),
            None => PipelineStateExt::Unknown(s),
        })
    }
}

impl serde::Serialize for PipelineStateExt {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Known(k) => s.serialize_str(k.as_str()),
            Self::Unknown(raw) => s.serialize_str(raw),
        }
    }
}

#[cfg(test)]
mod typed_state_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn known_states_round_trip() {
        for raw in ["null", "ready", "paused", "playing"] {
            let parsed: PipelineState = serde_json::from_value(json!(raw)).unwrap();
            assert_eq!(parsed.as_str(), raw);
        }
    }

    #[test]
    fn unknown_state_falls_through() {
        let parsed: PipelineState =
            serde_json::from_value(json!("future_state")).unwrap();
        assert!(matches!(parsed, PipelineState::Other));
    }

    #[test]
    fn ext_preserves_unknown() {
        let parsed: PipelineStateExt =
            serde_json::from_value(json!("future_state")).unwrap();
        match parsed {
            PipelineStateExt::Unknown(s) => assert_eq!(s, "future_state"),
            other => panic!("expected Unknown, got {other:?}"),
        }
    }

    #[test]
    fn event_kinds() {
        let parsed: PipelineEventKind =
            serde_json::from_value(json!("state_changed")).unwrap();
        assert_eq!(parsed, PipelineEventKind::StateChanged);

        let unknown: PipelineEventKind =
            serde_json::from_value(json!("new_event")).unwrap();
        assert_eq!(unknown, PipelineEventKind::Other);
    }
}
