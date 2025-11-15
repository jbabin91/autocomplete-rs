use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

mod daemon;
mod parser;
mod tui;

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
        #[arg(short, long, default_value = "/tmp/autocomplete-rs.sock")]
        socket: String,
    },
    /// Stop the running daemon
    Stop {
        /// Unix socket path
        #[arg(short, long, default_value = "/tmp/autocomplete-rs.sock")]
        socket: String,
    },
    /// Check daemon status
    Status {
        /// Unix socket path
        #[arg(short, long, default_value = "/tmp/autocomplete-rs.sock")]
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
        #[arg(short, long, default_value = "/tmp/autocomplete-rs.sock")]
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
    // Initialize logging (only for daemon, suppress for complete command)
    if std::env::args().any(|arg| arg == "daemon") {
        tracing_subscriber::fmt::init();
    }

    let cli = Cli::parse();

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

/// Handle the complete command: connect to daemon, get suggestions, show TUI
async fn complete_command(buffer: &str, cursor: usize, socket_path: &str) -> Result<()> {
    // Connect to daemon
    let stream = UnixStream::connect(socket_path)
        .await
        .context("Failed to connect to daemon. Is it running?")?;

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Send request
    let request = daemon::CompletionRequest {
        buffer: buffer.to_string(),
        cursor,
        version: 1,
    };
    let request_json = serde_json::to_string(&request)?;
    writer.write_all(request_json.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;

    // Read response
    let mut response_line = String::new();
    reader.read_line(&mut response_line).await?;

    // Parse response
    let response: daemon::CompletionResponse =
        serde_json::from_str(&response_line).context("Failed to parse daemon response")?;

    // Show TUI with suggestions
    if !response.suggestions.is_empty() {
        let mut ui = tui::CompletionUI::new(response.suggestions);
        if let Some(selected) = ui.run()? {
            // Print selected completion to stdout for zsh to capture
            println!("{}", selected.text);
        }
    }

    Ok(())
}

/// Stop the running daemon
async fn stop_daemon(socket_path: &str) -> Result<()> {
    use std::path::Path;

    if !Path::new(socket_path).exists() {
        println!("Daemon is not running (socket not found)");
        return Ok(());
    }

    // Try to connect to send shutdown signal
    match UnixStream::connect(socket_path).await {
        Ok(_stream) => {
            // Connection successful means daemon is running
            // For now, we'll just remove the socket and let the daemon detect it
            // In a production system, you'd send a shutdown message
            std::fs::remove_file(socket_path)?;
            println!("Daemon stopped");
        }
        Err(_) => {
            // Can't connect, remove stale socket
            std::fs::remove_file(socket_path)?;
            println!("Removed stale socket (daemon was not running)");
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
        Err(_) => {
            println!("Socket exists but daemon is not responding (stale socket)");
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
