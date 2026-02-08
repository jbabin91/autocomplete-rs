use anyhow::{Context, Result};
use turso::Connection;

/// Run all pending migrations, returning the final schema version.
pub async fn run_migrations(conn: &Connection) -> Result<u32> {
    // Bootstrap the schema_version table
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER NOT NULL,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )
    .await
    .context("failed to create schema_version table")?;

    let current = current_version(conn).await?;

    if current < 1 {
        apply_v1(conn).await?;
        record_version(conn, 1).await?;
    }

    current_version(conn).await
}

async fn current_version(conn: &Connection) -> Result<u32> {
    let mut rows = conn
        .query("SELECT COALESCE(MAX(version), 0) FROM schema_version", ())
        .await
        .context("failed to query schema version")?;

    let row = rows
        .next()
        .await
        .context("failed to read schema version row")?;
    match row {
        Some(row) => {
            let version: i64 = row.get(0).context("failed to get version column")?;
            u32::try_from(version).context("schema_version value out of range for u32")
        }
        None => Ok(0),
    }
}

async fn record_version(conn: &Connection, version: u32) -> Result<()> {
    conn.execute(
        "INSERT INTO schema_version (version) VALUES (?1)",
        turso::params![i64::from(version)],
    )
    .await
    .context("failed to record schema version")?;
    Ok(())
}

async fn apply_v1(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE daemon_sessions (
            session_id TEXT PRIMARY KEY,
            pid INTEGER NOT NULL,
            version TEXT NOT NULL,
            mode TEXT NOT NULL,
            socket_path TEXT NOT NULL,
            started_at TEXT NOT NULL DEFAULT (datetime('now')),
            stopped_at TEXT,
            stop_reason TEXT
        );

        CREATE TABLE diagnostic_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT NOT NULL,
            request_id TEXT,
            timestamp TEXT NOT NULL DEFAULT (datetime('now')),
            severity TEXT NOT NULL,
            category TEXT NOT NULL,
            message TEXT NOT NULL,
            context TEXT
        );

        CREATE INDEX idx_diagnostic_session ON diagnostic_events(session_id);
        CREATE INDEX idx_diagnostic_timestamp ON diagnostic_events(timestamp);
        CREATE INDEX idx_diagnostic_severity ON diagnostic_events(severity);

        CREATE TABLE metrics_snapshots (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT NOT NULL,
            timestamp TEXT NOT NULL DEFAULT (datetime('now')),
            total_requests INTEGER NOT NULL,
            active_connections INTEGER NOT NULL,
            uptime_secs INTEGER NOT NULL
        );

        CREATE INDEX idx_metrics_session ON metrics_snapshots(session_id);",
    )
    .await
    .context("failed to apply v1 migration")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn in_memory_conn() -> Connection {
        let db = turso::Builder::new_local(":memory:").build().await.unwrap();
        db.connect().unwrap()
    }

    #[tokio::test]
    async fn migrations_create_tables() {
        let conn = in_memory_conn().await;
        let version = run_migrations(&conn).await.unwrap();
        assert_eq!(version, 1);

        // Verify tables exist by inserting into each
        conn.execute(
            "INSERT INTO daemon_sessions (session_id, pid, version, mode, socket_path) \
             VALUES ('test', 1, '0.1.0', 'production', '/tmp/test.sock')",
            (),
        )
        .await
        .unwrap();

        conn.execute(
            "INSERT INTO diagnostic_events (session_id, severity, category, message) \
             VALUES ('test', 'error', 'connection', 'test error')",
            (),
        )
        .await
        .unwrap();

        conn.execute(
            "INSERT INTO metrics_snapshots (session_id, total_requests, active_connections, uptime_secs) \
             VALUES ('test', 10, 2, 60)",
            (),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn migrations_idempotent() {
        let conn = in_memory_conn().await;
        let v1 = run_migrations(&conn).await.unwrap();
        let v2 = run_migrations(&conn).await.unwrap();
        assert_eq!(v1, v2);
    }
}
