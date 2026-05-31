# vendor/gstpop divergences from upstream `dabrain34/gstpop/daemon`

Tracked so an upstream sync does not silently revert Android-relevant changes.

| File | Change | Reason |
|---|---|---|
| `src/server.rs` | Pre-binds `TcpListener` before spawn | Surface bind errors synchronously for Android `EmbeddedStatus.last_error` |
| `src/websocket/server.rs` | `run(self, listener, event_rx)` takes pre-bound listener | Pair of the above |
| `Cargo.toml` | `gstreamer`/`gstreamer-pbutils` bumped to `0.25` | Workspace dependency alignment |
| `src/lib.rs` | License header trimmed, `signal` module removed | Library-only, no CLI |
| (removed) `src/main.rs`, `src/cmd/`, `src/signal.rs`, `src/cli_tests.rs` | CLI surface intentionally absent | Library-only build for embedded use |
