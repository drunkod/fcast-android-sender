use super::protocol::{classify, ClassifiedFrame, Event};

#[test]
fn classifies_state_changed_event() {
    let text = r#"{"event":"state_changed","data":{"pipeline_id":"0","old_state":"paused","new_state":"playing"}}"#;
    match classify(text) {
        ClassifiedFrame::Event(Event::StateChanged {
            pipeline_id,
            new_state,
            ..
        }) => {
            assert_eq!(pipeline_id, "0");
            assert_eq!(new_state, "playing");
        }
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn classifies_response_with_string_id() {
    let text = r#"{"id":"abc","result":{"pipeline_id":"0"}}"#;
    match classify(text) {
        ClassifiedFrame::Response(response) => {
            assert_eq!(response.id_as_str(), Some("abc".to_owned()));
            assert!(response.error.is_none());
        }
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn unknown_event_falls_back_to_other() {
    let text = r#"{"event":"future_unknown","data":{}}"#;
    match classify(text) {
        ClassifiedFrame::Event(Event::Other) => {}
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn deserializes_a_full_pipeline_added_event() {
    let text = r#"{"event":"pipeline_added","data":{"pipeline_id":"7","description":"videotestsrc ! autovideosink"}}"#;
    match classify(text) {
        ClassifiedFrame::Event(Event::PipelineAdded {
            pipeline_id,
            description,
        }) => {
            assert_eq!(pipeline_id, "7");
            assert_eq!(description, "videotestsrc ! autovideosink");
        }
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn response_with_error_does_not_yield_result() {
    let text = r#"{"id":"abc","error":{"code":-32000,"message":"Pipeline not found"}}"#;
    match classify(text) {
        ClassifiedFrame::Response(response) => {
            assert!(response.result.is_none());
            assert_eq!(response.error.as_ref().unwrap().code, -32000);
        }
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn integer_id_responses_round_trip_to_string() {
    let text = r#"{"id":42,"result":{}}"#;
    match classify(text) {
        ClassifiedFrame::Response(response) => {
            assert_eq!(response.id_as_str(), Some("42".to_owned()));
        }
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn null_id_response_yields_none() {
    let text = r#"{"id":null,"result":{}}"#;
    match classify(text) {
        ClassifiedFrame::Response(response) => {
            assert_eq!(response.id_as_str(), None);
        }
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn non_json_garbage_is_classified_garbage() {
    assert!(matches!(classify("not json at all"), ClassifiedFrame::Garbage));
    assert!(matches!(classify(""), ClassifiedFrame::Garbage));
    assert!(matches!(classify("{invalid"), ClassifiedFrame::Garbage));
}

#[test]
fn json_without_event_or_id_is_garbage() {
    assert!(matches!(classify(r#"{"foo":"bar"}"#), ClassifiedFrame::Garbage));
    assert!(matches!(classify(r#"{}"#), ClassifiedFrame::Garbage));
    assert!(matches!(classify(r#"[]"#), ClassifiedFrame::Garbage));
}

#[test]
fn classifies_eos_event() {
    let text = r#"{"event":"eos","data":{"pipeline_id":"pipe1"}}"#;
    match classify(text) {
        ClassifiedFrame::Event(Event::Eos { pipeline_id }) => {
            assert_eq!(pipeline_id, "pipe1");
        }
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn classifies_error_event() {
    let text = r#"{"event":"error","data":{"pipeline_id":"pipe2","message":"something broke"}}"#;
    match classify(text) {
        ClassifiedFrame::Event(Event::Error { pipeline_id, message }) => {
            assert_eq!(pipeline_id, "pipe2");
            assert_eq!(message, "something broke");
        }
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn classifies_pipeline_updated_event() {
    let text = r#"{"event":"pipeline_updated","data":{"pipeline_id":"p1","description":"fakesrc ! fakesink"}}"#;
    match classify(text) {
        ClassifiedFrame::Event(Event::PipelineUpdated { pipeline_id, description }) => {
            assert_eq!(pipeline_id, "p1");
            assert_eq!(description, "fakesrc ! fakesink");
        }
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn classifies_pipeline_removed_event() {
    let text = r#"{"event":"pipeline_removed","data":{"pipeline_id":"gone"}}"#;
    match classify(text) {
        ClassifiedFrame::Event(Event::PipelineRemoved { pipeline_id }) => {
            assert_eq!(pipeline_id, "gone");
        }
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn classifies_unsupported_event() {
    let text = r#"{"event":"unsupported","data":{"pipeline_id":"p1","message":"codec not found"}}"#;
    match classify(text) {
        ClassifiedFrame::Event(Event::Unsupported { pipeline_id, message }) => {
            assert_eq!(pipeline_id, "p1");
            assert_eq!(message, "codec not found");
        }
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn request_ids_are_unique() {
    use super::protocol::Request;
    use serde_json::json;
    let ids: Vec<_> = (0..50)
        .map(|_| Request::new("ping", json!({})).id)
        .collect();
    let unique: std::collections::HashSet<_> = ids.iter().collect();
    assert_eq!(unique.len(), 50, "Request IDs must be unique");
}

#[test]
fn pipeline_state_ext_known_serializes_to_string() {
    use super::protocol::{PipelineState, PipelineStateExt};
    let cases = [
        (PipelineState::Null, "null"),
        (PipelineState::Ready, "ready"),
        (PipelineState::Paused, "paused"),
        (PipelineState::Playing, "playing"),
    ];
    for (state, expected) in cases {
        let ext = PipelineStateExt::Known(state);
        let serialized = serde_json::to_string(&ext).unwrap();
        assert_eq!(serialized, format!("\"{expected}\""), "state {expected} must round-trip");
        let back: PipelineStateExt = serde_json::from_str(&serialized).unwrap();
        assert_eq!(back, PipelineStateExt::Known(state));
    }
}

#[test]
fn all_pipeline_event_kinds_deserialize() {
    use super::protocol::PipelineEventKind;
    use serde_json::json;
    let cases = [
        ("state_changed", PipelineEventKind::StateChanged),
        ("position", PipelineEventKind::Position),
        ("eos", PipelineEventKind::Eos),
        ("error", PipelineEventKind::Error),
        ("warning", PipelineEventKind::Warning),
        ("info", PipelineEventKind::Info),
        ("stream_start", PipelineEventKind::StreamStart),
        ("async_done", PipelineEventKind::AsyncDone),
    ];
    for (raw, expected) in cases {
        let parsed: PipelineEventKind = serde_json::from_value(json!(raw)).unwrap();
        assert_eq!(parsed, expected, "failed for {raw}");
    }
}
