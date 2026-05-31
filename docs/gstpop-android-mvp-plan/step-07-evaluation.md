# Step 7 Evaluation — JNI Bridge Decision Gate

Date: 2026-05-31

## Evaluation Outcome: SKIPPED

Following the explicit instruction in `docs/gstpop-android-mvp-plan/step-07-jni-bridge.md`:
> If the existing `android-sender` crate already exposes the calls Kotlin needs (start/stop/status) via its own JNI module, skip this step. Two JNI boundaries for the same runtime is worse than one.

We evaluated the `android-sender` crate's JNI surface and confirmed:
1. `android-sender`'s `src/lib.rs` and `src/jni_bridge/gstpop_bridge.rs` already fully and cleanly expose GstPopServiceBridge JNI calls to Kotlin:
   - `Java_org_fcast_android_sender_GstPopServiceBridge_nativeStartGstPopServiceHost`
   - `Java_org_fcast_android_sender_GstPopServiceBridge_nativeStopGstPopServiceHost`
   - `Java_org_fcast_android_sender_GstPopServiceBridge_nativeGetGstPopServiceStatus`
2. These methods directly and securely wrap the `gstpop_runtime::start_embedded()`, `stop_embedded()`, and `embedded_status()` functions, handling JSON serialization and tokio runtime coordination perfectly on separate threads.
3. Therefore, adding a duplicate JNI boundary inside the `gstpop-runtime` crate is completely unnecessary and would introduce redundant complexity.

We have skipped implementation of Step 7 to maintain codebase integrity and cleanliness.
