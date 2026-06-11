# STEP-1E — Tests, smoke & pitfalls

## Unit tests (`protocol.rs`)

```rust
#[test]
fn rist_destination_defaults_when_omitted() {
    let cmd: Command = serde_json::from_str(
        r#"{"createdestination":{"id":"r1","family":{"Rist":{"address":"10.0.0.5"}}}}"#,
    ).unwrap();
    match cmd {
        Command::CreateDestination {
            family: DestinationFamily::Rist { port, sender_buffer_ms, .. }, ..
        } => {
            assert_eq!(port, 5004);
            assert_eq!(sender_buffer_ms, 1000);
        }
        other => panic!("expected Rist, got {other:?}"),
    }
}

#[test]
fn rist_destination_roundtrip() {
    let family = DestinationFamily::Rist {
        address: "10.0.0.5".into(), port: 7000, sender_buffer_ms: 1500,
    };
    let json = serde_json::to_string(&family).unwrap();
    assert_eq!(serde_json::from_str::<DestinationFamily>(&json).unwrap(), family);
}
```

Profile test (`nodes/destination.rs`):

```rust
#[test]
fn rist_profile_lists_ristsink_and_mpegtsmux() {
    let family = DestinationFamily::Rist { address: "10.0.0.5".into(), port: 5004, sender_buffer_ms: 1000 };
    let profile = DestinationPipelineProfile::from_family(&family, true, true);
    assert!(profile.elements.iter().any(|e| e == "ristsink"));
    assert!(profile.elements.iter().any(|e| e == "mpegtsmux"));
}
```

```bash
cargo test -p migration-runtime -- rist
```

## On-device smoke

```bash
# Receiver — RIST wraps TS in RTP, so depay before tsdemux:
gst-launch-1.0 -v ristsrc address=0.0.0.0 port=5004 ! rtpmp2tdepay ! tsdemux \
  ! h264parse ! avdec_h264 ! videoconvert ! autovideosink
# Phone: CreateDestination {"Rist":{"address":"<recv-ip>","port":5004}} → Connect → Start
```

## Pitfalls

| # | Symptom | Fix |
|---|---|---|
| P1 | `ristsink` not found | `rist` not in Android.mk (1D) or absent from prebuilt SDK |
| P2 | Receiver sees nothing | needs `rtpmp2tdepay` before `tsdemux` |
| P3 | continuity errors | keep `alignment=7` |
| P4 | high latency | tune `sender-buffer` (ms) to match the receiver |

## Done — STEP-1 complete

→ Next: [../step-2/INDEX.md](../step-2/INDEX.md)
