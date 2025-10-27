use anyhow::Result;
use clap::{Parser, Subcommand};

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
    /// Get completion suggestions for a command buffer
    Complete {
        /// Command buffer to complete
        buffer: String,
        /// Cursor position in the buffer
        #[arg(short, long)]
        cursor: usize,
    },
    /// Install shell integration
    Install {
        /// Shell to install for (zsh, bash, fish)
        shell: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Daemon { socket } => {
            tracing::info!("Starting autocomplete daemon on {}", socket);
            daemon::start(&socket).await?;
        }
        Commands::Complete { buffer, cursor } => {
            // TODO: Implement completion logic
            println!("Completing: {} at position {}", buffer, cursor);
        }
        Commands::Install { shell } => {
            // TODO: Implement shell integration installation
            println!("Installing for shell: {}", shell);
        }
    }

    Ok(())
}
