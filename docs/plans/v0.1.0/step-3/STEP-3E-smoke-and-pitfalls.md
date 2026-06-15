# STEP-3E — End-to-end smoke test & pitfalls

> Verification sub-step (no code edit). Confirms the assembled SRT pipeline
> actually pushes video to a remote listener, and catalogues the SRT-specific
> traps.

---

## End-to-end smoke test (destination)

Pre-req: `adb forward tcp:8080 tcp:8080`, migration runtime running.

### 1. On a second machine, start an SRT listener

```bash
gst-launch-1.0 -v \
  srtsrc uri="srt://0.0.0.0:1234?mode=listener" latency=200 \
  ! tsdemux name=d \
  ! queue ! h264parse ! avdec_h264 ! videoconvert ! autovideosink \
  d. ! queue ! aacparse ! avdec_aac ! audioconvert ! autoaudiosink
```

### 2. Build the graph on the phone

```bash
LISTENER=10.0.0.42

curl -s -X POST http://127.0.0.1:8080/command \
  -d '{"createvideogenerator":{"id":"gen-1"}}'

curl -s -X POST http://127.0.0.1:8080/command \
  -d "{\"createdestination\":{\"id\":\"srt-out\",\
      \"family\":{\"Srt\":{\"uri\":\"srt://${LISTENER}:1234\",\"latency\":200}},\
      \"audio\":false,\"video\":true}}"

curl -s -X POST http://127.0.0.1:8080/command \
  -d '{"connect":{"link_id":"L1","src_id":"gen-1","sink_id":"srt-out",\
      "audio":false,"video":true}}'

curl -s -X POST http://127.0.0.1:8080/command \
  -d '{"start":{"id":"srt-out"}}'

curl -s -X POST http://127.0.0.1:8080/command \
  -d '{"start":{"id":"gen-1"}}'
```

**Expected:** the `autovideosink` window opens on the listener machine within
~1 s, showing the ball-pattern test source.

### 3. Encryption smoke (optional)

Add `passphrase` + `pbkeylen` to both ends and confirm the stream still flows;
a passphrase mismatch should drop the connection (and surface as `last_error`
on the destination's `getinfo`).

```bash
# Listener:
gst-launch-1.0 -v \
  srtsrc uri="srt://0.0.0.0:1234?mode=listener&passphrase=supersecretphrase&pbkeylen=16" \
  ! tsdemux ! queue ! h264parse ! avdec_h264 ! videoconvert ! autovideosink
```

---

## Pitfalls

| # | Symptom | Root cause | Fix |
|---|---|---|---|
| P1 | `srtsink` blocks PLAYING for 3 s then `Stopped` | caller mode + `wait-for-connection=true` | 3A sets `wait-for-connection=false`; for inbound topology append `?mode=listener` to the URI |
| P2 | Receiver logs continuity-counter errors | missing `alignment=7` on `mpegtsmux` | 3A sets it |
| P3 | `GLib-GObject-WARNING: cannot set property 'latency'` | passing `i64`, not `i32` | field is `i32` ms (STEP-1B) |
| P4 | `srtsrc` warning: key-material not received | `passphrase` set but `pbkeylen` is `None` (or vice-versa) | 3A's `if let (Some, Some)` guard sets both or neither |
| P5 | Connection refused / mismatched encryption | passphrase differs between ends, or length not 10–79 | use identical passphrase; validate length in UI (STEP-7) |
| P6 | IPv6 host fails to parse | missing brackets | use `srt://[fe80::1]:1234` |

---

## Done — STEP-3 complete

| Sub-step | Status |
|---|---|
| 3A mux + sink setup | ✅ |
| 3B video chain | ✅ |
| 3C audio chain | ✅ |
| 3D link + assemble | ✅ |
| 3E smoke + pitfalls | ✅ |

The squash commit (STEP-1 + STEP-2 + STEP-3) now compiles and pushes SRT
end-to-end.

→ Next top-level step: [../step-4/INDEX.md](../step-4/INDEX.md)
