use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::signal;
use tracing::{error, info};

/// Request from shell client containing command buffer and cursor position
#[derive(Debug, Serialize, Deserialize)]
pub struct CompletionRequest {
    /// Current command buffer text
    pub buffer: String,
    /// Cursor position in the buffer
    pub cursor: usize,
    /// Protocol version for future compatibility
    #[serde(default = "default_version")]
    pub version: u8,
}

fn default_version() -> u8 {
    1
}

/// Individual completion suggestion
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Suggestion {
    /// Text to insert
    pub text: String,
    /// Description of what this completion does
    pub description: String,
}

/// Response sent back to shell client
#[derive(Debug, Serialize, Deserialize)]
pub struct CompletionResponse {
    /// List of suggestions
    pub suggestions: Vec<Suggestion>,
}

/// Error response sent when request fails
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    /// Error message
    pub error: String,
}

pub async fn start(socket_path: &str) -> Result<()> {
    // Remove existing socket if it exists
    let _ = std::fs::remove_file(socket_path);

    let listener = UnixListener::bind(socket_path)
        .context(format!("Failed to bind to socket: {}", socket_path))?;
    info!("Daemon listening on {}", socket_path);

    // Set up graceful shutdown
    let shutdown = signal::ctrl_c();
    tokio::pin!(shutdown);

    loop {
        tokio::select! {
            // Handle new connections
            result = listener.accept() => {
                match result {
                    Ok((stream, _addr)) => {
                        tokio::spawn(async move {
                            if let Err(e) = handle_connection(stream).await {
                                error!("Connection error: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        error!("Failed to accept connection: {}", e);
                    }
                }
            }
            // Handle shutdown signal
            _ = &mut shutdown => {
                info!("Received shutdown signal, cleaning up...");
                break;
            }
        }
    }

    // Cleanup socket file
    let _ = std::fs::remove_file(socket_path);
    info!("Daemon shut down gracefully");

    Ok(())
}

async fn handle_connection(stream: UnixStream) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    // Read request (one line of JSON)
    reader
        .read_line(&mut line)
        .await
        .context("Failed to read request")?;

    // Parse request
    let request: CompletionRequest = match serde_json::from_str(&line) {
        Ok(req) => req,
        Err(e) => {
            // Send error response for malformed JSON
            let error_response = ErrorResponse {
                error: format!("Invalid JSON: {}", e),
            };
            let response = serde_json::to_string(&error_response)?;
            writer.write_all(response.as_bytes()).await?;
            writer.write_all(b"\n").await?;
            return Ok(());
        }
    };

    info!(
        "Received request: buffer='{}', cursor={}",
        request.buffer, request.cursor
    );

    // Generate suggestions (hardcoded for now, will be implemented in MVP parser phase)
    let suggestions = generate_suggestions(&request);

    // Send response
    let response = CompletionResponse { suggestions };
    let response_json = serde_json::to_string(&response)?;
    writer.write_all(response_json.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;

    Ok(())
}

/// Generate completion suggestions for a request
/// TODO: This is a placeholder - real implementation in Phase 1B (MVP Parser)
fn generate_suggestions(request: &CompletionRequest) -> Vec<Suggestion> {
    // For now, return empty suggestions
    // This will be replaced with actual parser logic in implement-mvp-parser
    let _ = request;
    Vec::new()
}
