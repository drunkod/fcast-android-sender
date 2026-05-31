//! End-to-end tests against a real embedded gst-pop server.
//!
//! These tests mutate process-global state in `gstpop-runtime::embedded`
//! and must run single-threaded. We use a process-global mutex to enforce
//! this even when cargo test runs them concurrently.

#![cfg(feature = "typed-client")]

use std::net::TcpListener;
use std::time::Duration;

use gstpop_runtime::{
    embedded_status, start_embedded, start_embedded_with_config, stop_embedded,
    EmbeddedConfig, EmbeddedState, Event, GstPopClient, TypedGstPopClient,
};

static TEST_MUTEX: once_cell::sync::Lazy<tokio::sync::Mutex<()>> =
    once_cell::sync::Lazy::new(|| tokio::sync::Mutex::new(()));

static GST_INIT: std::sync::Once = std::sync::Once::new();

fn init_gstreamer() {
    GST_INIT.call_once(|| {
        gst::init().expect("failed to initialize GStreamer");
    });
}

fn pick_free_port() -> u16 {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    l.local_addr().unwrap().port()
}

/// Ensure a clean slate even if a previous test panicked.
async fn hard_reset() {
    let _ = stop_embedded().await;
    // Grace period for sockets to release
    tokio::time::sleep(Duration::from_millis(200)).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn embedded_server_starts_and_stops_cleanly() {
    let _guard = TEST_MUTEX.lock().await;
    hard_reset().await;
    let port = pick_free_port();

    let status = start_embedded(port).await;
    assert!(
        matches!(status.state, EmbeddedState::Running),
        "expected Running, got {:?}; last_error={:?}",
        status.state,
        status.last_error,
    );
    assert_eq!(status.port, port);
    assert_eq!(status.bind, "127.0.0.1");

    let snap = embedded_status();
    assert!(matches!(snap.state, EmbeddedState::Running));

    let stopped = stop_embedded().await;
    assert!(matches!(stopped.state, EmbeddedState::Stopped));

    // Second stop is a no-op.
    let stopped_again = stop_embedded().await;
    assert!(matches!(stopped_again.state, EmbeddedState::Stopped));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_start_calls_converge() {
    let _guard = TEST_MUTEX.lock().await;
    hard_reset().await;
    let port = pick_free_port();

    let a = tokio::spawn(start_embedded(port));
    let b = tokio::spawn(start_embedded(port));
    let c = tokio::spawn(start_embedded(port));

    let results = [a.await.unwrap(), b.await.unwrap(), c.await.unwrap()];
    for r in &results {
        assert!(
            matches!(r.state, EmbeddedState::Running),
            "expected Running, got {:?}; last_error={:?}",
            r.state,
            r.last_error,
        );
        assert_eq!(r.port, port);
    }

    let _ = stop_embedded().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bind_failure_populates_last_error() {
    let _guard = TEST_MUTEX.lock().await;
    hard_reset().await;

    // Hold the port on 0.0.0.0 so the embedded server's 127.0.0.1 bind fails
    // (Linux allows this overlap to be detected; on macOS the listener on
    // 0.0.0.0 prevents the loopback bind from succeeding).
    let port = pick_free_port();
    let _blocker = TcpListener::bind(("0.0.0.0", port)).expect("hold port");

    let cfg = EmbeddedConfig {
        bind: "127.0.0.1".into(),
        port,
        api_key: None,
        allowed_origins: vec![],
    };
    let status = start_embedded_with_config(cfg).await;

    // On some platforms the overlap is permitted and the server adopts the
    // external listener. Accept either outcome but assert the negative
    // outcome carries a populated last_error.
    match status.state {
        EmbeddedState::Error => {
            assert!(
                status.last_error.is_some(),
                "Error state must populate last_error",
            );
        }
        EmbeddedState::Running => {
            // adopted; nothing to assert beyond not crashing
        }
        other => panic!("unexpected state {other:?}"),
    }

    drop(_blocker);
    let _ = stop_embedded().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn typed_client_round_trips_playback_lifecycle() {
    let _guard = TEST_MUTEX.lock().await;
    init_gstreamer();
    hard_reset().await;
    let port = pick_free_port();
    let status = start_embedded(port).await;
    assert!(matches!(status.state, EmbeddedState::Running));

    let url = format!("ws://127.0.0.1:{port}/");
    let inner = GstPopClient::connect(&url, None)
        .await
        .expect("client connect");
    let client = TypedGstPopClient::new(inner);

    // Trivially valid pipeline: no sources/sinks needed for create/remove.
    // `videotestsrc ! fakesink` requires gstreamer plugins, which are present
    // in the daemon's compiled-in set on test hosts.
    let pid = client
        .create_pipeline("videotestsrc ! fakesink")
        .await
        .expect("create_pipeline");

    let pipelines = client.list_pipelines().await.expect("list_pipelines");
    assert!(pipelines.iter().any(|p| p.id == pid));

    client.play(Some(&pid)).await.expect("play");
    tokio::time::sleep(Duration::from_millis(100)).await;
    client.pause(Some(&pid)).await.expect("pause");
    client.stop(Some(&pid)).await.expect("stop");
    client.remove_pipeline(&pid).await.expect("remove");

    let _ = stop_embedded().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_pipeline_changes_description() {
    let _guard = TEST_MUTEX.lock().await;
    init_gstreamer();
    hard_reset().await;
    let port = pick_free_port();
    let status = start_embedded(port).await;
    assert!(matches!(status.state, EmbeddedState::Running));

    let url = format!("ws://127.0.0.1:{port}/");
    let inner = GstPopClient::connect(&url, None).await.expect("client connect");
    let client = TypedGstPopClient::new(inner);

    let pid = client
        .create_pipeline("videotestsrc ! fakesink")
        .await
        .expect("create_pipeline");

    client
        .update_pipeline(&pid, "videotestsrc pattern=snow ! fakesink")
        .await
        .expect("update_pipeline");

    let pipelines = client.list_pipelines().await.expect("list_pipelines");
    let summary = pipelines.iter().find(|p| p.id == pid).expect("pipeline present after update");
    assert!(
        summary.description.contains("pattern=snow"),
        "description should be updated, got: {}",
        summary.description
    );

    client.remove_pipeline(&pid).await.expect("remove");
    let _ = stop_embedded().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn client_receives_state_changed_events() {
    let _guard = TEST_MUTEX.lock().await;
    init_gstreamer();
    hard_reset().await;
    let port = pick_free_port();
    let status = start_embedded(port).await;
    assert!(matches!(status.state, EmbeddedState::Running));

    let url = format!("ws://127.0.0.1:{port}/");
    let inner = GstPopClient::connect(&url, None).await.expect("client connect");
    let mut events = inner.subscribe();
    let client = TypedGstPopClient::new(inner);

    let pid = client
        .create_pipeline("videotestsrc ! fakesink")
        .await
        .expect("create_pipeline");

    client.play(Some(&pid)).await.expect("play");

    // Drain events for up to 2s, looking for a StateChanged event for our pipeline.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    let mut saw_state_changed = false;
    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_millis(200), events.recv()).await {
            Ok(Ok(gstpop_runtime::Event::StateChanged { pipeline_id, .. })) if pipeline_id == pid => {
                saw_state_changed = true;
                break;
            }
            Ok(Ok(_)) => continue,
            Ok(Err(_)) => break,
            Err(_) => break,
        }
    }
    assert!(saw_state_changed, "expected StateChanged event for pipeline {pid}");

    client.stop(Some(&pid)).await.expect("stop");
    client.remove_pipeline(&pid).await.expect("remove");
    let _ = stop_embedded().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_pipelines_empty_after_removes() {
    let _guard = TEST_MUTEX.lock().await;
    init_gstreamer();
    hard_reset().await;
    let port = pick_free_port();
    let status = start_embedded(port).await;
    assert!(matches!(status.state, EmbeddedState::Running));

    let url = format!("ws://127.0.0.1:{port}/");
    let inner = GstPopClient::connect(&url, None).await.expect("client connect");
    let client = TypedGstPopClient::new(inner);

    let p1 = client.create_pipeline("videotestsrc ! fakesink").await.expect("create p1");
    let p2 = client.create_pipeline("videotestsrc ! fakesink").await.expect("create p2");

    let before = client.list_pipelines().await.expect("list before");
    assert!(before.iter().any(|p| p.id == p1));
    assert!(before.iter().any(|p| p.id == p2));

    client.remove_pipeline(&p1).await.expect("remove p1");
    client.remove_pipeline(&p2).await.expect("remove p2");

    let after = client.list_pipelines().await.expect("list after");
    assert!(!after.iter().any(|p| p.id == p1), "p1 should be gone");
    assert!(!after.iter().any(|p| p.id == p2), "p2 should be gone");

    let _ = stop_embedded().await;
}
