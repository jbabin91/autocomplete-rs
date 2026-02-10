//! Context analysis for shell command completion.
//!
//! Walks the tokenized buffer to determine what kind of completion the user
//! needs: a command, subcommand, option, argument, or filename.

use super::tokenizer::{TokenKind, TokenizeResult};

/// What the user is completing at the cursor position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionContext {
    /// Completing a command name (first word in a pipeline segment).
    Command,
    /// Completing a subcommand (second word after the command).
    Subcommand { command: String },
    /// Completing an option (current token starts with `-`).
    Option { command: String },
    /// Completing a positional argument.
    Argument { command: String, position: usize },
    /// Completing a filename (after a redirection operator).
    Filename,
}

/// Analyze the tokenized buffer to determine completion context.
///
/// Walks backward from the cursor to find the most recent chain operator
/// (`|`, `&&`, `||`, `;`), which defines the active pipeline segment.
/// Then counts `Word` tokens in that segment to determine context.
pub fn analyze_context(result: &TokenizeResult) -> CompletionContext {
    let tokens = &result.tokens;

    if tokens.is_empty() {
        return CompletionContext::Command;
    }

    // Find the start of the active segment (after the last chain operator)
    let segment_start = find_segment_start(tokens, result.cursor_token_index);

    // Check if the previous token is a redirection operator
    let prev_token_idx = if result.at_word_boundary {
        tokens.len().checked_sub(1)
    } else {
        result.cursor_token_index.and_then(|i| i.checked_sub(1))
    };

    if let Some(idx) = prev_token_idx {
        let prev = &tokens[idx];
        if prev.kind == TokenKind::Operator && is_redirect(&prev.text) {
            return CompletionContext::Filename;
        }
    }

    // Check if the current token (being edited) starts with `-`
    if !result.at_word_boundary
        && let Some(idx) = result.cursor_token_index
        && result.prefix.starts_with('-')
    {
        let command = find_command(tokens, segment_start, idx);
        return CompletionContext::Option {
            command: command.unwrap_or_default(),
        };
    }

    // Count Word tokens in the active segment before cursor
    let word_count = count_words_before_cursor(tokens, segment_start, result);

    match word_count {
        0 => CompletionContext::Command,
        1 => {
            let command = find_command(tokens, segment_start, tokens.len());
            CompletionContext::Subcommand {
                command: command.unwrap_or_default(),
            }
        }
        n => {
            let command = find_command(tokens, segment_start, tokens.len());
            CompletionContext::Argument {
                command: command.unwrap_or_default(),
                position: n - 1,
            }
        }
    }
}

/// Find the index where the active pipeline segment starts.
/// Scans backward from the cursor (or end of tokens) for chain operators.
fn find_segment_start(tokens: &[super::tokenizer::Token], cursor_idx: Option<usize>) -> usize {
    let search_end = cursor_idx.map_or(tokens.len(), |i| i + 1);

    for i in (0..search_end).rev() {
        if tokens[i].kind == TokenKind::Operator && is_chain_operator(&tokens[i].text) {
            return i + 1;
        }
    }
    0
}

/// Check if an operator is a chain/pipe operator that starts a new command.
fn is_chain_operator(op: &str) -> bool {
    matches!(op, "|" | "||" | "&&" | ";" | "|&")
}

/// Check if an operator is a redirection operator.
fn is_redirect(op: &str) -> bool {
    matches!(op, "<" | ">" | ">>")
}

/// Find the command name (first Word token) in the active segment.
fn find_command(
    tokens: &[super::tokenizer::Token],
    segment_start: usize,
    before: usize,
) -> Option<String> {
    tokens[segment_start..before]
        .iter()
        .find(|t| t.kind == TokenKind::Word)
        .map(|t| t.text.clone())
}

/// Count Word tokens before the cursor in the active segment.
fn count_words_before_cursor(
    tokens: &[super::tokenizer::Token],
    segment_start: usize,
    result: &TokenizeResult,
) -> usize {
    let end = if result.at_word_boundary {
        tokens.len()
    } else {
        result.cursor_token_index.unwrap_or(tokens.len())
    };

    tokens[segment_start..end]
        .iter()
        .filter(|t| t.kind == TokenKind::Word)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::tokenizer::tokenize;

    #[test]
    fn empty_buffer_is_command() {
        let result = tokenize("", 0);
        assert_eq!(analyze_context(&result), CompletionContext::Command);
    }

    #[test]
    fn partial_first_word_is_command() {
        // Typing "gi" — still completing the command name
        // This tests cursor inside first word, no preceding words
        let result = tokenize("gi", 2);
        // cursor is inside first word, prefix starts with "gi" (not "-")
        // word_count before cursor = 0 (cursor is ON the first word, not after it)
        assert_eq!(analyze_context(&result), CompletionContext::Command);
    }

    #[test]
    fn command_with_trailing_space() {
        let result = tokenize("git ", 4);
        assert_eq!(
            analyze_context(&result),
            CompletionContext::Subcommand {
                command: "git".into()
            }
        );
    }

    #[test]
    fn subcommand_position() {
        let result = tokenize("git checkout ", 13);
        assert_eq!(
            analyze_context(&result),
            CompletionContext::Argument {
                command: "git".into(),
                position: 1,
            }
        );
    }

    #[test]
    fn option_with_dash() {
        let result = tokenize("git -", 5);
        assert_eq!(
            analyze_context(&result),
            CompletionContext::Option {
                command: "git".into()
            }
        );
    }

    #[test]
    fn option_with_double_dash() {
        let result = tokenize("git --ver", 9);
        assert_eq!(
            analyze_context(&result),
            CompletionContext::Option {
                command: "git".into()
            }
        );
    }

    #[test]
    fn pipe_resets_to_subcommand() {
        let result = tokenize("ls | grep ", 10);
        assert_eq!(
            analyze_context(&result),
            CompletionContext::Subcommand {
                command: "grep".into()
            }
        );
    }

    #[test]
    fn pipe_command_position() {
        let result = tokenize("ls | ", 5);
        assert_eq!(analyze_context(&result), CompletionContext::Command);
    }

    #[test]
    fn and_chain_resets_context() {
        let result = tokenize("make && ", 8);
        assert_eq!(analyze_context(&result), CompletionContext::Command);
    }

    #[test]
    fn semicolon_resets_context() {
        let result = tokenize("echo done; ", 11);
        assert_eq!(analyze_context(&result), CompletionContext::Command);
    }

    #[test]
    fn redirect_output() {
        let result = tokenize("echo hello > ", 13);
        assert_eq!(analyze_context(&result), CompletionContext::Filename);
    }

    #[test]
    fn redirect_input() {
        let result = tokenize("cmd < ", 6);
        assert_eq!(analyze_context(&result), CompletionContext::Filename);
    }

    #[test]
    fn redirect_append() {
        let result = tokenize("echo log >> ", 12);
        assert_eq!(analyze_context(&result), CompletionContext::Filename);
    }

    #[test]
    fn argument_after_subcommand() {
        let result = tokenize("git checkout feature ", 20);
        assert_eq!(
            analyze_context(&result),
            CompletionContext::Argument {
                command: "git".into(),
                position: 2,
            }
        );
    }

    #[test]
    fn complex_pipeline() {
        let result = tokenize("cat file | grep -i pattern | sort ", 34);
        assert_eq!(
            analyze_context(&result),
            CompletionContext::Subcommand {
                command: "sort".into()
            }
        );
    }
}
