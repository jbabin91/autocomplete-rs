//! `ParserEngine` — real completion engine backed by the tokenizer and context analyzer.
//!
//! MVP: parses the buffer, classifies the completion context, logs it,
//! and returns empty suggestions. Spec-based suggestion generation is
//! the next phase.

use tracing::debug;

use crate::engine::CompletionEngine;
use crate::protocol::{CompletionRequest, CompletionResponse};

use super::context::analyze_context;
use super::tokenizer::tokenize;

/// A completion engine that parses shell buffers and classifies context.
///
/// Stateless and inherently `Send + Sync`.
pub struct ParserEngine;

impl ParserEngine {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ParserEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl CompletionEngine for ParserEngine {
    fn complete(&self, request: &CompletionRequest) -> CompletionResponse {
        let result = tokenize(&request.buffer, request.cursor);
        let context = analyze_context(&result);

        debug!(
            buffer = %request.buffer,
            cursor = request.cursor,
            ?context,
            token_count = result.tokens.len(),
            prefix = %result.prefix,
            "parsed completion request"
        );

        // MVP: return empty suggestions. Spec-based generation is next phase.
        CompletionResponse {
            suggestions: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    #[test]
    fn parser_engine_returns_empty_suggestions() {
        let engine = ParserEngine::new();
        let req = CompletionRequest {
            buffer: "git checkout ".into(),
            cursor: 13,
            version: 1,
        };
        let resp = engine.complete(&req);
        assert!(resp.suggestions.is_empty());
    }

    #[test]
    fn parser_engine_is_send_sync() {
        let engine: Arc<dyn CompletionEngine> = Arc::new(ParserEngine::new());
        let req = CompletionRequest {
            buffer: "ls -la".into(),
            cursor: 6,
            version: 1,
        };
        let resp = engine.complete(&req);
        assert!(resp.suggestions.is_empty());
    }

    #[test]
    fn parser_engine_handles_empty_buffer() {
        let engine = ParserEngine::new();
        let req = CompletionRequest {
            buffer: String::new(),
            cursor: 0,
            version: 1,
        };
        let resp = engine.complete(&req);
        assert!(resp.suggestions.is_empty());
    }
}
