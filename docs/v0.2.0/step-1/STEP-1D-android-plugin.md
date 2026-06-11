# STEP-1D — Add `rist` to `Android.mk`

> Independent of 1A–1C. Mandatory for any on-device RIST test.

**File:** `app/jni/Android.mk` — add `rist` to `GSTREAMER_PLUGINS` (after `srt`):

```makefile
    srt \
    rist \
    webrtc \
```

`rist` lives in `gst-plugins-bad` in the same prebuilt SDK that already provides
`srt`. If the NDK link fails with `undefined reference to rist_*`, the prebuilt
ABI lacks libRIST — verify before promising RIST to users.

Confirm on device:

```bash
adb logcat | grep -E 'ristsink|ristsrc'
```

→ Next: [STEP-1E-tests-and-smoke.md](STEP-1E-tests-and-smoke.md)
