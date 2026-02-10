//! FSM-based shell tokenizer with cursor tracking.
//!
//! Single-pass tokenizer that handles quoting, escaping, and shell operators.
//! Tracks cursor position to determine which token is being edited and what
//! prefix has been typed so far.

/// The kind of token produced by the tokenizer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    /// A regular word (command name, argument, option).
    Word,
    /// A shell operator (`|`, `&&`, `||`, `;`, `&`, `<`, `>`, `>>`).
    Operator,
}

/// A single token from the command buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// What kind of token this is.
    pub kind: TokenKind,
    /// The text content of the token.
    pub text: String,
    /// Byte offset where this token starts in the buffer.
    pub start: usize,
    /// Byte offset where this token ends (exclusive) in the buffer.
    pub end: usize,
    /// Whether this token has an unclosed quote (user is still typing).
    pub quote_open: bool,
}

/// Result of tokenizing a buffer with cursor tracking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenizeResult {
    /// All tokens found in the buffer.
    pub tokens: Vec<Token>,
    /// Index of the token the cursor is inside (if any).
    pub cursor_token_index: Option<usize>,
    /// Whether the cursor is at a word boundary (whitespace after a token).
    pub at_word_boundary: bool,
    /// The prefix of the current token up to the cursor position.
    pub prefix: String,
}

/// Internal FSM state for the tokenizer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Normal,
    SingleQuote,
    DoubleQuote,
    Escaped,
    EscapedInDouble,
}

/// Tokenize a shell command buffer with cursor position tracking.
///
/// Performs a single pass over the buffer, splitting into `Word` and `Operator`
/// tokens. Handles single quotes, double quotes, and backslash escaping.
/// Unclosed quotes produce tokens with `quote_open: true` (not an error — the
/// user is still typing).
pub fn tokenize(buffer: &str, cursor: usize) -> TokenizeResult {
    let mut tokens: Vec<Token> = Vec::new();
    let mut state = State::Normal;
    let mut current = String::new();
    let mut token_start: Option<usize> = None;
    let mut quote_open = false;

    let bytes = buffer.as_bytes();
    let len = buffer.len();
    let mut i = 0;

    while i < len {
        let b = bytes[i];

        // All shell metacharacters (whitespace, quotes, operators, backslash)
        // are ASCII. Non-ASCII bytes are always literal content, so we can
        // handle them as a single case that properly decodes the full
        // UTF-8 character.
        if !b.is_ascii() {
            if token_start.is_none() {
                token_start = Some(i);
            }
            let ch = buffer[i..].chars().next().unwrap();
            current.push(ch);
            i += ch.len_utf8();
            // Restore state after consuming an escaped non-ASCII char.
            // Escaped/EscapedInDouble expect exactly one char then transition.
            match state {
                State::Escaped => state = State::Normal,
                State::EscapedInDouble => state = State::DoubleQuote,
                _ => {}
            }
            continue;
        }

        let ch = b as char;

        match state {
            State::Normal => {
                if ch.is_ascii_whitespace() {
                    // Flush any accumulated word
                    if !current.is_empty() {
                        tokens.push(Token {
                            kind: TokenKind::Word,
                            text: std::mem::take(&mut current),
                            start: token_start.unwrap(),
                            end: i,
                            quote_open: false,
                        });
                        token_start = None;
                    }
                    i += 1;
                } else if ch == '\'' {
                    if token_start.is_none() {
                        token_start = Some(i);
                    }
                    state = State::SingleQuote;
                    quote_open = true;
                    i += 1;
                } else if ch == '"' {
                    if token_start.is_none() {
                        token_start = Some(i);
                    }
                    state = State::DoubleQuote;
                    quote_open = true;
                    i += 1;
                } else if ch == '\\' {
                    if token_start.is_none() {
                        token_start = Some(i);
                    }
                    state = State::Escaped;
                    i += 1;
                } else if is_operator_start(ch) {
                    // Flush any accumulated word first
                    if !current.is_empty() {
                        tokens.push(Token {
                            kind: TokenKind::Word,
                            text: std::mem::take(&mut current),
                            start: token_start.unwrap(),
                            end: i,
                            quote_open: false,
                        });
                        token_start = None;
                    }

                    let op_start = i;
                    let op = read_operator(bytes, &mut i, len);
                    tokens.push(Token {
                        kind: TokenKind::Operator,
                        text: op,
                        start: op_start,
                        end: i,
                        quote_open: false,
                    });
                } else {
                    if token_start.is_none() {
                        token_start = Some(i);
                    }
                    current.push(ch);
                    i += 1;
                }
            }
            State::SingleQuote => {
                if ch == '\'' {
                    state = State::Normal;
                    quote_open = false;
                    i += 1;
                } else {
                    current.push(ch);
                    i += 1;
                }
            }
            State::DoubleQuote => {
                if ch == '"' {
                    state = State::Normal;
                    quote_open = false;
                    i += 1;
                } else if ch == '\\' {
                    state = State::EscapedInDouble;
                    i += 1;
                } else {
                    current.push(ch);
                    i += 1;
                }
            }
            State::Escaped => {
                current.push(ch);
                state = State::Normal;
                i += 1;
            }
            State::EscapedInDouble => {
                current.push(ch);
                state = State::DoubleQuote;
                i += 1;
            }
        }
    }

    // Flush any remaining token
    if !current.is_empty() || token_start.is_some() {
        let is_open = matches!(
            state,
            State::SingleQuote | State::DoubleQuote | State::Escaped | State::EscapedInDouble
        );
        tokens.push(Token {
            kind: TokenKind::Word,
            text: std::mem::take(&mut current),
            start: token_start.unwrap_or(len),
            end: len,
            quote_open: is_open || quote_open,
        });
    }

    // Determine cursor position relative to tokens
    let cursor = cursor.min(len);
    let mut cursor_token_index = None;
    let mut at_word_boundary = true;
    let mut prefix = String::new();

    for (idx, token) in tokens.iter().enumerate() {
        // Token occupies bytes [start, end). Cursor is "inside" the token if
        // it's strictly within (start, end), or at end only when end == buffer
        // length (user is typing at the very end of the buffer).
        let in_token =
            cursor > token.start && (cursor < token.end || (cursor == token.end && cursor == len));
        if in_token {
            cursor_token_index = Some(idx);
            at_word_boundary = false;
            // Compute the prefix: re-tokenize just the portion up to cursor
            let partial = &buffer[token.start..cursor];
            prefix = extract_prefix(partial);
            break;
        }
        if cursor == token.start {
            // Cursor is right at the start of this token — treat as boundary
            at_word_boundary = true;
            break;
        }
    }

    // If no token matched the cursor, it's at a word boundary (before, between,
    // or after tokens — all are boundaries where a new word could start).
    if cursor_token_index.is_none() {
        at_word_boundary = true;
    }

    TokenizeResult {
        tokens,
        cursor_token_index,
        at_word_boundary,
        prefix,
    }
}

/// Check if a character starts a shell operator.
fn is_operator_start(ch: char) -> bool {
    matches!(ch, '|' | '&' | ';' | '<' | '>')
}

/// Read a full operator (with one-char lookahead for multi-char operators).
/// Advances `i` past the operator.
fn read_operator(bytes: &[u8], i: &mut usize, len: usize) -> String {
    let ch = bytes[*i] as char;
    *i += 1;

    match ch {
        '|' => {
            if *i < len {
                let next = bytes[*i] as char;
                if next == '|' {
                    *i += 1;
                    return "||".to_string();
                }
                if next == '&' {
                    *i += 1;
                    return "|&".to_string();
                }
            }
            "|".to_string()
        }
        '&' => {
            if *i < len && bytes[*i] as char == '&' {
                *i += 1;
                return "&&".to_string();
            }
            "&".to_string()
        }
        '>' => {
            if *i < len && bytes[*i] as char == '>' {
                *i += 1;
                return ">>".to_string();
            }
            ">".to_string()
        }
        _ => ch.to_string(), // ';', '<'
    }
}

/// Extract the user-typed prefix from a partial token slice, stripping quotes.
fn extract_prefix(partial: &str) -> String {
    let mut result = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    for ch in partial.chars() {
        if escaped {
            result.push(ch);
            escaped = false;
            continue;
        }
        match ch {
            '\\' if !in_single => escaped = true,
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            _ => result.push(ch),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_words() {
        let result = tokenize("git commit -m hello", 19);
        let texts: Vec<&str> = result.tokens.iter().map(|t| t.text.as_str()).collect();
        assert_eq!(texts, ["git", "commit", "-m", "hello"]);
        assert!(result.tokens.iter().all(|t| t.kind == TokenKind::Word));
        assert!(result.tokens.iter().all(|t| !t.quote_open));
    }

    #[test]
    fn single_quotes() {
        let result = tokenize("echo 'hello world'", 18);
        assert_eq!(result.tokens.len(), 2);
        assert_eq!(result.tokens[1].text, "hello world");
        assert!(!result.tokens[1].quote_open);
    }

    #[test]
    fn double_quotes() {
        let result = tokenize("echo \"hello world\"", 18);
        assert_eq!(result.tokens.len(), 2);
        assert_eq!(result.tokens[1].text, "hello world");
        assert!(!result.tokens[1].quote_open);
    }

    #[test]
    fn backslash_escaping() {
        let result = tokenize("echo hello\\ world", 17);
        assert_eq!(result.tokens.len(), 2);
        assert_eq!(result.tokens[1].text, "hello world");
    }

    #[test]
    fn escape_in_double_quotes() {
        let result = tokenize(r#"echo "hello\"world""#, 19);
        assert_eq!(result.tokens.len(), 2);
        assert_eq!(result.tokens[1].text, "hello\"world");
    }

    #[test]
    fn operators() {
        let result = tokenize("ls | grep foo && echo done", 26);
        let kinds: Vec<&TokenKind> = result.tokens.iter().map(|t| &t.kind).collect();
        let texts: Vec<&str> = result.tokens.iter().map(|t| t.text.as_str()).collect();
        assert_eq!(texts, ["ls", "|", "grep", "foo", "&&", "echo", "done"]);
        assert_eq!(
            kinds,
            [
                &TokenKind::Word,
                &TokenKind::Operator,
                &TokenKind::Word,
                &TokenKind::Word,
                &TokenKind::Operator,
                &TokenKind::Word,
                &TokenKind::Word,
            ]
        );
    }

    #[test]
    fn multi_char_operators() {
        let result = tokenize("a || b && c >> d |& e", 21);
        let ops: Vec<&str> = result
            .tokens
            .iter()
            .filter(|t| t.kind == TokenKind::Operator)
            .map(|t| t.text.as_str())
            .collect();
        assert_eq!(ops, ["||", "&&", ">>", "|&"]);
    }

    #[test]
    fn semicolon_operator() {
        let result = tokenize("echo a; echo b", 14);
        assert_eq!(result.tokens[2].kind, TokenKind::Operator);
        assert_eq!(result.tokens[2].text, ";");
    }

    #[test]
    fn redirect_operators() {
        let result = tokenize("cat < input > output", 20);
        let ops: Vec<&str> = result
            .tokens
            .iter()
            .filter(|t| t.kind == TokenKind::Operator)
            .map(|t| t.text.as_str())
            .collect();
        assert_eq!(ops, ["<", ">"]);
    }

    #[test]
    fn unclosed_single_quote() {
        let result = tokenize("echo 'hello", 11);
        assert_eq!(result.tokens.len(), 2);
        assert!(result.tokens[1].quote_open);
        assert_eq!(result.tokens[1].text, "hello");
    }

    #[test]
    fn unclosed_double_quote() {
        let result = tokenize("echo \"hello", 11);
        assert_eq!(result.tokens.len(), 2);
        assert!(result.tokens[1].quote_open);
        assert_eq!(result.tokens[1].text, "hello");
    }

    #[test]
    fn cursor_inside_word() {
        let result = tokenize("git commit", 6);
        assert_eq!(result.cursor_token_index, Some(1));
        assert!(!result.at_word_boundary);
        assert_eq!(result.prefix, "co");
    }

    #[test]
    fn cursor_at_word_boundary() {
        let result = tokenize("git ", 4);
        assert!(result.at_word_boundary);
        assert_eq!(result.cursor_token_index, None);
        assert_eq!(result.prefix, "");
    }

    #[test]
    fn cursor_at_end_of_word() {
        let result = tokenize("git", 3);
        assert_eq!(result.cursor_token_index, Some(0));
        assert!(!result.at_word_boundary);
        assert_eq!(result.prefix, "git");
    }

    #[test]
    fn cursor_at_start() {
        let result = tokenize("git", 0);
        assert!(result.at_word_boundary);
        assert_eq!(result.cursor_token_index, None);
    }

    #[test]
    fn empty_buffer() {
        let result = tokenize("", 0);
        assert!(result.tokens.is_empty());
        assert!(result.at_word_boundary);
        assert_eq!(result.prefix, "");
    }

    #[test]
    fn only_whitespace() {
        let result = tokenize("   ", 3);
        assert!(result.tokens.is_empty());
        assert!(result.at_word_boundary);
    }

    #[test]
    fn multi_byte_utf8() {
        let result = tokenize("echo café", 10);
        assert_eq!(result.tokens.len(), 2);
        assert_eq!(result.tokens[1].text, "café");
    }

    #[test]
    fn escaped_non_ascii() {
        let result = tokenize("echo \\é next", 13);
        assert_eq!(result.tokens.len(), 3);
        assert_eq!(result.tokens[1].text, "é");
        assert_eq!(result.tokens[2].text, "next");
    }

    #[test]
    fn escaped_non_ascii_in_double_quotes() {
        let result = tokenize("echo \"\\éworld\"", 14);
        assert_eq!(result.tokens.len(), 2);
        assert_eq!(result.tokens[1].text, "éworld");
        assert!(!result.tokens[1].quote_open);
    }

    #[test]
    fn prefix_with_quotes() {
        let result = tokenize("echo \"hel", 9);
        assert_eq!(result.cursor_token_index, Some(1));
        assert_eq!(result.prefix, "hel");
    }

    #[test]
    fn adjacent_operators() {
        let result = tokenize("a|b", 3);
        assert_eq!(result.tokens.len(), 3);
        assert_eq!(result.tokens[0].text, "a");
        assert_eq!(result.tokens[1].text, "|");
        assert_eq!(result.tokens[2].text, "b");
    }

    #[test]
    fn token_positions() {
        let result = tokenize("git commit", 10);
        assert_eq!(result.tokens[0].start, 0);
        assert_eq!(result.tokens[0].end, 3);
        assert_eq!(result.tokens[1].start, 4);
        assert_eq!(result.tokens[1].end, 10);
    }

    #[test]
    fn background_operator() {
        let result = tokenize("sleep 10 &", 10);
        let ops: Vec<&str> = result
            .tokens
            .iter()
            .filter(|t| t.kind == TokenKind::Operator)
            .map(|t| t.text.as_str())
            .collect();
        assert_eq!(ops, ["&"]);
    }
}
