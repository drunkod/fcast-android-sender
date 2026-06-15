# STEP-5A — Panel variants

**File:** `ui/bridge.slint` — add to the `Panel` enum (after `protocol-srt-settings`):

```slint
    // v0.2.0 scenes & widgets
    scene-list,
    scene-edit,
    widget-wizard,
    widget-settings,
    scene-widget-layout,
```

> Verified in v0.1.0: every Rust `Panel` reference is `== Panel::Variant`, never
> an exhaustive `match`, so new variants are additive and don't break the build.

→ Next: [STEP-5B-structs-enum.md](STEP-5B-structs-enum.md)
