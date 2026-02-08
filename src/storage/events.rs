use serde::Serialize;

/// Events sent from the daemon to the storage actor for persistence.
#[derive(Debug)]
pub enum StorageEvent {
    /// Daemon session started.
    SessionStart {
        session_id: String,
        pid: u32,
        version: String,
        mode: String,
        socket_path: String,
    },
    /// Daemon session stopped.
    SessionStop { session_id: String, reason: String },
    /// A diagnostic event (error, warning, or info).
    Diagnostic {
        session_id: String,
        request_id: Option<String>,
        severity: Severity,
        category: DiagnosticCategory,
        message: String,
        context: Option<String>,
    },
    /// Periodic metrics snapshot.
    MetricsSnapshot {
        session_id: String,
        total_requests: u64,
        active_connections: u64,
        uptime_secs: u64,
    },
    /// Sentinel for clean shutdown — triggers final drain and actor exit.
    Flush,
}

/// Severity level for diagnostic events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Error => write!(f, "error"),
            Self::Warning => write!(f, "warning"),
            Self::Info => write!(f, "info"),
        }
    }
}

/// Category for diagnostic events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticCategory {
    Connection,
    Protocol,
    Engine,
    Internal,
    Storage,
}

impl std::fmt::Display for DiagnosticCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connection => write!(f, "connection"),
            Self::Protocol => write!(f, "protocol"),
            Self::Engine => write!(f, "engine"),
            Self::Internal => write!(f, "internal"),
            Self::Storage => write!(f, "storage"),
        }
    }
}
