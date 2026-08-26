use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

use autocomplete_rs::daemon::pid::derive_pid_path;
use autocomplete_rs::daemon::state::DaemonState;
use autocomplete_rs::engine::{CompletionEngine, StubEngine};
use autocomplete_rs::logging;
use autocomplete_rs::protocol::{CompletionResponse, ErrorResponse, ShutdownAck};

/// Atomic counter to ensure unique socket paths across parallel tests.
static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Shared 0700 directory holding test sockets, reused across runs.
///
/// Not a `TempDir`: destructors never run for `static`s, so a `OnceLock<TempDir>` leaks a
/// fresh directory per test process instead of reaping it. A single deterministic root
/// keeps that bounded. The name is predictable, so it is validated rather than trusted —
/// `symlink_metadata` refuses a symlink pointing somewhere world-writable, and the mode
/// and owner must both be ours. Short segments keep sockets clear of the `sun_path` limit.
fn test_socket_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        use std::os::unix::fs::{DirBuilderExt, MetadataExt};

        // SAFETY: `getuid` takes no arguments, cannot fail, and has no side effects.
        let uid = unsafe { libc::getuid() };
        let dir = PathBuf::from(format!("/tmp/acrs-test-{uid}"));

        // symlink_metadata, not metadata: a symlink planted here must not be followed.
        match std::fs::symlink_metadata(&dir) {
            Ok(meta) => {
                assert!(
                    meta.is_dir(),
                    "{} must be a directory, not a symlink or file",
                    dir.display()
                );
                assert_eq!(
                    meta.uid(),
                    uid,
                    "{} is owned by another user",
                    dir.display()
                );
                assert_eq!(
                    meta.permissions().mode() & 0o777,
                    0o700,
                    "{} must be private",
                    dir.display()
                );
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                std::fs::DirBuilder::new()
                    .recursive(true)
                    .mode(0o700)
                    .create(&dir)
                    .unwrap_or_else(|e| panic!("failed to create {}: {e}", dir.display()));
            }
            Err(e) => panic!("failed to stat {}: {e}", dir.display()),
        }
        dir
    })
    .as_path()
}

/// A private directory under the shared root that is removed when the test ends.
fn scoped_socket_dir(prefix: &str) -> tempfile::TempDir {
    let dir = tempfile::Builder::new()
        .prefix(prefix)
        .tempdir_in(test_socket_dir())
        .expect("failed to create scoped socket directory");
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700))
        .expect("failed to restrict scoped socket directory");
    dir
}

/// Generate a unique temp socket path to avoid test collisions.
fn temp_socket_path() -> PathBuf {
    test_socket_dir().join(format!("{}.sock", unique_suffix()))
}

/// Unique within this machine: the root is shared across concurrent test processes.
fn unique_suffix() -> String {
    format!(
        "{}-{}",
        std::process::id(),
        TEST_COUNTER.fetch_add(1, Ordering::Relaxed)
    )
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
    let socket = PathBuf::from("/home/u/.autocomplete-rs/daemon.sock");
    let pid = derive_pid_path(&socket);
    assert_eq!(pid, PathBuf::from("/home/u/.autocomplete-rs/daemon.pid"));
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

/// The daemon must create a missing socket directory at 0700 before it needs it.
///
/// Regression: deleting the `ensure_private_parent` call in `run_daemon` left every test
/// passing, because the shared helper pre-creates its directory at 0700 and the guard
/// took the already-private path. This test starts from a directory that does not exist,
/// which also pins the ordering — `PidFile::acquire` writes into this same directory and
/// fails outright if the guard has not run yet.
#[tokio::test]
async fn full_entrypoint_creates_missing_socket_directory() {
    let scope = scoped_socket_dir("miss-");
    let parent = scope.path().join("d");
    let socket = parent.join("d.sock");
    assert!(
        std::fs::metadata(&parent).is_err(),
        "precondition: parent must not exist"
    );

    let db = private_tempdir();
    let handle = start_full_daemon(&socket, &db.path().join("test.db")).await;

    let meta = std::fs::metadata(&parent).expect("daemon should have created the parent");
    assert_eq!(
        meta.permissions().mode() & 0o777,
        0o700,
        "socket directory must be created private"
    );
    assert!(
        std::fs::metadata(derive_pid_path(&socket)).is_ok(),
        "PID file should have landed in the created directory"
    );

    shutdown_full_daemon(&socket, handle).await;
}

/// A group/other-accessible socket directory the user chose is refused, not modified.
///
/// Repairing is reserved for the default data directory; silently chmodding a path the
/// user pointed at could revoke access to unrelated files beneath it.
#[tokio::test]
async fn full_entrypoint_rejects_open_socket_directory() {
    let scope = scoped_socket_dir("open-");
    let parent = scope.path().join("d");
    std::fs::create_dir(&parent).unwrap();
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o755)).unwrap();
    let socket = parent.join("d.sock");

    let db = private_tempdir();
    let db_path = db.path().join("test.db");
    let engine: Arc<dyn CompletionEngine> = Arc::new(StubEngine);
    let err = tokio::time::timeout(
        Duration::from_secs(10),
        autocomplete_rs::daemon::start_with_engine(socket.to_str().unwrap(), engine, &db_path),
    )
    .await
    .expect("daemon must fail fast, not start and serve")
    .expect_err("an open socket directory must be refused");

    let msg = format!("{err:#}");
    assert!(
        msg.contains("group/other accessible"),
        "error should name the problem: {msg}"
    );
    assert_eq!(
        std::fs::metadata(&parent).unwrap().permissions().mode() & 0o777,
        0o755,
        "a user-chosen directory must be left untouched"
    );
    assert!(
        std::fs::metadata(&socket).is_err(),
        "no socket should have been created"
    );
}

/// A database directory that cannot exist is fatal, not a silent degrade.
///
/// Regression: `storage::init` degrades gracefully on failure, which previously demoted a
/// rejected data directory to a warning while the daemon carried on serving.
#[tokio::test]
async fn full_entrypoint_rejects_unusable_database_directory() {
    let scope = scoped_socket_dir("dbbl-");
    let parent = scope.path().join("d");
    std::fs::create_dir(&parent).unwrap();
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o700)).unwrap();

    // A regular file where the database's directory should be.
    let blocker = parent.join("blocked");
    std::fs::write(&blocker, b"not a directory").unwrap();

    let socket = parent.join("d.sock");
    let engine: Arc<dyn CompletionEngine> = Arc::new(StubEngine);
    let err = tokio::time::timeout(
        Duration::from_secs(10),
        autocomplete_rs::daemon::start_with_engine(
            socket.to_str().unwrap(),
            engine,
            &blocker.join("autocomplete.db"),
        ),
    )
    .await
    .expect("daemon must fail fast, not start and serve")
    .expect_err("an unusable database directory must be refused");

    assert!(
        format!("{err:#}").contains("database directory is not usable"),
        "unexpected error: {err:#}"
    );
}

/// An over-long socket path is rejected with guidance, not an opaque `bind` failure.
#[tokio::test]
async fn full_entrypoint_rejects_over_long_socket_path() {
    let long_name = "x".repeat(autocomplete_rs::paths::MAX_SOCKET_PATH_LEN);
    let socket = test_socket_dir().join(format!("{long_name}.sock"));

    let db = private_tempdir();
    let engine: Arc<dyn CompletionEngine> = Arc::new(StubEngine);
    let err = tokio::time::timeout(
        Duration::from_secs(10),
        autocomplete_rs::daemon::start_with_engine(
            socket.to_str().unwrap(),
            engine,
            &db.path().join("test.db"),
        ),
    )
    .await
    .expect("daemon must fail fast, not start and serve")
    .expect_err("an over-long socket path must be refused");

    assert!(
        format!("{err:#}").contains("AUTOCOMPLETE_RS_SOCKET"),
        "error should tell the user how to fix it: {err:#}"
    );
}

// ─── Full entrypoint tests ──────────────────────────────────────────────────
//
// These tests exercise `daemon::start_with_engine` — the real daemon lifecycle
// including PID file acquisition, storage initialization, session events, and
// cleanup. The tests above use `start_daemon` which manually builds the accept
// loop and skips PID/storage/session orchestration.

/// Start the daemon via the real `start_with_engine` entrypoint.
///
/// Spawns the daemon in a background task with a temp DB and waits for the
/// socket to become connectable before returning.
async fn start_full_daemon(
    socket_path: &Path,
    db_path: &Path,
) -> tokio::task::JoinHandle<anyhow::Result<()>> {
    let socket_str = socket_path.to_str().unwrap().to_string();
    let db = db_path.to_path_buf();
    let engine: Arc<dyn CompletionEngine> = Arc::new(StubEngine);

    let handle = tokio::spawn(async move {
        autocomplete_rs::daemon::start_with_engine(&socket_str, engine, &db).await
    });

    // Wait for socket to be ready (polling with retry — storage init may take
    // a moment due to DB creation and migrations). Check for early task failure
    // to surface startup errors instead of timing out with a generic message.
    for _ in 0..100 {
        if handle.is_finished() {
            match handle.await {
                Ok(Ok(())) => panic!("daemon exited immediately without error"),
                Ok(Err(e)) => panic!("daemon failed to start: {e:#}"),
                Err(e) => panic!("daemon task panicked: {e}"),
            }
        }
        if UnixStream::connect(socket_path).await.is_ok() {
            return handle;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("full daemon did not start in time");
}

/// Shut down a full daemon and await its completion.
///
/// Asserts the shutdown ack, then uses `select!` with `&mut handle` instead
/// of `timeout(handle)` to avoid consuming the JoinHandle on timeout — this
/// ensures we can abort and observe the task for clean cleanup.
async fn shutdown_full_daemon(
    socket_path: &Path,
    mut handle: tokio::task::JoinHandle<anyhow::Result<()>>,
) {
    let resp = send_request(socket_path, r#"{"type":"shutdown"}"#).await;
    let ack: serde_json::Value =
        serde_json::from_str(&resp).expect("shutdown response is valid JSON");
    assert_eq!(
        ack["status"], "shutting_down",
        "expected ShutdownAck, got: {resp}"
    );

    tokio::select! {
        result = &mut handle => {
            result
                .expect("daemon task panicked")
                .expect("daemon returned error");
        }
        () = tokio::time::sleep(Duration::from_secs(5)) => {
            handle.abort();
            // Observe the aborted task to ensure cleanup (PID file, socket).
            let _ = handle.await;
            panic!("full daemon did not exit within timeout");
        }
    }
}

#[tokio::test]
async fn full_entrypoint_lifecycle() {
    let dir = private_tempdir();
    let socket_path = temp_socket_path();
    let db_path = dir.path().join("data").join("autocomplete.db");
    let pid_path = derive_pid_path(&socket_path);

    let handle = start_full_daemon(&socket_path, &db_path).await;

    // PID file should exist and contain our process ID
    let pid_contents = std::fs::read_to_string(&pid_path).unwrap();
    assert_eq!(
        pid_contents.trim(),
        std::process::id().to_string(),
        "PID file should contain the current process ID"
    );

    // Send a completion request through the full stack
    let resp = send_request(&socket_path, r#"{"buffer":"git ","cursor":4}"#).await;
    let parsed: CompletionResponse = serde_json::from_str(&resp).unwrap();
    assert!(parsed.suggestions.is_empty());

    // Shut down via protocol message
    shutdown_full_daemon(&socket_path, handle).await;

    // Socket should be cleaned up
    assert!(
        !socket_path.exists(),
        "socket file should be removed after shutdown"
    );
    // PID file should be cleaned up (RAII Drop)
    assert!(
        !pid_path.exists(),
        "PID file should be removed after shutdown"
    );
}

#[tokio::test]
async fn full_entrypoint_storage_records_session() {
    let dir = private_tempdir();
    let socket_path = temp_socket_path();
    let db_path = dir.path().join("data").join("autocomplete.db");

    let handle = start_full_daemon(&socket_path, &db_path).await;

    // Send a request so there's activity
    let resp = send_request(&socket_path, r#"{"buffer":"ls","cursor":2}"#).await;
    let _: CompletionResponse = serde_json::from_str(&resp).unwrap();

    // Shut down — this flushes storage before returning
    shutdown_full_daemon(&socket_path, handle).await;

    // Query the database for session events
    let conn = autocomplete_rs::storage::open_readonly(&db_path)
        .await
        .unwrap();
    let report = autocomplete_rs::storage::query_diagnose_report(&conn)
        .await
        .unwrap();

    // Should have exactly one session
    assert_eq!(
        report.recent_sessions.len(),
        1,
        "should record exactly one session"
    );
    let session = &report.recent_sessions[0];
    assert_eq!(session.pid, i64::from(std::process::id()));
    assert_eq!(session.stop_reason.as_deref(), Some("shutdown"));
}

#[tokio::test]
async fn full_entrypoint_stale_socket_cleanup() {
    let socket_path = temp_socket_path();
    let dir = private_tempdir();
    let db_path = dir.path().join("data").join("autocomplete.db");

    // Create a stale socket file (simulates leftover from a crash)
    std::fs::write(&socket_path, "stale").unwrap();
    assert!(socket_path.exists());

    // Daemon should remove the stale socket and bind successfully
    let handle = start_full_daemon(&socket_path, &db_path).await;

    // Verify it's actually working
    let resp = send_request(&socket_path, r#"{"buffer":"ls","cursor":2}"#).await;
    let _: CompletionResponse = serde_json::from_str(&resp).unwrap();

    shutdown_full_daemon(&socket_path, handle).await;
}

#[tokio::test]
async fn full_entrypoint_double_start_rejected() {
    let dir = private_tempdir();
    let socket_path = temp_socket_path();
    let db_path = dir.path().join("data").join("autocomplete.db");

    // Start the first daemon
    let handle = start_full_daemon(&socket_path, &db_path).await;

    // Try to start a second daemon on the same socket — should fail because
    // the PID file exists and references our (still-alive) process.
    let socket_str = socket_path.to_str().unwrap().to_string();
    let db2 = dir.path().join("data2").join("autocomplete.db");
    let engine: Arc<dyn CompletionEngine> = Arc::new(StubEngine);
    let result = autocomplete_rs::daemon::start_with_engine(&socket_str, engine, &db2).await;

    assert!(result.is_err(), "second daemon should fail to start");
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("another daemon is already running"),
        "error should mention another daemon is running"
    );

    // Clean up first daemon
    shutdown_full_daemon(&socket_path, handle).await;
}

/// A temp directory at mode 0700.
///
/// `tempfile` honours the umask (0755 in practice), but the daemon refuses to put private
/// data in a group/other-accessible directory — as a real user's `~/.autocomplete-rs`
/// would never be.
fn private_tempdir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700))
        .expect("failed to restrict temp dir");
    dir
}
