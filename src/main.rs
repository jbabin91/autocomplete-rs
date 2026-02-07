use std::time::Duration;

use anyhow::{Context, Result};
use autocomplete_rs::daemon;
use autocomplete_rs::protocol::{CompletionRequest, DaemonMessage, PROTOCOL_VERSION, ShutdownAck};
use clap::{Parser, Subcommand};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

const DEFAULT_SOCKET: &str = "/tmp/autocomplete-rs.sock";

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
        /// Unix socket path
        #[arg(short, long, default_value = DEFAULT_SOCKET, env = "AUTOCOMPLETE_RS_SOCKET")]
        socket: String,
    },
    /// Stop the running daemon
    Stop {
        /// Unix socket path
        #[arg(short, long, default_value = DEFAULT_SOCKET, env = "AUTOCOMPLETE_RS_SOCKET")]
        socket: String,
    },
    /// Check daemon status
    Status {
        /// Unix socket path
        #[arg(short, long, default_value = DEFAULT_SOCKET, env = "AUTOCOMPLETE_RS_SOCKET")]
        socket: String,
    },
    /// Get completion suggestions for a command buffer
    Complete {
        /// Command buffer to complete
        buffer: String,
        /// Cursor position in the buffer
        #[arg(short, long)]
        cursor: usize,
        /// Unix socket path
        #[arg(short, long, default_value = DEFAULT_SOCKET, env = "AUTOCOMPLETE_RS_SOCKET")]
        socket: String,
    },
    /// Install shell integration
    Install {
        /// Shell to install for (zsh, bash, fish)
        shell: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging only for the daemon subcommand
    if matches!(cli.command, Commands::Daemon { .. }) {
        autocomplete_rs::logging::init().context("failed to initialize logging")?;
    }

    match cli.command {
        Commands::Daemon { socket } => {
            tracing::info!("Starting autocomplete daemon on {}", socket);
            daemon::start(&socket).await?;
        }
        Commands::Stop { socket } => {
            stop_daemon(&socket).await?;
        }
        Commands::Status { socket } => {
            status_command(&socket).await?;
        }
        Commands::Complete {
            buffer,
            cursor,
            socket,
        } => {
            complete_command(&buffer, cursor, &socket).await?;
        }
        Commands::Install { shell } => {
            install_command(&shell)?;
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
    reader.read_line(&mut response_line).await?;

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
                Ok(Err(e)) => println!("Daemon stopped (read error: {e})"),
                Err(_) => println!("Daemon stop requested (timed out waiting for ack)"),
            }
        }
        Err(e) => {
            // Only remove the socket if the error indicates nothing is listening
            // (ConnectionRefused). Other errors (PermissionDenied, transient FS
            // errors) could mean a live daemon we can't reach — deleting its
            // socket would break it.
            if e.kind() == std::io::ErrorKind::ConnectionRefused {
                if let Err(rm_err) = std::fs::remove_file(socket_path) {
                    if rm_err.kind() != std::io::ErrorKind::NotFound {
                        return Err(rm_err)
                            .context(format!("failed to remove stale socket: {socket_path}"));
                    }
                }
                println!("Removed stale socket (daemon was not running)");
            } else {
                eprintln!("Could not connect to daemon: {e}");
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
