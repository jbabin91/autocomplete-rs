/// Shell buffer parser for the autocomplete engine.
///
/// Tokenizes command buffers, tracks cursor position, and classifies
/// what kind of completion the user needs. Sub-modules:
///
/// - `tokenizer` — FSM tokenizer with cursor tracking
/// - `context` — completion context analysis
/// - `engine` — `ParserEngine` implementing `CompletionEngine`
pub mod context;
pub mod engine;
pub mod tokenizer;

pub use context::CompletionContext;
pub use engine::ParserEngine;
pub use tokenizer::{Token, TokenKind, TokenizeResult};
