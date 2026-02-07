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

use self::pid::PidFile;
use self::state::DaemonState;

/// Start the daemon with the default `StubEngine`.
pub async fn start(socket_path: &str) -> Result<()> {
    start_with_engine(socket_path, Arc::new(StubEngine)).await
}

/// Start the daemon with a custom completion engine.
///
/// Acquires a PID file, binds the Unix socket, and runs the accept loop
/// until shutdown. Cleans up socket and PID file on exit.
pub async fn start_with_engine(socket_path: &str, engine: Arc<dyn CompletionEngine>) -> Result<()> {
    let path = Path::new(socket_path);

    // Single-instance enforcement
    let _pid_file = PidFile::acquire(path)?;
    info!("PID file acquired");

    // Remove existing socket if it exists (stale from a crash)
    if let Err(e) = std::fs::remove_file(path) {
        if e.kind() != std::io::ErrorKind::NotFound {
            return Err(e)
                .with_context(|| format!("failed to remove stale socket: {}", socket_path));
        }
    }

    let listener = UnixListener::bind(path)
        .with_context(|| format!("failed to bind to socket: {}", socket_path))?;
    info!("daemon listening on {}", socket_path);

    let state = DaemonState::new(engine);

    // Run the accept loop (blocks until shutdown)
    let result = server::run(listener, state, path).await;

    // Cleanup socket file (PID file cleaned up by Drop)
    if let Err(e) = std::fs::remove_file(path) {
        if e.kind() != std::io::ErrorKind::NotFound {
            tracing::warn!("failed to remove socket on shutdown: {}", e);
        }
    }

    result
}
