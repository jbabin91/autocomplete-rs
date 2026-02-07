use serde::{Deserialize, Serialize};

/// Maximum buffer length we accept (protects against abuse / accidental pastes).
pub const MAX_BUFFER_LEN: usize = 10_000;

/// Maximum raw request size in bytes (100 KB).
pub const MAX_REQUEST_SIZE: u64 = 100 * 1024;

/// Current protocol version.
pub const PROTOCOL_VERSION: u8 = 1;

// ── Request types ──────────────────────────────────────────────────

/// Request from shell client containing command buffer and cursor position.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct CompletionRequest {
    /// Current command buffer text.
    pub buffer: String,
    /// Cursor position in the buffer (byte offset).
    pub cursor: usize,
    /// Protocol version for future compatibility.
    #[serde(default = "default_version")]
    pub version: u8,
}

fn default_version() -> u8 {
    PROTOCOL_VERSION
}

/// Tagged envelope for daemon messages.
///
/// Shell integration sends bare `CompletionRequest` (no `"type"` field) for backward
/// compatibility. The handler tries `DaemonMessage` first, then falls back.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonMessage {
    /// A completion request wrapped in the envelope.
    Complete(CompletionRequest),
    /// Request the daemon to shut down gracefully.
    Shutdown,
}

// ── Response types ─────────────────────────────────────────────────

/// Individual completion suggestion.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Suggestion {
    /// Text to insert.
    pub text: String,
    /// Description of what this completion does.
    pub description: String,
}

/// Response sent back to shell client.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct CompletionResponse {
    /// List of suggestions.
    pub suggestions: Vec<Suggestion>,
}

/// Error response sent when a request fails.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ErrorResponse {
    /// Error message.
    pub error: String,
}

/// Acknowledgement sent in response to a `Shutdown` message.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ShutdownAck {
    pub status: String,
}

// ── Validation ─────────────────────────────────────────────────────

/// Validation error for incoming requests.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ValidationError {
    #[error("buffer exceeds maximum length ({len} > {MAX_BUFFER_LEN})")]
    BufferTooLong { len: usize },

    #[error("cursor position {cursor} exceeds buffer length {buffer_len}")]
    CursorOutOfBounds { cursor: usize, buffer_len: usize },
}

/// Validate a `CompletionRequest` before processing.
pub fn validate_request(req: &CompletionRequest) -> Result<(), ValidationError> {
    if req.buffer.len() > MAX_BUFFER_LEN {
        return Err(ValidationError::BufferTooLong {
            len: req.buffer.len(),
        });
    }
    if req.cursor > req.buffer.len() {
        return Err(ValidationError::CursorOutOfBounds {
            cursor: req.cursor,
            buffer_len: req.buffer.len(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Serde round-trips ──────────────────────────────────────────

    #[test]
    fn completion_request_round_trip() {
        let req = CompletionRequest {
            buffer: "git comm".into(),
            cursor: 8,
            version: 1,
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: CompletionRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req, parsed);
    }

    #[test]
    fn completion_request_default_version() {
        let json = r#"{"buffer":"ls","cursor":2}"#;
        let req: CompletionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.version, PROTOCOL_VERSION);
    }

    #[test]
    fn completion_response_round_trip() {
        let resp = CompletionResponse {
            suggestions: vec![Suggestion {
                text: "commit".into(),
                description: "Record changes".into(),
            }],
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: CompletionResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(resp, parsed);
    }

    #[test]
    fn error_response_round_trip() {
        let err = ErrorResponse {
            error: "bad request".into(),
        };
        let json = serde_json::to_string(&err).unwrap();
        let parsed: ErrorResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(err, parsed);
    }

    // ── DaemonMessage variants ─────────────────────────────────────

    #[test]
    fn daemon_message_complete_variant() {
        let msg = DaemonMessage::Complete(CompletionRequest {
            buffer: "ls".into(),
            cursor: 2,
            version: 1,
        });
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"complete""#));
        let parsed: DaemonMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, parsed);
    }

    #[test]
    fn daemon_message_shutdown_variant() {
        let json = r#"{"type":"shutdown"}"#;
        let parsed: DaemonMessage = serde_json::from_str(json).unwrap();
        assert_eq!(parsed, DaemonMessage::Shutdown);
    }

    #[test]
    fn bare_request_does_not_parse_as_daemon_message() {
        // Bare CompletionRequest has no "type" field → must fail DaemonMessage parse
        let json = r#"{"buffer":"git","cursor":3}"#;
        assert!(serde_json::from_str::<DaemonMessage>(json).is_err());
    }

    // ── Validation ─────────────────────────────────────────────────

    #[test]
    fn validate_good_request() {
        let req = CompletionRequest {
            buffer: "git commit".into(),
            cursor: 10,
            version: 1,
        };
        assert!(validate_request(&req).is_ok());
    }

    #[test]
    fn validate_cursor_at_zero() {
        let req = CompletionRequest {
            buffer: "ls".into(),
            cursor: 0,
            version: 1,
        };
        assert!(validate_request(&req).is_ok());
    }

    #[test]
    fn validate_empty_buffer_cursor_zero() {
        let req = CompletionRequest {
            buffer: String::new(),
            cursor: 0,
            version: 1,
        };
        assert!(validate_request(&req).is_ok());
    }

    #[test]
    fn validate_cursor_out_of_bounds() {
        let req = CompletionRequest {
            buffer: "ls".into(),
            cursor: 5,
            version: 1,
        };
        assert_eq!(
            validate_request(&req),
            Err(ValidationError::CursorOutOfBounds {
                cursor: 5,
                buffer_len: 2,
            })
        );
    }

    #[test]
    fn validate_buffer_too_long() {
        let req = CompletionRequest {
            buffer: "x".repeat(MAX_BUFFER_LEN + 1),
            cursor: 0,
            version: 1,
        };
        assert_eq!(
            validate_request(&req),
            Err(ValidationError::BufferTooLong {
                len: MAX_BUFFER_LEN + 1,
            })
        );
    }

    #[test]
    fn validate_buffer_exactly_at_max() {
        let req = CompletionRequest {
            buffer: "x".repeat(MAX_BUFFER_LEN),
            cursor: 0,
            version: 1,
        };
        assert!(validate_request(&req).is_ok());
    }
}
