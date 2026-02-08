use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

use autocomplete_rs::daemon::pid::derive_pid_path;
use autocomplete_rs::daemon::state::DaemonState;
use autocomplete_rs::engine::StubEngine;
use autocomplete_rs::logging;
use autocomplete_rs::protocol::{CompletionResponse, ErrorResponse, ShutdownAck};

/// Atomic counter to ensure unique socket paths across parallel tests.
static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Generate a unique temp socket path to avoid test collisions.
fn temp_socket_path() -> PathBuf {
    let pid = std::process::id();
    let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    PathBuf::from(format!("/tmp/autocomplete-rs-test-{}-{}.sock", pid, id))
}

/// Start a daemon in the background, returning a handle to shut it down.
async fn start_daemon(socket_path: &Path) -> tokio::task::JoinHandle<()> {
    let path = socket_path.to_path_buf();
    let state = DaemonState::new(Arc::new(StubEngine), logging::Mode::Production);
    let cancel = state.cancel.clone();

    // Remove stale socket (NotFound is expected; anything else is a real problem)
    if let Err(e) = std::fs::remove_file(&path)
        && e.kind() != std::io::ErrorKind::NotFound
    {
        panic!(
            "failed to remove stale test socket {}: {}",
            path.display(),
            e
        );
    }

    let listener = tokio::net::UnixListener::bind(&path).unwrap();

    // Set permissions
    let perms = std::fs::Permissions::from_mode(0o600);
    std::fs::set_permissions(&path, perms).unwrap();

    let path2 = path.clone();
    let handle = tokio::spawn(async move {
        let mut tasks = tokio::task::JoinSet::new();

        loop {
            tokio::select! {
                biased;
                () = cancel.cancelled() => break,
                result = listener.accept() => {
                    match result {
                        Ok((stream, _)) => {
                            let state = state.clone();
                            tasks.spawn(async move {
                                let (r, w) = stream.into_split();
                                if let Err(e) =
                                    autocomplete_rs::daemon::handler::handle_connection(
                                        r, w, &state, 0,
                                    )
                                    .await
                                {
                                    eprintln!("test handler error: {e}");
                                }
                            });
                        }
                        Err(_) => break,
                    }
                }
            }
        }

        // Drain tasks
        while tasks.join_next().await.is_some() {}

        if let Err(e) = std::fs::remove_file(&path2)
            && e.kind() != std::io::ErrorKind::NotFound
        {
            panic!("failed to clean up test socket {}: {}", path2.display(), e);
        }
    });

    // Wait for socket to be ready
    for _ in 0..50 {
        if path.exists() {
            // Try connecting to verify it's listening
            if UnixStream::connect(&path).await.is_ok() {
                return handle;
            }
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("daemon did not start in time");
}

/// Shut down the daemon, assert the shutdown ack, and assert it exits within the timeout.
///
/// Grabs an `AbortHandle` before awaiting so the task is cancelled on timeout
/// instead of silently leaked (dropping a `JoinHandle` detaches the task).
async fn shutdown_daemon(socket_path: &Path, handle: tokio::task::JoinHandle<()>) {
    let resp = send_request(socket_path, r#"{"type":"shutdown"}"#).await;
    let ack: serde_json::Value =
        serde_json::from_str(&resp).expect("shutdown response is valid JSON");
    assert_eq!(
        ack["status"], "shutting_down",
        "expected ShutdownAck, got: {resp}"
    );
    let abort = handle.abort_handle();
    match tokio::time::timeout(Duration::from_secs(2), handle).await {
        Ok(result) => result.expect("daemon task panicked"),
        Err(_) => {
            abort.abort();
            panic!("daemon did not exit within timeout — task aborted");
        }
    }
}

/// Send a JSON line to the daemon and read the response.
///
/// Wraps the entire operation in a 5-second timeout to prevent test hangs
/// if the daemon fails to respond.
async fn send_request(socket_path: &Path, json: &str) -> String {
    tokio::time::timeout(Duration::from_secs(5), async {
        let stream = UnixStream::connect(socket_path).await.unwrap();
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        writer.write_all(json.as_bytes()).await.unwrap();
        writer.write_all(b"\n").await.unwrap();
        writer.flush().await.unwrap();
        drop(writer);

        let mut response = String::new();
        reader.read_line(&mut response).await.unwrap();
        response.trim().to_string()
    })
    .await
    .expect("send_request timed out after 5s")
}

#[tokio::test]
async fn start_connect_complete() {
    let path = temp_socket_path();
    let handle = start_daemon(&path).await;

    let resp = send_request(&path, r#"{"buffer":"git ","cursor":4}"#).await;
    let parsed: CompletionResponse = serde_json::from_str(&resp).unwrap();
    assert!(parsed.suggestions.is_empty());

    shutdown_daemon(&path, handle).await;
}

#[tokio::test]
async fn shutdown_message_clean_exit() {
    let path = temp_socket_path();
    let handle = start_daemon(&path).await;

    let resp = send_request(&path, r#"{"type":"shutdown"}"#).await;
    let ack: ShutdownAck = serde_json::from_str(&resp).unwrap();
    assert_eq!(ack.status, "shutting_down");

    // Daemon should exit cleanly
    let abort = handle.abort_handle();
    match tokio::time::timeout(Duration::from_secs(2), handle).await {
        Ok(result) => result.expect("daemon task panicked"),
        Err(_) => {
            abort.abort();
            panic!("daemon did not exit in time — task aborted");
        }
    }

    // Socket should be cleaned up
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(
        !path.exists(),
        "socket file should be removed after shutdown"
    );
}

#[tokio::test]
async fn socket_permissions() {
    let path = temp_socket_path();
    let handle = start_daemon(&path).await;

    let metadata = std::fs::metadata(&path).unwrap();
    let mode = metadata.permissions().mode() & 0o777;
    // We set 0o600, so all group/other bits should be zero.
    assert_eq!(
        mode & 0o077,
        0,
        "socket should not be group/other accessible"
    );

    shutdown_daemon(&path, handle).await;
}

#[tokio::test]
async fn concurrent_connections() {
    let path = temp_socket_path();
    let handle = start_daemon(&path).await;

    // Send 10 concurrent requests (cursor always within buffer bounds)
    let mut handles = Vec::new();
    for i in 0..10 {
        let p = path.clone();
        handles.push(tokio::spawn(async move {
            let buffer = format!("cmd{}", i);
            let cursor = 0; // always valid
            let req = format!(r#"{{"buffer":"{}","cursor":{}}}"#, buffer, cursor);
            let resp = send_request(&p, &req).await;
            let parsed: CompletionResponse = serde_json::from_str(&resp).unwrap();
            assert!(parsed.suggestions.is_empty());
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    shutdown_daemon(&path, handle).await;
}

#[tokio::test]
async fn malformed_json_returns_error() {
    let path = temp_socket_path();
    let handle = start_daemon(&path).await;

    let resp = send_request(&path, "not json").await;
    let parsed: ErrorResponse = serde_json::from_str(&resp).unwrap();
    assert!(parsed.error.contains("invalid JSON"));

    shutdown_daemon(&path, handle).await;
}

#[tokio::test]
async fn pid_file_path_derivation() {
    let socket = PathBuf::from("/tmp/autocomplete-rs.sock");
    let pid = derive_pid_path(&socket);
    assert_eq!(pid, PathBuf::from("/tmp/autocomplete-rs.pid"));
}

#[tokio::test]
async fn envelope_and_bare_request_both_work() {
    let path = temp_socket_path();
    let handle = start_daemon(&path).await;

    // Bare request (backward compat)
    let resp1 = send_request(&path, r#"{"buffer":"ls","cursor":2}"#).await;
    let _: CompletionResponse = serde_json::from_str(&resp1).unwrap();

    // Envelope request
    let resp2 = send_request(&path, r#"{"type":"complete","buffer":"ls","cursor":2}"#).await;
    let _: CompletionResponse = serde_json::from_str(&resp2).unwrap();

    shutdown_daemon(&path, handle).await;
}
