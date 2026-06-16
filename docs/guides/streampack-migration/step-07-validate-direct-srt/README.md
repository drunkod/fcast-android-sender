# Step 07 — ✅ Validate `StreamPackDirectSrt` (Phase 1 gate)

**Master plan:** §5.5 · **Phase:** 1 · **Depends on:** step-01…06 · **Lang:** — (on-device)

## Goal

Prove the StreamPack `Camera2 → Surface → MediaCodec → SRT` path works on-device and
beats the legacy raw path on CPU/thermals/latency — **before** any Phase 2 work begins.
This is the go/no-go gate.

## Validation checklist

```
1. App still launches; Slint NativeActivity renders; GStreamer.init(this) succeeds.
2. Flag OFF → legacy raw path identical to today (regression guard).
3. Flag ON  → StreamPack starts at 1280x720@30; SRT receiver sees A/V.
4. Start/stop 20× with no leak / no crash (watch releaseBlocking + the mutex/state machine).
5. No green edge at 1280x720.
6. Test 1920x1072, then 1920x1080.
7. Compare CPU%, thermals, latency vs legacy path (this is the payoff).
```

## Test receiver

```bash
gst-launch-1.0 srtsrc uri="srt://:9000?mode=listener" ! tsdemux ! h264parse \
  ! avdec_h264 ! videoconvert ! autovideosink
```

## Suggested measurement method

```
- CPU%:      adb shell top -m 10 | grep fcast   (or Perfetto / Android Studio profiler)
- Thermals:  adb shell dumpsys thermalservice   (watch throttling status over 10 min)
- Latency:   glass-to-glass with a millisecond clock on-screen vs receiver
- Leaks:     repeated start/stop ×20, then `adb shell dumpsys meminfo <pkg>` delta
```

Record legacy vs StreamPack side-by-side at 720p30 and 1080p30.

## Exit criteria (all must hold to proceed to Phase 2)

```
✅ Clean A/V at 720p and 1080p over SRT.
✅ 20× start/stop with no crash, no leaked camera/codec, no stuck "starting".
✅ Measurably lower CPU and/or better thermal headroom than the legacy raw path.
✅ Flag OFF still byte-for-byte the legacy behaviour.
```

## If it fails

- **Green edge at 1080:** confirm `alignDown16` is in effect (encoder should be 1072).
  Cross-check with the legacy `FCAST_DUMP_DIR` dump tooling and `scripts/dump-frames.sh`.
- **Stuck "starting":** inspect the step-02 state machine / step-03 sessionId; a stop
  during configuration should leave `state = IDLE` and `capturing = false`.
- **No payoff (CPU not lower):** re-check that the GL readback path is truly bypassed in
  StreamPack mode (no `nativeProcessFrame` calls in logcat) and preview was destroyed.

## Do NOT proceed past this gate

Step-08+ (Phase 2 custom endpoint) must wait until this step is green.
