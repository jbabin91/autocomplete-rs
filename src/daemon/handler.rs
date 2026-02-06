use std::time::Duration;

use anyhow::Result;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::time::timeout;
use tracing::{debug, instrument, warn};

use crate::protocol::{
    CompletionRequest, DaemonMessage, ErrorResponse, MAX_REQUEST_SIZE, ShutdownAck,
    validate_request,
};

use super::state::DaemonState;

/// Timeout for reading a request from a client.
const READ_TIMEOUT: Duration = Duration::from_secs(1);

/// Timeout for writing a response to a client.
const WRITE_TIMEOUT: Duration = Duration::from_secs(1);

/// Handle a single connection.
///
/// Reads one request, processes it, writes one response, then closes.
#[instrument(skip_all, fields(conn_id = %conn_id))]
pub async fn handle_connection<R, W>(
    reader: R,
    mut writer: W,
    state: &DaemonState,
    conn_id: u64,
) -> Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let _guard = state.connection_guard();

    // Limit read size to prevent abuse
    let limited = reader.take(MAX_REQUEST_SIZE);
    let mut buf_reader = BufReader::new(limited);
    let mut line = String::new();

    // Read request with timeout
    let read_result = timeout(READ_TIMEOUT, buf_reader.read_line(&mut line)).await;

    let bytes_read = match read_result {
        Ok(Ok(n)) => n,
        Ok(Err(e)) => {
            debug!("read error: {}", e);
            return Ok(());
        }
        Err(_) => {
            debug!("read timeout");
            let err = ErrorResponse {
                error: "request timed out".into(),
            };
            write_json(&mut writer, &err).await?;
            return Ok(());
        }
    };

    // Empty read = client disconnected
    if bytes_read == 0 {
        debug!("client disconnected (empty read)");
        return Ok(());
    }

    state.record_request();

    // Try to parse as DaemonMessage (envelope with "type" field) first,
    // then fall back to bare CompletionRequest for backward compatibility.
    let request = match parse_message(&line) {
        ParsedMessage::Complete(req) => req,
        ParsedMessage::Shutdown => {
            debug!("received shutdown request");
            let ack = ShutdownAck {
                status: "shutting_down".into(),
            };
            write_json(&mut writer, &ack).await?;
            state.cancel.cancel();
            return Ok(());
        }
        ParsedMessage::Error(msg) => {
            warn!("malformed request: {}", msg);
            let err = ErrorResponse { error: msg };
            write_json(&mut writer, &err).await?;
            return Ok(());
        }
    };

    // Validate the request
    if let Err(e) = validate_request(&request) {
        let err = ErrorResponse {
            error: e.to_string(),
        };
        write_json(&mut writer, &err).await?;
        return Ok(());
    }

    // Generate completions
    let response = state.engine.complete(&request);
    write_json(&mut writer, &response).await?;

    Ok(())
}

/// Result of parsing a raw JSON line.
enum ParsedMessage {
    Complete(CompletionRequest),
    Shutdown,
    Error(String),
}

/// Parse a JSON line, trying `DaemonMessage` first, then bare `CompletionRequest`.
///
/// If the JSON contains a `"type"` field but fails to parse as `DaemonMessage`,
/// we return an error instead of falling back — this prevents masking protocol bugs
/// (e.g. a misspelled type value being silently treated as a completion request).
fn parse_message(line: &str) -> ParsedMessage {
    // Try envelope format first
    if let Ok(msg) = serde_json::from_str::<DaemonMessage>(line) {
        return match msg {
            DaemonMessage::Complete(req) => ParsedMessage::Complete(req),
            DaemonMessage::Shutdown => ParsedMessage::Shutdown,
        };
    }

    // If the JSON has a "type" field, it was intended as a DaemonMessage but had an
    // unrecognized type value — don't silently fall back to CompletionRequest.
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
        if value.get("type").is_some() {
            return ParsedMessage::Error(format!("unknown message type: {}", value["type"]));
        }
    }

    // Fall back to bare CompletionRequest (backward compat — no "type" field)
    match serde_json::from_str::<CompletionRequest>(line) {
        Ok(req) => ParsedMessage::Complete(req),
        Err(e) => ParsedMessage::Error(format!("invalid JSON: {e}")),
    }
}

/// Serialize a value as JSON and write it as a newline-terminated line with timeout.
async fn write_json<W, T>(writer: &mut W, value: &T) -> Result<()>
where
    W: AsyncWrite + Unpin,
    T: serde::Serialize,
{
    let json = serde_json::to_string(value)?;
    timeout(WRITE_TIMEOUT, async {
        writer.write_all(json.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
        Ok::<(), anyhow::Error>(())
    })
    .await
    .map_err(|_| anyhow::anyhow!("write timed out"))??;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tokio::net::UnixStream;

    use super::*;
    use crate::engine::StubEngine;
    use crate::protocol::CompletionResponse;

    fn make_state() -> DaemonState {
        DaemonState::new(Arc::new(StubEngine))
    }

    /// Helper: send a request line and return the response line.
    async fn roundtrip(request_line: &str, state: &DaemonState) -> String {
        let (client, server) = UnixStream::pair().unwrap();
        let (sr, sw) = server.into_split();

        // Spawn handler
        let state_clone = state.clone();
        let handle = tokio::spawn(async move {
            handle_connection(sr, sw, &state_clone, 1).await.unwrap();
        });

        let (cr, mut cw) = client.into_split();

        // Send request
        cw.write_all(request_line.as_bytes()).await.unwrap();
        cw.write_all(b"\n").await.unwrap();
        // Signal we're done writing
        drop(cw);

        // Read response
        let mut reader = BufReader::new(cr);
        let mut response = String::new();
        reader.read_line(&mut response).await.unwrap();

        handle.await.unwrap();
        response.trim().to_string()
    }

    #[tokio::test]
    async fn valid_bare_request() {
        let state = make_state();
        let resp = roundtrip(r#"{"buffer":"git ","cursor":4}"#, &state).await;
        let parsed: CompletionResponse = serde_json::from_str(&resp).unwrap();
        assert!(parsed.suggestions.is_empty());
    }

    #[tokio::test]
    async fn valid_envelope_request() {
        let state = make_state();
        let resp = roundtrip(r#"{"type":"complete","buffer":"git ","cursor":4}"#, &state).await;
        let parsed: CompletionResponse = serde_json::from_str(&resp).unwrap();
        assert!(parsed.suggestions.is_empty());
    }

    #[tokio::test]
    async fn malformed_json() {
        let state = make_state();
        let resp = roundtrip("not json at all", &state).await;
        let parsed: ErrorResponse = serde_json::from_str(&resp).unwrap();
        assert!(parsed.error.contains("invalid JSON"));
    }

    #[tokio::test]
    async fn cursor_out_of_bounds() {
        let state = make_state();
        let resp = roundtrip(r#"{"buffer":"ls","cursor":99}"#, &state).await;
        let parsed: ErrorResponse = serde_json::from_str(&resp).unwrap();
        assert!(parsed.error.contains("cursor position"));
    }

    #[tokio::test]
    async fn buffer_too_long() {
        let state = make_state();
        let long_buf = "x".repeat(11_000);
        let req = format!(r#"{{"buffer":"{}","cursor":0}}"#, long_buf);
        let resp = roundtrip(&req, &state).await;
        let parsed: ErrorResponse = serde_json::from_str(&resp).unwrap();
        assert!(parsed.error.contains("buffer exceeds maximum length"));
    }

    #[tokio::test]
    async fn empty_disconnect() {
        let state = make_state();
        let (client, server) = UnixStream::pair().unwrap();
        let (sr, sw) = server.into_split();

        // Close immediately without sending anything
        drop(client);

        handle_connection(sr, sw, &state, 1).await.unwrap();
        // Should not panic or error
    }

    #[tokio::test]
    async fn shutdown_message() {
        let state = make_state();
        let resp = roundtrip(r#"{"type":"shutdown"}"#, &state).await;
        let parsed: ShutdownAck = serde_json::from_str(&resp).unwrap();
        assert_eq!(parsed.status, "shutting_down");
        assert!(state.cancel.is_cancelled());
    }

    #[tokio::test]
    async fn unknown_message_type_rejected() {
        let state = make_state();
        let resp = roundtrip(r#"{"type":"unknown","buffer":"ls","cursor":2}"#, &state).await;
        let parsed: ErrorResponse = serde_json::from_str(&resp).unwrap();
        assert!(parsed.error.contains("unknown message type"));
    }

    #[tokio::test]
    async fn connection_guard_tracks_active() {
        let state = make_state();
        let (client, server) = UnixStream::pair().unwrap();
        let (sr, sw) = server.into_split();
        let (cr, mut cw) = client.into_split();

        let state2 = state.clone();
        let handle = tokio::spawn(async move {
            handle_connection(sr, sw, &state2, 1).await.unwrap();
        });

        cw.write_all(b"{\"buffer\":\"ls\",\"cursor\":2}\n")
            .await
            .unwrap();
        drop(cw);

        // Read the response to prevent broken pipe
        let mut reader = BufReader::new(cr);
        let mut resp = String::new();
        reader.read_line(&mut resp).await.unwrap();

        handle.await.unwrap();
        // After handler finishes, active connections should be back to 0
        assert_eq!(
            state
                .active_connections
                .load(std::sync::atomic::Ordering::Relaxed),
            0
        );
    }
}
