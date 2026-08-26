pub mod handler;
pub mod pid;
mod server;
pub mod state;

use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, bail};
use tokio::net::UnixListener;
use tracing::info;

use crate::engine::CompletionEngine;
use crate::logging;
use crate::overlay::app::OverlayApp;
use crate::parser::ParserEngine;
use crate::storage::{self, StorageEvent, StorageHandle};

use self::pid::PidFile;
use self::state::{DaemonState, OverlayChannel};

/// Start the daemon with the default `ParserEngine`.
pub async fn start(socket_path: &str) -> Result<()> {
    let db_path = storage::default_db_path()?;
    start_with_engine(socket_path, Arc::new(ParserEngine::new()), &db_path).await
}

/// Start the daemon with the overlay window on the main thread.
///
/// winit must own the main thread, so the Tokio runtime runs on a background
/// thread. They communicate via `std::sync::mpsc` + `EventLoopProxy::wake_up()`.
///
/// This is the entry point used by the `daemon` CLI command. Tests use
/// `start_with_engine()` directly (no winit involved).
pub fn start_with_overlay(socket_path: &str) -> Result<()> {
    use winit::event_loop::EventLoop;

    let event_loop = EventLoop::new().context("failed to create event loop")?;
    let (proxy, tx, rx) = OverlayApp::create_channel(&event_loop);

    let socket = socket_path.to_string();
    // The daemon runs on another thread, so its failure has to be carried back here by
    // hand; without this the process exits 0 and every supervisor believes it started.
    let daemon_result: Arc<Mutex<Option<anyhow::Error>>> = Arc::new(Mutex::new(None));
    let daemon_result_tx = Arc::clone(&daemon_result);
    let tokio_handle = std::thread::Builder::new()
        .name("tokio-runtime".into())
        .spawn(move || {
            // First statement in the closure: the runtime builder below can panic, and a
            // panic before this guard exists leaves winit blocked in Wait forever.
            let _wake_guard = WakeOnDrop(proxy.clone());

            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .worker_threads(2)
                .thread_name("tokio-worker")
                .build()
                .expect("failed to build Tokio runtime");

            let wake = {
                let proxy = proxy.clone();
                Arc::new(move || proxy.wake_up())
            };
            let channel = OverlayChannel::new(tx, wake);

            rt.block_on(async {
                let outcome = match storage::default_db_path() {
                    Ok(db_path) => {
                        run_daemon(
                            &socket,
                            Arc::new(ParserEngine::new()),
                            &db_path,
                            Some(channel),
                        )
                        .await
                    }
                    Err(e) => Err(e),
                };
                if let Err(e) = outcome {
                    tracing::error!("daemon error: {e:#}");
                    *daemon_result_tx.lock().unwrap_or_else(|e| e.into_inner()) = Some(e);
                }
            });
        })
        .context("failed to spawn Tokio thread")?;

    // Run winit on the main thread (blocks until exit). Its result is held rather than
    // propagated: the daemon thread must still be joined, and its error is the more
    // specific one when both fail.
    let loop_result = event_loop.run_app(OverlayApp::new(rx));

    // A failed event loop leaves the daemon still serving with nothing to cancel it, so
    // joining would block forever. Report the daemon's error if it already recorded one,
    // otherwise surface the loop error and let process exit reap the thread.
    if let Err(e) = loop_result {
        if let Some(daemon_err) = daemon_result
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            return Err(daemon_err);
        }
        return Err(e).context("event loop error");
    }

    info!("overlay exited, waiting for daemon thread...");
    let panicked = match tokio_handle.join() {
        Ok(()) => {
            info!("daemon thread joined cleanly");
            false
        }
        Err(e) => {
            tracing::error!("daemon thread panicked: {e:?}");
            true
        }
    };

    let daemon_err = daemon_result
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take();
    join_outcome(daemon_err, panicked)
}

/// Wakes the winit event loop on drop, including while unwinding from a panic.
///
/// Without this the loop stays blocked in `Wait` and the process never exits.
struct WakeOnDrop(winit::event_loop::EventLoopProxy);

impl Drop for WakeOnDrop {
    fn drop(&mut self) {
        self.0.wake_up();
    }
}

/// Reduce the daemon thread's outcome to a result.
///
/// Split out from [`start_with_overlay`] because winit owns the main thread there, which
/// makes the surrounding function untestable.
fn join_outcome(daemon_err: Option<anyhow::Error>, panicked: bool) -> Result<()> {
    match (daemon_err, panicked) {
        (Some(e), _) => Err(e),
        (None, true) => bail!("daemon thread panicked"),
        (None, false) => Ok(()),
    }
}

/// Start the daemon with a custom completion engine.
///
/// Acquires a PID file, binds the Unix socket, initializes storage at
/// `db_path` (degrading gracefully on failure), and runs the accept loop
/// until shutdown. Cleans up socket and PID file on exit.
pub async fn start_with_engine(
    socket_path: &str,
    engine: Arc<dyn CompletionEngine>,
    db_path: &Path,
) -> Result<()> {
    run_daemon(socket_path, engine, db_path, None).await
}

/// Shared daemon startup logic.
///
/// When `overlay_channel` is `Some`, completions are forwarded to the overlay
/// window. When `None`, the daemon runs headless (used by tests).
async fn run_daemon(
    socket_path: &str,
    engine: Arc<dyn CompletionEngine>,
    db_path: &Path,
    overlay_channel: Option<OverlayChannel>,
) -> Result<()> {
    let path = Path::new(socket_path);

    crate::paths::check_socket_path_len(path)?;

    // Must run before the PID file is acquired: it lives in this same directory. A
    // world-writable parent would let another local user pre-create the socket path and
    // accept our clients' connections. Only the default directory is ours to tighten; a
    // path the user chose is validated and reported on.
    let socket_dirs = crate::paths::ensure_private_parent(path)
        .with_context(|| format!("socket directory is not usable for {}", socket_path))?;
    crate::paths::log_dir_actions(&socket_dirs);

    // Single-instance enforcement
    let _pid_file = PidFile::acquire(path)?;
    info!("PID file acquired");

    // Checked up front so a rejected data directory is fatal; storage::init degrades
    // gracefully, which would otherwise demote a security failure to a warning.
    let db_dirs = crate::paths::ensure_private_parent(db_path)
        .with_context(|| format!("database directory is not usable for {}", db_path.display()))?;
    crate::paths::log_dir_actions(&db_dirs);

    // Initialize storage — degrade gracefully if it fails
    let storage_handle: Option<StorageHandle> = match storage::init(db_path).await {
        Ok(handle) => {
            info!("storage layer initialized");
            Some(handle)
        }
        Err(e) => {
            tracing::warn!("storage init failed, running in degraded mode: {e:#}");
            None
        }
    };

    // Remove existing socket if it exists (stale from a crash)
    if let Err(e) = std::fs::remove_file(path)
        && e.kind() != std::io::ErrorKind::NotFound
    {
        return Err(e).with_context(|| format!("failed to remove stale socket: {}", socket_path));
    }

    let listener = UnixListener::bind(path)
        .with_context(|| format!("failed to bind to socket: {}", socket_path))?;

    info!("daemon listening on {}", socket_path);

    let session_id = uuid::Uuid::new_v4().to_string();
    let mode = logging::detect_mode();

    let mut state = DaemonState::new(engine, mode.clone()).with_session_id(session_id.clone());
    if let Some(channel) = overlay_channel {
        state = state.with_overlay(channel);
    }
    if let Some(ref handle) = storage_handle {
        state = state.with_storage(handle.sender.clone());
    }

    // Emit session start
    state.emit_storage_event(StorageEvent::SessionStart {
        session_id: session_id.clone(),
        pid: std::process::id(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        mode: mode.to_string(),
        socket_path: socket_path.to_string(),
    });

    // Run the accept loop (blocks until shutdown)
    let shutdown_reason = server::run(listener, state, path).await;

    // Determine stop reason
    let reason = if shutdown_reason.is_ok() {
        "shutdown"
    } else {
        "error"
    };

    // Emit session stop — use send().await with timeout (not try_send) since
    // this is the shutdown path, not the hot path, and we want reliable delivery
    // so sessions don't remain "running" in the DB.
    if let Some(ref handle) = storage_handle {
        let stop_event = StorageEvent::SessionStop {
            session_id,
            reason: reason.to_string(),
        };
        match tokio::time::timeout(
            std::time::Duration::from_secs(2),
            handle.sender.send(stop_event),
        )
        .await
        {
            Ok(Ok(())) => {}
            Ok(Err(e)) => tracing::warn!("failed to emit session stop event: {e}"),
            Err(_) => tracing::warn!("timed out sending session stop event"),
        }
    }

    // Shut down storage actor (flushes remaining events including SessionStop)
    if let Some(handle) = storage_handle {
        handle.shutdown().await;
    }

    // Cleanup socket file (PID file cleaned up by Drop)
    if let Err(e) = std::fs::remove_file(path)
        && e.kind() != std::io::ErrorKind::NotFound
    {
        tracing::warn!("failed to remove socket on shutdown: {}", e);
    }

    shutdown_reason
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_outcome_reports_a_daemon_error() {
        let err = join_outcome(Some(anyhow::anyhow!("bind failed")), false)
            .expect_err("a daemon error must propagate");
        assert!(err.to_string().contains("bind failed"));
    }

    #[test]
    fn join_outcome_prefers_the_daemon_error_over_the_panic_flag() {
        let err = join_outcome(Some(anyhow::anyhow!("bind failed")), true)
            .expect_err("a daemon error must propagate");
        assert!(
            err.to_string().contains("bind failed"),
            "the specific error should win over the generic panic message: {err}"
        );
    }

    #[test]
    fn join_outcome_reports_a_bare_panic() {
        let err = join_outcome(None, true).expect_err("a panic must not look like success");
        assert!(err.to_string().contains("panicked"));
    }

    #[test]
    fn join_outcome_is_ok_on_clean_shutdown() {
        join_outcome(None, false).expect("a clean shutdown is not an error");
    }
}
