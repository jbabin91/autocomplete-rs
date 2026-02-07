use std::time::Duration;

use anyhow::Result;
use tokio::sync::mpsc;
use tracing::{debug, warn};
use turso::Connection;

use super::events::StorageEvent;

/// Maximum events to accumulate before flushing.
const BATCH_SIZE: usize = 50;

/// Maximum time to wait before flushing a partial batch.
const BATCH_TIMEOUT: Duration = Duration::from_millis(500);

/// Run the storage actor loop, consuming events and writing them in batched transactions.
pub async fn run(conn: Connection, mut rx: mpsc::Receiver<StorageEvent>) {
    let mut batch: Vec<StorageEvent> = Vec::with_capacity(BATCH_SIZE);

    loop {
        // Wait for the first event
        let event = match rx.recv().await {
            Some(event) => event,
            None => {
                // Channel closed — flush remaining and exit
                if !batch.is_empty() {
                    flush_batch(&conn, &mut batch).await;
                }
                debug!("storage actor: channel closed, exiting");
                return;
            }
        };

        // Check for Flush sentinel
        if matches!(event, StorageEvent::Flush) {
            if !batch.is_empty() {
                flush_batch(&conn, &mut batch).await;
            }
            debug!("storage actor: flush sentinel received, exiting");
            return;
        }

        batch.push(event);

        // Drain available events up to batch size or timeout
        let deadline = tokio::time::Instant::now() + BATCH_TIMEOUT;
        while batch.len() < BATCH_SIZE {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                break;
            }
            match tokio::time::timeout(remaining, rx.recv()).await {
                Ok(Some(StorageEvent::Flush)) => {
                    flush_batch(&conn, &mut batch).await;
                    debug!("storage actor: flush sentinel received, exiting");
                    return;
                }
                Ok(Some(event)) => batch.push(event),
                Ok(None) => {
                    // Channel closed
                    flush_batch(&conn, &mut batch).await;
                    debug!("storage actor: channel closed during drain, exiting");
                    return;
                }
                Err(_) => break, // Timeout — flush what we have
            }
        }

        flush_batch(&conn, &mut batch).await;
    }
}

/// Write a batch of events inside a single transaction.
async fn flush_batch(conn: &Connection, batch: &mut Vec<StorageEvent>) {
    if batch.is_empty() {
        return;
    }

    let count = batch.len();
    debug!(count, "storage actor: flushing batch");

    let tx = match conn.unchecked_transaction().await {
        Ok(tx) => tx,
        Err(e) => {
            warn!("storage: failed to begin transaction: {e}");
            batch.clear();
            return;
        }
    };

    for event in batch.drain(..) {
        if let Err(e) = write_event(&tx, event).await {
            warn!("storage: failed to write event: {e}");
            // Continue with remaining events — don't abort the batch
        }
    }

    if let Err(e) = tx.commit().await {
        warn!("storage: failed to commit batch: {e}");
    }
}

/// Write a single event to the database within a transaction.
async fn write_event(tx: &turso::transaction::Transaction<'_>, event: StorageEvent) -> Result<()> {
    match event {
        StorageEvent::SessionStart {
            session_id,
            pid,
            version,
            mode,
            socket_path,
        } => {
            tx.execute(
                "INSERT INTO daemon_sessions (session_id, pid, version, mode, socket_path) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                turso::params![session_id, i64::from(pid), version, mode, socket_path],
            )
            .await?;
        }
        StorageEvent::SessionStop { session_id, reason } => {
            tx.execute(
                "UPDATE daemon_sessions SET stopped_at = datetime('now'), stop_reason = ?1 \
                 WHERE session_id = ?2",
                turso::params![reason, session_id],
            )
            .await?;
        }
        StorageEvent::Diagnostic {
            session_id,
            request_id,
            severity,
            category,
            message,
            context,
        } => {
            tx.execute(
                "INSERT INTO diagnostic_events \
                 (session_id, request_id, severity, category, message, context) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                turso::params![
                    session_id,
                    request_id,
                    severity.to_string(),
                    category.to_string(),
                    message,
                    context
                ],
            )
            .await?;
        }
        StorageEvent::MetricsSnapshot {
            session_id,
            total_requests,
            active_connections,
            uptime_secs,
        } => {
            let total_requests = i64::try_from(total_requests).unwrap_or(i64::MAX);
            let active_connections = i64::try_from(active_connections).unwrap_or(i64::MAX);
            let uptime_secs = i64::try_from(uptime_secs).unwrap_or(i64::MAX);

            tx.execute(
                "INSERT INTO metrics_snapshots \
                 (session_id, total_requests, active_connections, uptime_secs) \
                 VALUES (?1, ?2, ?3, ?4)",
                turso::params![session_id, total_requests, active_connections, uptime_secs],
            )
            .await?;
        }
        StorageEvent::Flush => {
            // Handled by caller — should not reach here
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;
    use crate::storage::events::{DiagnosticCategory, Severity};
    use crate::storage::schema::run_migrations;

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    async fn setup() -> (
        Connection,
        mpsc::Sender<StorageEvent>,
        tokio::task::JoinHandle<()>,
        tempfile::TempDir,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let db_path = dir.path().join(format!("test-{id}.db"));

        let db = turso::Builder::new_local(db_path.to_str().unwrap())
            .build()
            .await
            .unwrap();
        let conn = db.connect().unwrap();
        run_migrations(&conn).await.unwrap();

        let actor_conn = db.connect().unwrap();
        let (tx, rx) = mpsc::channel(1024);

        let handle = tokio::spawn(run(actor_conn, rx));

        (conn, tx, handle, dir)
    }

    #[tokio::test]
    async fn session_lifecycle() {
        let (conn, tx, actor_handle, _dir) = setup().await;

        tx.send(StorageEvent::SessionStart {
            session_id: "sess-1".into(),
            pid: 42,
            version: "0.1.0".into(),
            mode: "production".into(),
            socket_path: "/tmp/test.sock".into(),
        })
        .await
        .unwrap();

        tx.send(StorageEvent::SessionStop {
            session_id: "sess-1".into(),
            reason: "shutdown".into(),
        })
        .await
        .unwrap();

        tx.send(StorageEvent::Flush).await.unwrap();
        actor_handle.await.unwrap();

        let mut rows = conn
            .query(
                "SELECT session_id, stop_reason FROM daemon_sessions WHERE session_id = 'sess-1'",
                (),
            )
            .await
            .unwrap();
        let row = rows.next().await.unwrap().unwrap();
        let id: String = row.get(0).unwrap();
        let reason: String = row.get(1).unwrap();
        assert_eq!(id, "sess-1");
        assert_eq!(reason, "shutdown");
    }

    #[tokio::test]
    async fn diagnostic_event_persisted() {
        let (conn, tx, actor_handle, _dir) = setup().await;

        tx.send(StorageEvent::Diagnostic {
            session_id: "sess-2".into(),
            request_id: Some("req-1".into()),
            severity: Severity::Warning,
            category: DiagnosticCategory::Connection,
            message: "read timeout".into(),
            context: None,
        })
        .await
        .unwrap();

        tx.send(StorageEvent::Flush).await.unwrap();
        actor_handle.await.unwrap();

        let mut rows = conn
            .query(
                "SELECT severity, category, message FROM diagnostic_events WHERE session_id = 'sess-2'",
                (),
            )
            .await
            .unwrap();
        let row = rows.next().await.unwrap().unwrap();
        let severity: String = row.get(0).unwrap();
        let category: String = row.get(1).unwrap();
        let message: String = row.get(2).unwrap();
        assert_eq!(severity, "warning");
        assert_eq!(category, "connection");
        assert_eq!(message, "read timeout");
    }

    #[tokio::test]
    async fn metrics_snapshot_persisted() {
        let (conn, tx, actor_handle, _dir) = setup().await;

        tx.send(StorageEvent::MetricsSnapshot {
            session_id: "sess-3".into(),
            total_requests: 100,
            active_connections: 5,
            uptime_secs: 3600,
        })
        .await
        .unwrap();

        tx.send(StorageEvent::Flush).await.unwrap();
        actor_handle.await.unwrap();

        let mut rows = conn
            .query(
                "SELECT total_requests, uptime_secs FROM metrics_snapshots WHERE session_id = 'sess-3'",
                (),
            )
            .await
            .unwrap();
        let row = rows.next().await.unwrap().unwrap();
        let total: i64 = row.get(0).unwrap();
        let uptime: i64 = row.get(1).unwrap();
        assert_eq!(total, 100);
        assert_eq!(uptime, 3600);
    }
}
