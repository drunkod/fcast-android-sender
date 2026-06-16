# Step 7 — `src/lib.rs`: register module

← [Step 6](step-6-codec-perf-rs.md) · [Index](README.md) · Next → [Step 8](step-8-android-main-handlers.md)

Add `pub mod codec_perf;` after `pub mod application;` (keeps the list ordered):

```rust
pub mod app;
pub mod application;
pub mod codec_perf;          // ← ADD THIS LINE
pub mod command;
pub mod config;
pub mod jni_bridge;
pub mod log_ring;
pub mod platform;
pub mod secret;

#[cfg(target_os = "android")]
mod android_main;

mod backend;
mod gstpop_service;
mod migration_service;
```

> `codec_perf` is not `#[cfg(target_os = "android")]`-gated: it only uses the
> `gst` crate (host-buildable). The `amc*` factories simply won't exist on a host
> registry, so the benchmarks report "NOT FOUND" / fall back to `x264enc` there.

---

← [Step 6](step-6-codec-perf-rs.md) · [Index](README.md) · Next → [Step 8](step-8-android-main-handlers.md)
