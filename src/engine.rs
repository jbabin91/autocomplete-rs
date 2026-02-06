use crate::protocol::{CompletionRequest, CompletionResponse};

/// Trait for completion backends.
///
/// Designed to be callable from either the daemon or a direct CLI path,
/// so the daemon-vs-single-process decision can be deferred.
pub trait CompletionEngine: Send + Sync {
    /// Generate completions for the given request.
    fn complete(&self, request: &CompletionRequest) -> CompletionResponse;
}

/// A stub engine that always returns empty suggestions.
///
/// Used during development until the real parser is wired in.
pub struct StubEngine;

impl CompletionEngine for StubEngine {
    fn complete(&self, _request: &CompletionRequest) -> CompletionResponse {
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
    fn stub_engine_returns_empty() {
        let engine = StubEngine;
        let req = CompletionRequest {
            buffer: "git ".into(),
            cursor: 4,
            version: 1,
        };
        let resp = engine.complete(&req);
        assert!(resp.suggestions.is_empty());
    }

    #[test]
    fn engine_trait_object_behind_arc() {
        let engine: Arc<dyn CompletionEngine> = Arc::new(StubEngine);
        let req = CompletionRequest {
            buffer: "ls -la".into(),
            cursor: 6,
            version: 1,
        };
        let resp = engine.complete(&req);
        assert!(resp.suggestions.is_empty());
    }
}
