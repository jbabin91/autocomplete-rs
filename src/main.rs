use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use autocomplete_rs::daemon;
use autocomplete_rs::protocol::{CompletionRequest, DaemonMessage, PROTOCOL_VERSION, ShutdownAck};
use autocomplete_rs::storage;
use clap::{Parser, Subcommand};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

/// Resolve the socket path from the flag, the environment, or the default.
///
/// The default depends on `$HOME`, which can be missing, so it is resolved here rather
/// than as a clap `default_value` — an unset `$HOME` becomes a clear error instead of a
/// path under `/tmp`.
fn resolve_socket(explicit: Option<String>) -> Result<String> {
    match explicit {
        Some(path) => Ok(path),
        None => Ok(autocomplete_rs::paths::default_socket_path()?
            .to_string_lossy()
            .into_owned()),
    }
}

#[derive(Parser)]
#[command(name = "autocomplete-rs")]
#[command(about = "Fast, universal terminal autocomplete", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the autocomplete daemon
    Daemon {
        /// Unix socket path [default: ~/.autocomplete-rs/daemon.sock]
        #[arg(short, long, env = "AUTOCOMPLETE_RS_SOCKET")]
        socket: Option<String>,
    },
    /// Stop the running daemon
    Stop {
        /// Unix socket path [default: ~/.autocomplete-rs/daemon.sock]
        #[arg(short, long, env = "AUTOCOMPLETE_RS_SOCKET")]
        socket: Option<String>,
    },
    /// Check daemon status
    Status {
        /// Unix socket path [default: ~/.autocomplete-rs/daemon.sock]
        #[arg(short, long, env = "AUTOCOMPLETE_RS_SOCKET")]
        socket: Option<String>,
    },
    /// Get completion suggestions for a command buffer
    Complete {
        /// Command buffer to complete
        buffer: String,
        /// Cursor position in the buffer
        #[arg(short, long)]
        cursor: usize,
        /// Unix socket path [default: ~/.autocomplete-rs/daemon.sock]
        #[arg(short, long, env = "AUTOCOMPLETE_RS_SOCKET")]
        socket: Option<String>,
    },
    /// Install shell integration
    Install {
        /// Shell to install for (zsh, bash, fish)
        shell: String,
    },
    /// Show diagnostic report from the storage database
    Diagnose {
        /// Path to the storage database [default: ~/.autocomplete-rs/autocomplete.db]
        #[arg(long)]
        db: Option<PathBuf>,
        /// Output as JSON instead of human-readable format
        #[arg(long)]
        json: bool,
    },
}

/// Build a single-threaded Tokio runtime and run the given future to completion.
fn block_on<F: std::future::Future<Output = Result<()>>>(f: F) -> Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("failed to build runtime")?;
    rt.block_on(f)
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Daemon { socket } => {
            let socket = resolve_socket(socket)?;

            // Initialize logging before starting the daemon
            autocomplete_rs::logging::init().context("failed to initialize logging")?;
            tracing::info!("Starting autocomplete daemon on {}", socket);

            // winit must own the main thread; Tokio runs on a background thread.
            daemon::start_with_overlay(&socket)?;
        }
        Commands::Stop { socket } => block_on(stop_daemon(&resolve_socket(socket)?))?,
        Commands::Status { socket } => block_on(status_command(&resolve_socket(socket)?))?,
        Commands::Complete {
            buffer,
            cursor,
            socket,
        } => block_on(complete_command(&buffer, cursor, &resolve_socket(socket)?))?,
        Commands::Install { shell } => {
            install_command(&shell)?;
        }
        Commands::Diagnose { db, json } => {
            let db = match db {
                Some(path) => path,
                None => storage::default_db_path()?,
            };
            block_on(diagnose_command(&db, json))?
        }
    }

    Ok(())
}

/// Handle the complete command: connect to daemon, get suggestions, output as JSON
async fn complete_command(buffer: &str, cursor: usize, socket_path: &str) -> Result<()> {
    // Connect to daemon
    let stream = UnixStream::connect(socket_path)
        .await
        .context("Failed to connect to daemon. Is it running?")?;

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Send request using the DaemonMessage envelope
    let msg = DaemonMessage::Complete(CompletionRequest {
        buffer: buffer.to_string(),
        cursor,
        version: PROTOCOL_VERSION,
    });
    let request_json = serde_json::to_string(&msg)?;
    writer.write_all(request_json.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;

    // Read response
    let mut response_line = String::new();
    // A zero-byte read is EOF, not a response: the daemon closed without answering.
    // Printing the empty string here would look like "no suggestions" to the widget.
    if reader.read_line(&mut response_line).await? == 0 {
        bail!(
            "the daemon at {socket_path} closed the connection without responding; \
             it may have crashed — check ~/.autocomplete-rs/logs/"
        );
    }

    // Output raw response for shell integration to consume
    print!("{}", response_line.trim());

    Ok(())
}

/// Stop the running daemon by sending a shutdown message over the socket.
async fn stop_daemon(socket_path: &str) -> Result<()> {
    use std::path::Path;

    if !Path::new(socket_path).exists() {
        println!("Daemon is not running (socket not found)");
        return Ok(());
    }

    match UnixStream::connect(socket_path).await {
        Ok(stream) => {
            let (reader, mut writer) = stream.into_split();
            let mut reader = BufReader::new(reader);

            // Send shutdown message
            let msg = DaemonMessage::Shutdown;
            let json = serde_json::to_string(&msg)?;
            writer.write_all(json.as_bytes()).await?;
            writer.write_all(b"\n").await?;
            writer.flush().await?;

            // Wait for acknowledgement with timeout
            let mut ack_line = String::new();
            match tokio::time::timeout(Duration::from_secs(5), reader.read_line(&mut ack_line))
                .await
            {
                Ok(Ok(0)) => {
                    // EOF — daemon closed the connection without responding
                    println!("Daemon stopped (connection closed)");
                }
                Ok(Ok(_)) => match serde_json::from_str::<ShutdownAck>(&ack_line) {
                    Ok(ack) if ack.status == "shutting_down" => {
                        println!("Daemon stopped");
                    }
                    Ok(ack) => {
                        println!("Daemon responded with unexpected status: {}", ack.status);
                    }
                    Err(_) => {
                        println!("Daemon responded unexpectedly: {}", ack_line.trim());
                    }
                },
                Ok(Err(e)) => {
                    bail!("could not read the daemon's shutdown acknowledgement: {e}")
                }
                Err(_) => bail!("timed out waiting for the daemon to acknowledge shutdown"),
            }
        }
        Err(e) => {
            // Only remove the socket if the error indicates nothing is listening
            // (ConnectionRefused). Other errors (PermissionDenied, transient FS
            // errors) could mean a live daemon we can't reach — deleting its
            // socket would break it.
            if e.kind() == std::io::ErrorKind::ConnectionRefused {
                if let Err(rm_err) = std::fs::remove_file(socket_path)
                    && rm_err.kind() != std::io::ErrorKind::NotFound
                {
                    return Err(rm_err)
                        .context(format!("failed to remove stale socket: {socket_path}"));
                }
                println!("Removed stale socket (daemon was not running)");
            } else {
                return Err(e).context(format!(
                    "could not reach the daemon at {socket_path}; it may be running as \
                     another user, or the socket may be unreadable"
                ));
            }
        }
    }

    Ok(())
}

/// Check daemon status
async fn status_command(socket_path: &str) -> Result<()> {
    use std::path::Path;

    if !Path::new(socket_path).exists() {
        println!("Daemon is not running (socket not found)");
        return Ok(());
    }

    // Try to connect to verify daemon is responsive
    match UnixStream::connect(socket_path).await {
        Ok(_stream) => {
            println!("Daemon is running on {}", socket_path);
        }
        Err(e) => {
            println!("Socket exists but daemon is not responding (stale socket: {e})");
        }
    }

    Ok(())
}

/// Install shell integration
fn install_command(shell: &str) -> Result<()> {
    match shell {
        "zsh" => {
            println!("To install autocomplete-rs for zsh, add this to your ~/.zshrc:");
            println!();
            println!("# autocomplete-rs");
            println!("source <(autocomplete-rs shell-init zsh)");
            println!();
            println!("Or manually source the integration script:");
            println!("source /path/to/autocomplete-rs/shell-integration/zsh.zsh");
        }
        _ => {
            anyhow::bail!(
                "Unsupported shell: {}. Currently only 'zsh' is supported.",
                shell
            );
        }
    }
    Ok(())
}

/// Show diagnostic report from the storage database.
async fn diagnose_command(db_path: &std::path::Path, json: bool) -> Result<()> {
    match std::fs::metadata(db_path) {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            println!("No storage database found at {}", db_path.display());
            println!("The daemon creates this database on first run.");
            return Ok(());
        }
        Err(e) => {
            anyhow::bail!(
                "cannot access storage database at {}: {e}",
                db_path.display()
            );
        }
    }

    let conn = storage::open_readonly(db_path)
        .await
        .context("failed to open storage database")?;

    let report = storage::query_diagnose_report(&conn)
        .await
        .context("failed to query diagnostic report")?;

    if json {
        let output = serde_json::to_string_pretty(&report)?;
        println!("{output}");
        return Ok(());
    }

    // Human-readable output
    println!("autocomplete-rs diagnostic report");
    println!("=================================");
    println!();

    // System info
    println!("System:");
    println!("  Version:  {}", env!("CARGO_PKG_VERSION"));
    println!(
        "  OS:       {} {}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    if let Ok(shell) = std::env::var("SHELL") {
        println!("  Shell:    {shell}");
    }
    println!("  DB path:  {}", db_path.display());
    match std::fs::metadata(db_path) {
        Ok(metadata) => println!("  DB size:  {} KB", metadata.len() / 1024),
        Err(e) => println!("  DB size:  unavailable ({e})"),
    }
    match autocomplete_rs::logging::default_log_dir() {
        Ok(dir) => println!("  Log dir:  {}", dir.display()),
        Err(e) => println!("  Log dir:  unavailable ({e})"),
    }
    println!();

    // Recent sessions
    if report.recent_sessions.is_empty() {
        println!("Sessions: (none)");
    } else {
        println!("Recent sessions ({}):", report.recent_sessions.len());
        for s in &report.recent_sessions {
            let status = match &s.stop_reason {
                Some(reason) => format!("stopped ({reason})"),
                None => "running".to_string(),
            };
            println!(
                "  {} | PID {} | v{} | {} | {}",
                s.started_at, s.pid, s.version, s.mode, status
            );
        }
    }
    println!();

    // Latest metrics
    if let Some(ref m) = report.latest_metrics {
        let hours = m.uptime_secs / 3600;
        let mins = (m.uptime_secs % 3600) / 60;
        let secs = m.uptime_secs % 60;
        println!("Latest metrics:");
        println!("  Uptime:       {hours}h {mins}m {secs}s");
        println!("  Requests:     {}", m.total_requests);
        println!("  Connections:  {}", m.active_connections);
        println!("  Recorded at:  {}", m.timestamp);
    } else {
        println!("Latest metrics: (none)");
    }
    println!();

    // Error counts by category (last 24h)
    if report.error_counts_by_category.is_empty() {
        println!("Errors (last 24h): (none)");
    } else {
        println!("Errors by category (last 24h):");
        for c in &report.error_counts_by_category {
            println!("  {}: {}", c.category, c.count);
        }
    }
    println!();

    // Recent errors
    if report.recent_errors.is_empty() {
        println!("Recent errors/warnings: (none)");
    } else {
        println!("Recent errors/warnings ({}):", report.recent_errors.len());
        for e in &report.recent_errors {
            println!(
                "  [{}] {} | {} | {}",
                e.timestamp, e.severity, e.category, e.message
            );
        }
    }

    Ok(())
}
