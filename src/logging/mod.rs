mod config;
mod fields;
mod layers;
mod privacy;

use std::path::PathBuf;
use std::sync::OnceLock;

use anyhow::{Context, Result};
use tracing::info;
use tracing_appender::non_blocking::WorkerGuard;

pub use self::fields::new_request_id;
pub use self::privacy::{RedactedField, redact_buffer, redact_sensitive_patterns, should_redact};

/// Logging mode, controlling verbosity, format, and redaction defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    /// Daemon default: info level, compact file format, buffers redacted.
    Production,
    /// `AUTOCOMPLETE_DEV=1`: debug level, pretty console + compact file, buffers visible.
    Development,
    /// `RUST_LOG` set: env-controlled level, optional console, JSON file, buffers redacted
    /// unless `AUTOCOMPLETE_LOG_FULL_BUFFERS=1`.
    Troubleshooting,
}

/// Configuration for the logging subsystem.
#[derive(Debug, Clone)]
pub struct LogConfig {
    pub mode: Mode,
    pub log_dir: PathBuf,
    pub retention_days: u32,
    pub enable_console: bool,
    pub redact_buffers: bool,
}

/// Guards that keep the non-blocking writer alive for the process lifetime.
static GUARDS: OnceLock<Vec<WorkerGuard>> = OnceLock::new();

/// Initialize logging with auto-detected settings.
///
/// Detects mode from environment variables, resolves log directory,
/// and installs the global tracing subscriber. Safe to call once.
pub fn init() -> Result<()> {
    let mode = config::detect_mode();
    let log_dir = config::resolve_log_dir();
    let retention_days = config::retention_days(&mode);
    let enable_console = config::console_enabled(&mode);
    let redact_buffers = should_redact(&mode);

    let cfg = LogConfig {
        mode,
        log_dir,
        retention_days,
        enable_console,
        redact_buffers,
    };

    init_with_config(cfg)
}

/// Initialize logging with explicit configuration.
///
/// Creates the log directory, installs the subscriber, and spawns
/// a background cleanup of old log files. Returns an error if called
/// more than once.
pub fn init_with_config(config: LogConfig) -> Result<()> {
    // Ensure log directory exists with correct permissions
    config::ensure_log_dir(&config.log_dir).context("failed to set up log directory")?;

    // Build and install subscriber
    let guards = layers::build_subscriber(&config).context("failed to build tracing subscriber")?;

    // Store guards; fail if already initialized
    GUARDS
        .set(guards)
        .map_err(|_| anyhow::anyhow!("logging already initialized"))?;

    info!(
        mode = ?config.mode,
        log_dir = %config.log_dir.display(),
        retention_days = config.retention_days,
        console = config.enable_console,
        redact = config.redact_buffers,
        "logging initialized"
    );

    // Clean up old logs (best-effort)
    let log_dir = config.log_dir.clone();
    let retention = config.retention_days;
    if let Err(e) = config::cleanup_old_logs(&log_dir, retention) {
        tracing::warn!("failed to clean up old logs: {e}");
    }

    Ok(())
}

/// Return the default log directory path (`~/.autocomplete-rs/logs/`).
pub fn default_log_dir() -> PathBuf {
    config::default_log_dir()
}
