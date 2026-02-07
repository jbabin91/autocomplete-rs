use anyhow::{Context, Result};
use libsql::Connection;
use serde::Serialize;

/// Complete diagnostic report for the `diagnose` CLI command.
#[derive(Debug, Serialize)]
pub struct DiagnoseReport {
    pub recent_sessions: Vec<SessionSummary>,
    pub recent_errors: Vec<DiagnosticEventRow>,
    pub latest_metrics: Option<MetricsRow>,
    pub error_counts_by_category: Vec<CategoryCount>,
}

/// Summary of a daemon session.
#[derive(Debug, Serialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub pid: i64,
    pub version: String,
    pub mode: String,
    pub socket_path: String,
    pub started_at: String,
    pub stopped_at: Option<String>,
    pub stop_reason: Option<String>,
}

/// A single diagnostic event row.
#[derive(Debug, Serialize)]
pub struct DiagnosticEventRow {
    pub session_id: String,
    pub request_id: Option<String>,
    pub timestamp: String,
    pub severity: String,
    pub category: String,
    pub message: String,
    pub context: Option<String>,
}

/// A metrics snapshot row.
#[derive(Debug, Serialize)]
pub struct MetricsRow {
    pub session_id: String,
    pub timestamp: String,
    pub total_requests: i64,
    pub active_connections: i64,
    pub uptime_secs: i64,
}

/// Error count grouped by category.
#[derive(Debug, Serialize)]
pub struct CategoryCount {
    pub category: String,
    pub count: i64,
}

/// Query all data needed for the diagnose report.
pub async fn query_diagnose_report(conn: &Connection) -> Result<DiagnoseReport> {
    let recent_sessions = query_recent_sessions(conn).await?;
    let recent_errors = query_recent_errors(conn).await?;
    let latest_metrics = query_latest_metrics(conn).await?;
    let error_counts_by_category = query_error_counts(conn).await?;

    Ok(DiagnoseReport {
        recent_sessions,
        recent_errors,
        latest_metrics,
        error_counts_by_category,
    })
}

async fn query_recent_sessions(conn: &Connection) -> Result<Vec<SessionSummary>> {
    let mut rows = conn
        .query(
            "SELECT session_id, pid, version, mode, socket_path, started_at, stopped_at, stop_reason \
             FROM daemon_sessions ORDER BY started_at DESC LIMIT 10",
            (),
        )
        .await
        .context("failed to query recent sessions")?;

    let mut sessions = Vec::new();
    while let Some(row) = rows.next().await.context("failed to read session row")? {
        sessions.push(SessionSummary {
            session_id: row.get(0).context("session_id")?,
            pid: row.get(1).context("pid")?,
            version: row.get(2).context("version")?,
            mode: row.get(3).context("mode")?,
            socket_path: row.get(4).context("socket_path")?,
            started_at: row.get(5).context("started_at")?,
            stopped_at: row.get::<Option<String>>(6).context("stopped_at")?,
            stop_reason: row.get::<Option<String>>(7).context("stop_reason")?,
        });
    }
    Ok(sessions)
}

async fn query_recent_errors(conn: &Connection) -> Result<Vec<DiagnosticEventRow>> {
    let mut rows = conn
        .query(
            "SELECT session_id, request_id, timestamp, severity, category, message, context \
             FROM diagnostic_events \
             WHERE severity IN ('error', 'warning') \
             ORDER BY timestamp DESC LIMIT 25",
            (),
        )
        .await
        .context("failed to query recent errors")?;

    let mut events = Vec::new();
    while let Some(row) = rows.next().await.context("failed to read diagnostic row")? {
        events.push(DiagnosticEventRow {
            session_id: row.get(0).context("session_id")?,
            request_id: row.get::<Option<String>>(1).context("request_id")?,
            timestamp: row.get(2).context("timestamp")?,
            severity: row.get(3).context("severity")?,
            category: row.get(4).context("category")?,
            message: row.get(5).context("message")?,
            context: row.get::<Option<String>>(6).context("context")?,
        });
    }
    Ok(events)
}

async fn query_latest_metrics(conn: &Connection) -> Result<Option<MetricsRow>> {
    let mut rows = conn
        .query(
            "SELECT session_id, timestamp, total_requests, active_connections, uptime_secs \
             FROM metrics_snapshots ORDER BY timestamp DESC LIMIT 1",
            (),
        )
        .await
        .context("failed to query latest metrics")?;

    match rows.next().await.context("failed to read metrics row")? {
        Some(row) => Ok(Some(MetricsRow {
            session_id: row.get(0).context("session_id")?,
            timestamp: row.get(1).context("timestamp")?,
            total_requests: row.get(2).context("total_requests")?,
            active_connections: row.get(3).context("active_connections")?,
            uptime_secs: row.get(4).context("uptime_secs")?,
        })),
        None => Ok(None),
    }
}

async fn query_error_counts(conn: &Connection) -> Result<Vec<CategoryCount>> {
    let mut rows = conn
        .query(
            "SELECT category, COUNT(*) as cnt \
             FROM diagnostic_events \
             WHERE severity IN ('error', 'warning') \
             AND timestamp >= datetime('now', '-1 day') \
             GROUP BY category ORDER BY cnt DESC",
            (),
        )
        .await
        .context("failed to query error counts")?;

    let mut counts = Vec::new();
    while let Some(row) = rows.next().await.context("failed to read count row")? {
        counts.push(CategoryCount {
            category: row.get(0).context("category")?,
            count: row.get(1).context("count")?,
        });
    }
    Ok(counts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::schema::run_migrations;

    async fn setup() -> Connection {
        let db = libsql::Builder::new_local(":memory:")
            .build()
            .await
            .unwrap();
        let conn = db.connect().unwrap();
        run_migrations(&conn).await.unwrap();
        conn
    }

    #[tokio::test]
    async fn empty_report() {
        let conn = setup().await;
        let report = query_diagnose_report(&conn).await.unwrap();
        assert!(report.recent_sessions.is_empty());
        assert!(report.recent_errors.is_empty());
        assert!(report.latest_metrics.is_none());
        assert!(report.error_counts_by_category.is_empty());
    }

    #[tokio::test]
    async fn report_with_data() {
        let conn = setup().await;

        // Insert test data
        conn.execute(
            "INSERT INTO daemon_sessions (session_id, pid, version, mode, socket_path) \
             VALUES ('s1', 42, '0.1.0', 'production', '/tmp/test.sock')",
            (),
        )
        .await
        .unwrap();

        conn.execute(
            "INSERT INTO diagnostic_events (session_id, severity, category, message) \
             VALUES ('s1', 'error', 'connection', 'test error')",
            (),
        )
        .await
        .unwrap();

        conn.execute(
            "INSERT INTO metrics_snapshots (session_id, total_requests, active_connections, uptime_secs) \
             VALUES ('s1', 100, 2, 3600)",
            (),
        )
        .await
        .unwrap();

        let report = query_diagnose_report(&conn).await.unwrap();
        assert_eq!(report.recent_sessions.len(), 1);
        assert_eq!(report.recent_sessions[0].session_id, "s1");
        assert_eq!(report.recent_errors.len(), 1);
        assert_eq!(report.recent_errors[0].message, "test error");
        assert!(report.latest_metrics.is_some());
        let metrics = report.latest_metrics.unwrap();
        assert_eq!(metrics.total_requests, 100);
        assert_eq!(metrics.uptime_secs, 3600);
    }
}
