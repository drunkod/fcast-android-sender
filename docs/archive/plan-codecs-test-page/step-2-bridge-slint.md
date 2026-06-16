# Step 2 — `ui/bridge.slint` (edit)

← [Step 1](step-1-kotlin-codecdump.md) · [Index](README.md) · Next → [Step 3](step-3-codec-test-page-slint.md)

Add inside `export global Bridge { … }`, just before its closing `}` (currently
**line 466**). Two `in` properties + three callbacks:

```slint
    // ── Codec test (Phase 8 wiring) ──────────────────────────────────────
    in property <string> codec-test-log: "";
    in property <bool>   codec-test-running: false;
    callback run-codec-test();        // full dump + quick-find + smoke test
    callback run-codec-dump-only();   // just the codec list dump (faster)
    callback run-codec-smoke-only();  // just the encoder smoke test
```

`codec-test-log` replaces the old `mock-log` stub; `codec-test-running` lets the UI
disable buttons and flip a status badge while a probe runs.

---

← [Step 1](step-1-kotlin-codecdump.md) · [Index](README.md) · Next → [Step 3](step-3-codec-test-page-slint.md)
