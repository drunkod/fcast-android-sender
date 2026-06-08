# Step 5 — `AndroidManifest.xml`: the `:codec_bench` service

← [Step 4](step-4-lib-register-result-downcall.md) · [Index](README.md) · Next → [Step 6](step-6-kotlin-service-aidl.md)

Add inside `<application>`, next to the existing `GstPopService` /
`MigrationRuntimeService` entries. The `android:process=":codec_bench"` attribute
is what creates the **separate process** — the whole point of this plan.

```xml
        <service
            android:name=".bench.CodecBenchmarkService"
            android:process=":codec_bench"
            android:exported="false"
            android:stopWithTask="true" />
```

Notes:
- No `foregroundServiceType` — this is a short-lived **bound** service, not a
  foreground service (unlike `ScreenCaptureService`/`GstPopService`).
- `:codec_bench` (leading colon) = private process for this app.
- `stopWithTask=true` — dies if the app task is removed.

---

← [Step 4](step-4-lib-register-result-downcall.md) · [Index](README.md) · Next → [Step 6](step-6-kotlin-service-aidl.md)
