use anyhow::Result;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use super::LogConfig;

/// Build and install the global tracing subscriber. Returns guards that must
/// be held for the process lifetime (dropping them flushes pending logs).
pub fn build_subscriber(cfg: &LogConfig) -> Result<Vec<WorkerGuard>> {
    let mut guards = Vec::new();

    // Set umask for 0600 file permissions before creating the appender
    // SAFETY: umask is process-global but we only call this during init
    unsafe {
        libc::umask(0o077);
    }

    let file_appender =
        tracing_appender::rolling::daily(&cfg.log_dir, super::config::LOG_FILE_PREFIX);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    guards.push(guard);

    let env_filter = make_env_filter(&cfg.mode);

    // Each mode builds its own subscriber to avoid boxing type-erasure issues.
    // The `Option<Layer>` pattern works within a single `.with()` chain because
    // tracing-subscriber blanket-impls `Layer<S> for Option<L> where L: Layer<S>`,
    // but that requires the concrete layer type — not a trait object.
    match (&cfg.mode, cfg.enable_console) {
        (super::Mode::Troubleshooting, true) => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(
                    fmt::layer()
                        .json()
                        .with_writer(non_blocking)
                        .with_target(true)
                        .with_thread_ids(true)
                        .with_file(true)
                        .with_line_number(true),
                )
                .with(fmt::layer().compact().with_target(true))
                .init();
        }
        (super::Mode::Troubleshooting, false) => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(
                    fmt::layer()
                        .json()
                        .with_writer(non_blocking)
                        .with_target(true)
                        .with_thread_ids(true)
                        .with_file(true)
                        .with_line_number(true),
                )
                .init();
        }
        (super::Mode::Development, true) => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(
                    fmt::layer()
                        .compact()
                        .with_writer(non_blocking)
                        .with_target(true)
                        .with_ansi(false),
                )
                .with(fmt::layer().pretty().with_target(true))
                .init();
        }
        (super::Mode::Development, false) => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(
                    fmt::layer()
                        .compact()
                        .with_writer(non_blocking)
                        .with_target(true)
                        .with_ansi(false),
                )
                .init();
        }
        (_, true) => {
            // Production with console override
            tracing_subscriber::registry()
                .with(env_filter)
                .with(
                    fmt::layer()
                        .compact()
                        .with_writer(non_blocking)
                        .with_target(true)
                        .with_ansi(false),
                )
                .with(fmt::layer().compact().with_target(true))
                .init();
        }
        (_, false) => {
            // Production (default)
            tracing_subscriber::registry()
                .with(env_filter)
                .with(
                    fmt::layer()
                        .compact()
                        .with_writer(non_blocking)
                        .with_target(true)
                        .with_ansi(false),
                )
                .init();
        }
    }

    Ok(guards)
}

/// Build the env filter for the given mode.
fn make_env_filter(mode: &super::Mode) -> EnvFilter {
    match mode {
        super::Mode::Production => EnvFilter::new("autocomplete_rs=info"),
        super::Mode::Development => EnvFilter::new("autocomplete_rs=debug"),
        super::Mode::Troubleshooting => EnvFilter::from_default_env(),
    }
}
