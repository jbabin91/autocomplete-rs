use std::path::Path;
use std::sync::atomic::Ordering;
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::net::UnixListener;
use tokio::signal;
use tokio::task::JoinSet;
use tracing::{debug, error, info, warn};

use super::handler;
use super::state::DaemonState;

/// Timeout for draining in-flight connections during shutdown.
const DRAIN_TIMEOUT: Duration = Duration::from_secs(5);

/// Run the daemon accept loop until shutdown.
///
/// Sets socket permissions to `0o600`, accepts connections with semaphore-based
/// backpressure, and orchestrates graceful shutdown.
pub async fn run(listener: UnixListener, state: DaemonState, socket_path: &Path) -> Result<()> {
    // Set socket permissions to owner-only (0600)
    set_socket_permissions(socket_path)?;

    let mut tasks = JoinSet::new();
    let mut conn_counter: u64 = 0;

    info!("daemon accepting connections on {}", socket_path.display());

    // Create signal future once, outside the loop, to avoid re-registering per iteration.
    let mut sigint = std::pin::pin!(signal::ctrl_c());

    loop {
        tokio::select! {
            biased;

            // Check for cancellation first (from shutdown message)
            () = state.cancel.cancelled() => {
                info!("shutdown requested via protocol message");
                break;
            }

            // Handle Ctrl+C
            result = &mut sigint => {
                match result {
                    Ok(()) => info!("received SIGINT, shutting down..."),
                    Err(e) => error!("signal handler error: {}", e),
                }
                state.cancel.cancel();
                break;
            }

            // Accept new connections
            result = listener.accept() => {
                match result {
                    Ok((stream, _addr)) => {
                        // Try to acquire a semaphore permit (backpressure)
                        let permit = match state.semaphore.clone().try_acquire_owned() {
                            Ok(permit) => permit,
                            Err(_) => {
                                warn!("connection limit reached, rejecting");
                                drop(stream);
                                continue;
                            }
                        };

                        conn_counter += 1;
                        let conn_id = conn_counter;
                        let state = state.clone();

                        tasks.spawn(async move {
                            let (reader, writer) = stream.into_split();
                            if let Err(e) =
                                handler::handle_connection(reader, writer, &state, conn_id).await
                            {
                                error!(conn_id, "connection error: {}", e);
                            }
                            // Release semaphore permit when done
                            drop(permit);
                        });
                    }
                    Err(e) => {
                        error!("failed to accept connection: {}", e);
                    }
                }
            }
        }
    }

    // Drain in-flight connections
    drain_connections(&mut tasks, &state).await;

    info!("daemon shut down gracefully");
    Ok(())
}

/// Wait for in-flight connections to complete, with a timeout.
async fn drain_connections(tasks: &mut JoinSet<()>, state: &DaemonState) {
    let active = state.active_connections.load(Ordering::Relaxed);
    if active > 0 {
        info!(active, "waiting for in-flight connections to drain...");
    }

    let drain = async { while tasks.join_next().await.is_some() {} };

    if tokio::time::timeout(DRAIN_TIMEOUT, drain).await.is_err() {
        let remaining = state.active_connections.load(Ordering::Relaxed);
        warn!(
            remaining,
            "drain timeout exceeded, aborting remaining connections"
        );
        tasks.abort_all();
        // Observe cancelled tasks so they don't outlive the JoinSet.
        while tasks.join_next().await.is_some() {}
    }

    debug!("all connections drained");
}

/// Set socket file permissions to `0o600` (owner read/write only).
fn set_socket_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let perms = std::fs::Permissions::from_mode(0o600);
    std::fs::set_permissions(path, perms)
        .with_context(|| format!("failed to set socket permissions on {}", path.display()))?;
    debug!(path = %path.display(), "socket permissions set to 0600");
    Ok(())
}
