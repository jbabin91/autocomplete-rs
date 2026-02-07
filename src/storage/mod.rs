mod actor;
pub mod events;
pub mod queries;
pub mod schema;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::debug;
use turso::Connection;

pub use self::events::{DiagnosticCategory, Severity, StorageEvent};
pub use self::queries::{DiagnoseReport, query_diagnose_report};

/// Sender half of the storage event channel.
pub type StorageEventSender = mpsc::Sender<StorageEvent>;

/// Channel capacity for storage events.
const CHANNEL_CAPACITY: usize = 1024;

/// Timeout for waiting on the actor to shut down.
const SHUTDOWN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// Handle to the storage actor, used for lifecycle management.
pub struct StorageHandle {
    pub sender: StorageEventSender,
    actor_handle: JoinHandle<()>,
}

impl StorageHandle {
    /// Send a `Flush` sentinel and wait for the actor to exit (with timeout).
    pub async fn shutdown(self) {
        // Best-effort send of Flush with timeout — if the actor is stalled
        // or the channel is full, don't block shutdown indefinitely.
        match tokio::time::timeout(SHUTDOWN_TIMEOUT, self.sender.send(StorageEvent::Flush)).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => debug!("failed to send Flush to storage actor (channel closed): {e}"),
            Err(_) => {
                tracing::warn!("timed out sending Flush to storage actor, proceeding to abort")
            }
        }

        // Use select! instead of timeout() so the JoinHandle isn't consumed
        // on timeout — we can still abort and observe the task afterward.
        let abort = self.actor_handle.abort_handle();
        let mut handle = self.actor_handle;

        let join_result = tokio::select! {
            res = &mut handle => res,
            () = tokio::time::sleep(SHUTDOWN_TIMEOUT) => {
                tracing::warn!("storage actor shutdown timed out, aborting");
                abort.abort();
                // Observe the cancellation so the task doesn't outlive shutdown.
                handle.await
            }
        };

        match join_result {
            Ok(()) => debug!("storage actor shut down cleanly"),
            Err(e) => {
                if !e.is_cancelled() {
                    tracing::warn!("storage actor panicked: {e}");
                }
            }
        }
    }
}

/// Initialize the storage layer: open DB, run migrations, spawn actor.
pub async fn init(db_path: &Path) -> Result<StorageHandle> {
    ensure_data_dir(db_path)?;

    let db_path_str = db_path
        .to_str()
        .context("database path is not valid UTF-8")?;

    let db = turso::Builder::new_local(db_path_str)
        .build()
        .await
        .context("failed to open storage database")?;

    let conn = db
        .connect()
        .context("failed to connect to storage database")?;
    schema::run_migrations(&conn)
        .await
        .context("failed to run storage migrations")?;

    let actor_conn = db.connect().context("failed to create actor connection")?;
    let (tx, rx) = mpsc::channel(CHANNEL_CAPACITY);

    let actor_handle = tokio::spawn(actor::run(actor_conn, rx));

    Ok(StorageHandle {
        sender: tx,
        actor_handle,
    })
}

/// Open a connection to the database intended for read-only use (e.g. the diagnose command).
///
/// Note: turso does not expose a read-only connection mode, so callers are
/// responsible for not issuing writes through this connection.
pub async fn open_readonly(db_path: &Path) -> Result<Connection> {
    let db_path_str = db_path
        .to_str()
        .context("database path is not valid UTF-8")?;

    let db = turso::Builder::new_local(db_path_str)
        .build()
        .await
        .context("failed to open storage database")?;
    db.connect()
        .context("failed to connect to storage database")
}

/// Return the default database path: `~/.autocomplete-rs/autocomplete.db`.
pub fn default_db_path() -> PathBuf {
    dirs_or_home()
        .join(".autocomplete-rs")
        .join("autocomplete.db")
}

/// Ensure the parent directory of the DB file exists with 0700 permissions.
fn ensure_data_dir(db_path: &Path) -> Result<()> {
    let dir = db_path
        .parent()
        .context("database path has no parent directory")?;

    if dir.exists() {
        let metadata = fs::metadata(dir).context("failed to read data directory metadata")?;
        if !metadata.is_dir() {
            bail!(
                "data directory path {} exists but is not a directory",
                dir.display()
            );
        }
        let perms = metadata.permissions().mode() & 0o777;
        if perms & 0o077 != 0 {
            bail!(
                "data directory {} has insecure permissions {:o} (expected 0700)",
                dir.display(),
                perms
            );
        }
        Ok(())
    } else {
        fs::create_dir_all(dir).context("failed to create data directory")?;
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))
            .context("failed to set data directory permissions")?;
        Ok(())
    }
}

/// Fallback home directory resolution.
fn dirs_or_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}
