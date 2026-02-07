use autocomplete_rs::storage::events::{DiagnosticCategory, Severity};
use autocomplete_rs::storage::{self, StorageEvent};

#[tokio::test]
async fn full_lifecycle_with_tempdir() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("data").join("autocomplete.db");

    // Initialize storage
    let handle = storage::init(&db_path).await.unwrap();
    assert!(db_path.exists());

    let session_id = "integration-test-session";

    // Emit session start
    handle
        .sender
        .send(StorageEvent::SessionStart {
            session_id: session_id.into(),
            pid: std::process::id(),
            version: "0.1.0".into(),
            mode: "test".into(),
            socket_path: "/tmp/test.sock".into(),
        })
        .await
        .unwrap();

    // Emit some diagnostics
    handle
        .sender
        .send(StorageEvent::Diagnostic {
            session_id: session_id.into(),
            request_id: Some("req-1".into()),
            severity: Severity::Warning,
            category: DiagnosticCategory::Connection,
            message: "read timeout".into(),
            context: None,
        })
        .await
        .unwrap();

    handle
        .sender
        .send(StorageEvent::Diagnostic {
            session_id: session_id.into(),
            request_id: None,
            severity: Severity::Error,
            category: DiagnosticCategory::Protocol,
            message: "malformed JSON".into(),
            context: Some(r#"{"invalid": true}"#.into()),
        })
        .await
        .unwrap();

    // Emit metrics
    handle
        .sender
        .send(StorageEvent::MetricsSnapshot {
            session_id: session_id.into(),
            total_requests: 42,
            active_connections: 3,
            uptime_secs: 120,
        })
        .await
        .unwrap();

    // Emit session stop
    handle
        .sender
        .send(StorageEvent::SessionStop {
            session_id: session_id.into(),
            reason: "shutdown".into(),
        })
        .await
        .unwrap();

    // Shut down the actor (flushes remaining events)
    handle.shutdown().await;

    // Open and query the report
    let conn = storage::open_readonly(&db_path).await.unwrap();
    let report = storage::query_diagnose_report(&conn).await.unwrap();

    // Verify sessions
    assert_eq!(report.recent_sessions.len(), 1);
    assert_eq!(report.recent_sessions[0].session_id, session_id);
    assert_eq!(report.recent_sessions[0].pid, i64::from(std::process::id()));
    assert_eq!(
        report.recent_sessions[0].stop_reason.as_deref(),
        Some("shutdown")
    );

    // Verify errors (both warning and error should appear)
    assert_eq!(report.recent_errors.len(), 2);

    // Verify metrics
    assert!(report.latest_metrics.is_some());
    let metrics = report.latest_metrics.unwrap();
    assert_eq!(metrics.total_requests, 42);
    assert_eq!(metrics.active_connections, 3);
    assert_eq!(metrics.uptime_secs, 120);
}

#[tokio::test]
async fn diagnose_report_on_empty_db() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("data").join("empty.db");

    let handle = storage::init(&db_path).await.unwrap();
    handle.shutdown().await;

    let conn = storage::open_readonly(&db_path).await.unwrap();
    let report = storage::query_diagnose_report(&conn).await.unwrap();

    assert!(report.recent_sessions.is_empty());
    assert!(report.recent_errors.is_empty());
    assert!(report.latest_metrics.is_none());
    assert!(report.error_counts_by_category.is_empty());
}

#[tokio::test]
async fn data_dir_permissions() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = dir.path().join("autocomplete-rs");
    let db_path = data_dir.join("autocomplete.db");

    let handle = storage::init(&db_path).await.unwrap();
    handle.shutdown().await;

    // Check directory permissions
    let metadata = std::fs::metadata(&data_dir).unwrap();
    use std::os::unix::fs::PermissionsExt;
    let mode = metadata.permissions().mode() & 0o777;
    assert_eq!(
        mode, 0o700,
        "data directory should have exact 0700 permissions"
    );
}
