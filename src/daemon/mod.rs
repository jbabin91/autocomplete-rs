use anyhow::Result;
use tokio::net::UnixListener;
use tracing::info;

pub async fn start(socket_path: &str) -> Result<()> {
    // Remove existing socket if it exists
    let _ = std::fs::remove_file(socket_path);

    let listener = UnixListener::bind(socket_path)?;
    info!("Daemon listening on {}", socket_path);

    loop {
        let (stream, _addr) = listener.accept().await?;
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream).await {
                tracing::error!("Connection error: {}", e);
            }
        });
    }
}

async fn handle_connection(_stream: tokio::net::UnixStream) -> Result<()> {
    // TODO: Implement connection handling
    // 1. Read command buffer from client
    // 2. Parse and get suggestions
    // 3. Send back suggestions
    info!("Handling connection");
    Ok(())
}
