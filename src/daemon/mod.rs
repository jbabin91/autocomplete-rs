pub mod handler;
pub mod pid;
mod server;
pub mod state;

use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
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
    let db_path = storage::default_db_path();
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
    let tokio_handle = std::thread::Builder::new()
        .name("tokio-runtime".into())
        .spawn(move || {
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
                let db_path = storage::default_db_path();
                if let Err(e) = run_daemon(
                    &socket,
                    Arc::new(ParserEngine::new()),
                    &db_path,
                    Some(channel),
                )
                .await
                {
                    tracing::error!("daemon error: {e:#}");
                }
            });
        })
        .context("failed to spawn Tokio thread")?;

    // Run winit on the main thread (blocks until exit)
    event_loop
        .run_app(OverlayApp::new(rx))
        .context("event loop error")?;

    // winit exited — wait for Tokio thread to shut down
    info!("overlay exited, waiting for daemon thread...");
    match tokio_handle.join() {
        Ok(()) => info!("daemon thread joined cleanly"),
        Err(e) => tracing::error!("daemon thread panicked: {e:?}"),
    }

    Ok(())
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

    // Single-instance enforcement
    let _pid_file = PidFile::acquire(path)?;
    info!("PID file acquired");

    // Initialize storage — degrade gracefully if it fails
    let storage_handle: Option<StorageHandle> = match storage::init(db_path).await {
        Ok(handle) => {
            info!("storage layer initialized");
            Some(handle)
        }
        Err(e) => {
            tracing::warn!("storage init failed, running in degraded mode: {e}");
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
