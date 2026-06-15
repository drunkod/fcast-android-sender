# Step 5 — `src/jni_bridge/mod.rs` (edit)

← [Step 4](step-4-codec-test-rs.md) · [Index](README.md) · Next → [Step 6](step-6-android-main-rs.md)

Add one line (keep alphabetical):

```rust
//! Java <-> Rust shims, grouped by the Java class that calls them.
//!
//! See refactor step 07 for context.

pub mod camera;
pub mod codec_test;          // ← ADD THIS LINE
pub mod discovery;
pub mod gstpop_bridge;
pub mod helpers;
pub mod main_activity;
pub mod migration_bridge;
```

---

← [Step 4](step-4-codec-test-rs.md) · [Index](README.md) · Next → [Step 6](step-6-android-main-rs.md)
