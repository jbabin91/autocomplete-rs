use std::sync::Arc;

use autocomplete_rs::engine::CompletionEngine;
use autocomplete_rs::parser::tokenizer::tokenize;
use autocomplete_rs::parser::{CompletionContext, ParserEngine};
use autocomplete_rs::protocol::{CompletionRequest, PROTOCOL_VERSION};

#[test]
fn parser_engine_is_send_sync_behind_arc() {
    // Compile-time check: ParserEngine must work as Arc<dyn CompletionEngine>
    let engine: Arc<dyn CompletionEngine> = Arc::new(ParserEngine::new());
    let req = CompletionRequest {
        buffer: "git ".into(),
        cursor: 4,
        version: PROTOCOL_VERSION,
    };
    let resp = engine.complete(&req);
    assert!(resp.suggestions.is_empty());
}

#[test]
fn various_buffers_dont_panic() {
    let engine = ParserEngine::new();
    let buffers = [
        ("", 0),
        ("ls", 2),
        ("git ", 4),
        ("git checkout feature", 20),
        ("echo 'hello world'", 18),
        ("echo \"hello world\"", 18),
        ("ls | grep foo && echo done", 26),
        ("cmd > output.txt", 16),
        ("cat < input.txt", 15),
        ("echo hello\\ world", 17),
        ("a || b && c; d | e", 18),
        ("   ", 3),
        ("git -", 5),
        ("git --version", 13),
    ];

    for (buffer, cursor) in buffers {
        let req = CompletionRequest {
            buffer: buffer.to_string(),
            cursor,
            version: PROTOCOL_VERSION,
        };
        let resp = engine.complete(&req);
        assert!(resp.suggestions.is_empty(), "buffer: {buffer:?}");
    }
}

#[test]
fn context_classification_command() {
    let result = tokenize("", 0);
    let ctx = autocomplete_rs::parser::context::analyze_context(&result);
    assert_eq!(ctx, CompletionContext::Command);
}

#[test]
fn context_classification_subcommand() {
    let result = tokenize("git ", 4);
    let ctx = autocomplete_rs::parser::context::analyze_context(&result);
    assert_eq!(
        ctx,
        CompletionContext::Subcommand {
            command: "git".into()
        }
    );
}

#[test]
fn context_classification_option() {
    let result = tokenize("git --ver", 9);
    let ctx = autocomplete_rs::parser::context::analyze_context(&result);
    assert_eq!(
        ctx,
        CompletionContext::Option {
            command: "git".into()
        }
    );
}

#[test]
fn context_classification_argument() {
    let result = tokenize("git checkout ", 13);
    let ctx = autocomplete_rs::parser::context::analyze_context(&result);
    assert_eq!(
        ctx,
        CompletionContext::Argument {
            command: "git".into(),
            position: 1,
        }
    );
}

#[test]
fn context_classification_filename() {
    let result = tokenize("cmd > ", 6);
    let ctx = autocomplete_rs::parser::context::analyze_context(&result);
    assert_eq!(ctx, CompletionContext::Filename);
}

#[test]
fn context_after_pipe() {
    let result = tokenize("ls | ", 5);
    let ctx = autocomplete_rs::parser::context::analyze_context(&result);
    assert_eq!(ctx, CompletionContext::Command);
}

#[test]
fn context_subcommand_after_pipe() {
    let result = tokenize("ls | grep ", 10);
    let ctx = autocomplete_rs::parser::context::analyze_context(&result);
    assert_eq!(
        ctx,
        CompletionContext::Subcommand {
            command: "grep".into()
        }
    );
}

#[test]
fn tokenizer_preserves_operator_positions() {
    let result = tokenize("a | b && c", 10);
    assert_eq!(result.tokens.len(), 5);

    // Check operator tokens
    assert_eq!(result.tokens[1].text, "|");
    assert_eq!(result.tokens[1].start, 2);
    assert_eq!(result.tokens[3].text, "&&");
    assert_eq!(result.tokens[3].start, 6);
}
