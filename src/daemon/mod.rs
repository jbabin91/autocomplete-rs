pub mod handler;
pub mod pid;
mod server;
pub mod state;

use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::net::UnixListener;
use tracing::info;

use crate::engine::{CompletionEngine, StubEngine};
use crate::logging;
use crate::storage::{self, StorageEvent, StorageHandle};

use self::pid::PidFile;
use self::state::DaemonState;

/// Start the daemon with the default `StubEngine`.
pub async fn start(socket_path: &str) -> Result<()> {
    start_with_engine(
        socket_path,
        Arc::new(StubEngine),
        &storage::default_db_path(),
    )
    .await
}

/// Start the daemon with a custom completion engine.
///
/// Acquires a PID file, binds the Unix socket, and runs the accept loop
/// until shutdown. Cleans up socket and PID file on exit.
pub async fn start_with_engine(
    socket_path: &str,
    engine: Arc<dyn CompletionEngine>,
    db_path: &Path,
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
