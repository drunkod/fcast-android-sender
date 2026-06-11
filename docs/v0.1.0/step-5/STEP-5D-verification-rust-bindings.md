# STEP-5D — Verification & generated Rust bindings

> No code edit. Confirms `bridge.slint` still compiles and documents the Rust
> binding names the backend will use.

---

## Slint compile check

```bash
slint-lsp ui/main.slint 2>&1 | grep -c error
# → 0
```

Or, since `bridge.slint` is re-exported from `main.slint` and compiled by
`slint_build::compile("ui/main.slint")`, a full build also validates it and
regenerates the Rust bindings:

```bash
./gradlew generateCargoSources 2>&1 | tail -20
# (or: cargo build — build.rs invokes slint_build)
```

---

## Generated Rust accessor names

Slint generates getters/setters on the `Bridge` global. kebab-case →
snake_case for members, PascalCase for enum/struct types:

| Slint member | Rust accessor |
|---|---|
| `Panel.protocol-rtmp-settings` | `Panel::ProtocolRtmpSettings` |
| `Panel.protocol-srt-settings` | `Panel::ProtocolSrtSettings` |
| `SrtDestination` struct | `SrtDestination` (fields: `uri`, `latency_ms`, `state`, `last_error`) |
| `srt-destination` property | `get_srt_destination()` / `set_srt_destination(SrtDestination)` |
| `srt-destination-passphrase` | `get_srt_destination_passphrase()` / `set_…` |
| `srt-destination-pbkeylen-idx` | `get_srt_destination_pbkeylen_idx()` / `set_…` |
| `start-srt-destination` callback | `on_start_srt_destination(impl Fn …)` |
| `stop-srt-destination` callback | `on_stop_srt_destination(…)` |
| `save-srt-destination-config` callback | `on_save_srt_destination_config(…)` |

> Struct field `latency-ms` becomes `latency_ms` and `last-error` becomes
> `last_error` in the generated Rust struct.

---

## Why "no Rust compilation required" holds

1. **New `Panel` variants** — every Rust `Panel` reference in `src/` is an
   equality check (`== Panel::CameraRtmpStream`, etc.), never an exhaustive
   `match`, so adding variants compiles unchanged.
2. **New callbacks** — Slint callbacks without an `on_…` handler are no-ops; the
   build does not require handlers to exist.
3. **New properties** — generated getters/setters are additive.

So STEP-5 can land independently of the backend wiring; the handlers in
STEP-5C's contract table are added when STEP-6/7 pages go live.

---

## Done — STEP-5 complete

| Sub-step | Status |
|---|---|
| 5A Panel enum | ✅ |
| 5B SrtDestination struct | ✅ |
| 5C Bridge properties + callbacks | ✅ |
| 5D verification & bindings | ✅ |

→ Next top-level step: [../step-6/INDEX.md](../step-6/INDEX.md)
