# Parser Architecture

This document details the design and implementation of the command buffer
parser.

## Overview

The parser is responsible for:

- Tokenizing command buffers
- Analyzing completion context
- Matching against completion specs
- Generating context-aware suggestions
- Completing in <5ms

## Design Principles

1. **Fast:** <5ms parsing time, optimized for common cases
2. **Accurate:** Correctly identify context even with complex commands
3. **Robust:** Handle incomplete, malformed, or edge-case inputs
4. **Modular:** Clear separation between tokenization, analysis, matching
5. **Testable:** Pure functions, easy to unit test

## Architecture

### Component Diagram

```text
┌─────────────────────────────────────────────────────────────┐
│                   Parser (src/parser/)                       │
│                                                               │
│  ┌────────────────────────────────────────────────────────┐ │
│  │  mod.rs - Coordination                                 │ │
│  │  pub fn parse(buffer: &str, cursor: usize)            │ │
│  └────────────────────────────────────────────────────────┘ │
│                           │                                  │
│           ┌───────────────┼───────────────┐                 │
│           ▼               ▼               ▼                  │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐          │
│  │ Tokenizer   │ │   Context   │ │   Matcher   │          │
│  │ tokenizer.rs│ │ context.rs  │ │ matcher.rs  │          │
│  │             │ │             │ │             │          │
│  │ Split buffer│ │ Analyze     │ │ Match specs │          │
│  │ into tokens │ │ what user   │ │ Generate    │          │
│  │             │ │ is typing   │ │ suggestions │          │
│  └─────────────┘ └─────────────┘ └─────────────┘          │
│                           │                                  │
│                           ▼                                  │
│  ┌────────────────────────────────────────────────────────┐ │
│  │         Spec Loader (src/specs/mod.rs)                │ │
│  │  - Load completion specs                              │ │
│  │  - Cache frequently used specs                        │ │
│  └────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### Data Flow

```text
Input: buffer="git checkout -b ", cursor=18

    ↓ [Tokenizer]

Tokens: ["git", "checkout", "-b", ""]

    ↓ [Context Analyzer]

Context: {
  command: "git",
  subcommands: ["checkout"],
  options: ["-b"],
  cursor_token: "",
  cursor_position: AfterOption("-b"),
  expects: Argument(BranchName)
}

    ↓ [Spec Loader]

Spec: git.msgpack → CompletionSpec

    ↓ [Matcher]

Matches: [
  {text: "feature/new", desc: "Create feature branch"},
  {text: "main", desc: "Branch from main"}
]

    ↓ [Output]

Suggestions: Vec<Suggestion>
```

## Tokenization

### Algorithm

**Goal:** Split command buffer into logical tokens

**Rules:**

1. Split on whitespace
2. Preserve quotes (single, double)
3. Handle escaped characters
4. Handle pipes, redirects, and operators

### Implementation

```rust
pub fn tokenize(buffer: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quote = None; // None | Some('\'') | Some('"')
    let mut escaped = false;

    for ch in buffer.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }

        match ch {
            '\\' => {
                escaped = true;
            }
            '\'' | '"' if in_quote.is_none() => {
                in_quote = Some(ch);
            }
            '\'' | '"' if in_quote == Some(ch) => {
                in_quote = None;
            }
            ' ' | '\t' if in_quote.is_none() => {
                if !current.is_empty() {
                    tokens.push(Token::Word(current));
                    current = String::new();
                }
            }
            '|' | '>' | '<' | ';' | '&' if in_quote.is_none() => {
                if !current.is_empty() {
                    tokens.push(Token::Word(current));
                    current = String::new();
                }
                tokens.push(Token::Operator(ch));
            }
            _ => {
                current.push(ch);
            }
        }
    }

    if !current.is_empty() {
        tokens.push(Token::Word(current));
    }

    tokens
}
```

### Token Types

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// Regular word/argument
    Word(String),

    /// Option flag (starts with -)
    Option(String),

    /// Operator (|, >, <, etc.)
    Operator(char),

    /// Variable ($VAR)
    Variable(String),

    /// Command substitution $(...)
    Substitution(String),
}
```

### Examples

```rust
// Simple
tokenize("git checkout main")
// → ["git", "checkout", "main"]

// With options
tokenize("git checkout -b feature")
// → ["git", "checkout", "-b", "feature"]

// Quoted
tokenize("git commit -m 'Initial commit'")
// → ["git", "commit", "-m", "Initial commit"]

// Incomplete
tokenize("git checkout ")
// → ["git", "checkout", ""]

// Pipes
tokenize("git log | grep error")
// → ["git", "log", "|", "grep", "error"]
```

## Context Analysis

### Goal

Determine what the user is currently typing and what completions are
appropriate.

### Context Types

```rust
#[derive(Debug, Clone)]
pub struct ParseContext {
    /// Base command (e.g., "git")
    pub command: String,

    /// Subcommand chain (e.g., ["checkout"])
    pub subcommands: Vec<String>,

    /// Options encountered (e.g., ["-b", "--verbose"])
    pub options: Vec<String>,

    /// Current partial token at cursor
    pub partial: String,

    /// What we expect next
    pub expects: Expectation,

    /// Cursor position in tokens
    pub cursor_token_index: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expectation {
    /// Expecting command name
    Command,

    /// Expecting subcommand
    Subcommand,

    /// Expecting option flag
    Option,

    /// Expecting argument for command/subcommand
    Argument { for_option: Option<String> },

    /// Expecting value for option
    OptionValue { option: String },
}
```

### Algorithm

```rust
pub fn analyze_context(tokens: &[Token], cursor: usize) -> ParseContext {
    let mut context = ParseContext::default();

    // Find which token cursor is in
    let mut pos = 0;
    let mut cursor_token_idx = 0;
    for (idx, token) in tokens.iter().enumerate() {
        let token_end = pos + token.text().len();
        if cursor <= token_end {
            cursor_token_idx = idx;
            break;
        }
        pos = token_end + 1; // +1 for space
    }

    // First token is always command
    if tokens.is_empty() {
        context.expects = Expectation::Command;
        return context;
    }

    context.command = tokens[0].text().to_string();

    // Analyze remaining tokens
    let mut idx = 1;
    while idx < cursor_token_idx {
        let token = &tokens[idx];

        match token {
            Token::Option(opt) => {
                context.options.push(opt.clone());

                // Check if option takes argument
                if option_takes_argument(&context, opt) {
                    idx += 1; // Skip argument
                }
            }
            Token::Word(word) if !word.starts_with('-') => {
                // Subcommand or argument
                if is_subcommand(&context, word) {
                    context.subcommands.push(word.clone());
                }
            }
            _ => {}
        }

        idx += 1;
    }

    // Determine what we expect at cursor
    if cursor_token_idx == 0 {
        context.expects = Expectation::Command;
        context.partial = tokens[0].text().to_string();
    } else {
        let current_token = &tokens[cursor_token_idx];
        context.partial = current_token.text().to_string();

        // Check previous token
        if cursor_token_idx > 0 {
            let prev_token = &tokens[cursor_token_idx - 1];

            if let Token::Option(opt) = prev_token {
                if option_takes_argument(&context, opt) {
                    context.expects = Expectation::OptionValue {
                        option: opt.clone()
                    };
                    return context;
                }
            }
        }

        // Default: expect subcommand or option
        if current_token.text().starts_with('-') {
            context.expects = Expectation::Option;
        } else if context.subcommands.is_empty() {
            context.expects = Expectation::Subcommand;
        } else {
            context.expects = Expectation::Argument {
                for_option: None
            };
        }
    }

    context
}
```

### Examples

```rust
// After command
analyze_context("git ", 4)
// → expects: Subcommand, partial: ""

// Partial subcommand
analyze_context("git che", 7)
// → expects: Subcommand, partial: "che"

// After subcommand
analyze_context("git checkout ", 13)
// → expects: Argument, partial: ""

// After option that takes arg
analyze_context("git checkout -b ", 16)
// → expects: OptionValue("-b"), partial: ""

// Partial option value
analyze_context("git checkout -b fea", 19)
// → expects: OptionValue("-b"), partial: "fea"
```

## Spec Matching

### Goal

Match context against completion spec to generate suggestions.

### Completion Spec Structure

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct CompletionSpec {
    /// Command name
    pub name: String,

    /// Description
    pub description: Option<String>,

    /// Subcommands
    pub subcommands: Vec<Subcommand>,

    /// Options (global)
    pub options: Vec<Option>,

    /// Arguments
    pub args: Vec<Argument>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Subcommand {
    pub name: String,
    pub description: Option<String>,
    pub subcommands: Vec<Subcommand>,  // Nested
    pub options: Vec<Option>,
    pub args: Vec<Argument>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Option {
    /// Names (e.g., ["-b", "--branch"])
    pub names: Vec<String>,
    pub description: Option<String>,

    /// Does it take an argument?
    pub takes_arg: bool,

    /// Argument suggestions
    pub arg_suggestions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Argument {
    pub name: String,
    pub description: Option<String>,

    /// Static suggestions
    pub suggestions: Vec<String>,

    /// Generator function name
    pub generator: Option<String>,
}
```

### Matching Algorithm

```rust
pub fn generate_suggestions(
    context: &ParseContext,
    spec: &CompletionSpec
) -> Vec<Suggestion> {
    match &context.expects {
        Expectation::Command => {
            // Match all available commands
            match_commands(&context.partial)
        }

        Expectation::Subcommand => {
            // Navigate to current position in spec tree
            let current_spec = navigate_spec(spec, &context.subcommands);

            // Match subcommands starting with partial
            current_spec
                .subcommands
                .iter()
                .filter(|sub| sub.name.starts_with(&context.partial))
                .map(|sub| Suggestion {
                    text: sub.name.clone(),
                    description: sub.description.clone(),
                    suggestion_type: SuggestionType::Subcommand,
                })
                .collect()
        }

        Expectation::Option => {
            // Match options
            let current_spec = navigate_spec(spec, &context.subcommands);

            current_spec
                .options
                .iter()
                .flat_map(|opt| &opt.names)
                .filter(|name| name.starts_with(&context.partial))
                .map(|name| Suggestion {
                    text: name.clone(),
                    description: None, // TODO: Get from option
                    suggestion_type: SuggestionType::Option,
                })
                .collect()
        }

        Expectation::OptionValue { option } => {
            // Find option spec
            let current_spec = navigate_spec(spec, &context.subcommands);
            let opt_spec = find_option(current_spec, option)?;

            // Return suggestions for option value
            opt_spec
                .arg_suggestions
                .iter()
                .filter(|s| s.starts_with(&context.partial))
                .map(|s| Suggestion {
                    text: s.clone(),
                    description: None,
                    suggestion_type: SuggestionType::Argument,
                })
                .collect()
        }

        Expectation::Argument { .. } => {
            // Match arguments
            let current_spec = navigate_spec(spec, &context.subcommands);

            if let Some(arg) = current_spec.args.first() {
                // Static suggestions
                let mut suggestions: Vec<_> = arg
                    .suggestions
                    .iter()
                    .filter(|s| s.starts_with(&context.partial))
                    .map(|s| Suggestion {
                        text: s.clone(),
                        description: None,
                        suggestion_type: SuggestionType::Argument,
                    })
                    .collect();

                // Generator (future)
                if let Some(gen) = &arg.generator {
                    let generated = run_generator(gen, context)?;
                    suggestions.extend(generated);
                }

                suggestions
            } else {
                vec![]
            }
        }
    }
}
```

### Fuzzy Matching (Future)

```rust
pub fn fuzzy_match(needle: &str, haystack: &str) -> Option<u32> {
    // Implement fuzzy matching algorithm
    // Return score (0-100), or None if no match

    // Example: "gco" matches "git checkout" with high score
    // Uses algorithm like fzf or sublime text
}

pub fn generate_suggestions_fuzzy(
    context: &ParseContext,
    spec: &CompletionSpec
) -> Vec<Suggestion> {
    let mut suggestions = generate_suggestions(context, spec);

    // Add fuzzy matches
    let fuzzy = fuzzy_match_suggestions(context, spec);

    suggestions.extend(fuzzy);

    // Sort by score
    suggestions.sort_by_key(|s| -(s.score as i32));

    suggestions
}
```

## Generators (Phase 2)

### Purpose

Dynamic suggestions based on runtime state (e.g., git branches, file names).

### Generator Types

```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum Generator {
    /// List files in directory
    Files {
        pattern: String,
    },

    /// List directories
    Directories {
        pattern: String,
    },

    /// Run shell command
    Script {
        command: String,
    },

    /// Git-specific generators
    GitBranches,
    GitRemotes,
    GitTags,

    /// NPM-specific
    NpmScripts,
}
```

### Implementation

```rust
pub fn run_generator(
    generator: &Generator,
    context: &ParseContext
) -> Result<Vec<Suggestion>> {
    match generator {
        Generator::Files { pattern } => {
            // Glob for files matching pattern
            let paths = glob::glob(pattern)?;

            paths
                .filter_map(Result::ok)
                .map(|path| Suggestion {
                    text: path.display().to_string(),
                    description: None,
                    suggestion_type: SuggestionType::File,
                })
                .collect()
        }

        Generator::GitBranches => {
            // Run: git branch --format='%(refname:short)'
            let output = Command::new("git")
                .args(&["branch", "--format=%(refname:short)"])
                .output()?;

            let branches = String::from_utf8(output.stdout)?;

            branches
                .lines()
                .map(|line| Suggestion {
                    text: line.trim().to_string(),
                    description: Some("Git branch".to_string()),
                    suggestion_type: SuggestionType::Argument,
                })
                .collect()
        }

        Generator::Script { command } => {
            // Run arbitrary command
            let output = Command::new("sh")
                .arg("-c")
                .arg(command)
                .output()?;

            let suggestions = String::from_utf8(output.stdout)?;

            suggestions
                .lines()
                .map(|line| Suggestion {
                    text: line.to_string(),
                    description: None,
                    suggestion_type: SuggestionType::Argument,
                })
                .collect()
        }

        _ => Ok(vec![]),
    }
}
```

**Performance Considerations:**

- Cache generator results (TTL: 1s)
- Run generators async (don't block parser)
- Timeout generators (100ms max)
- Limit output (max 100 suggestions)

## Optimization Strategies

### Early Exit

```rust
pub fn parse(buffer: &str, cursor: usize) -> Result<Vec<Suggestion>> {
    // Quick checks
    if buffer.is_empty() {
        return Ok(vec![]); // No suggestions for empty buffer
    }

    if cursor == 0 {
        return Ok(vec![]); // No suggestions at start
    }

    // ... continue with parsing
}
```

### Token Caching

```rust
pub struct Parser {
    // Cache tokenization result
    token_cache: Mutex<Option<(String, Vec<Token>)>>,
}

impl Parser {
    pub fn tokenize_cached(&self, buffer: &str) -> Vec<Token> {
        let cache = self.token_cache.lock();

        if let Some((cached_buffer, tokens)) = &*cache {
            if cached_buffer == buffer {
                return tokens.clone();
            }
        }

        // Cache miss, tokenize
        let tokens = tokenize(buffer);

        *cache = Some((buffer.to_string(), tokens.clone()));

        tokens
    }
}
```

### Spec Precomputation

During build, precompute:

- Trie for fast prefix matching
- Flattened option list
- Subcommand tree depth

```rust
pub struct PrecomputedSpec {
    spec: CompletionSpec,

    // Trie for fast prefix matching
    subcommand_trie: Trie,
    option_trie: Trie,

    // Flattened for quick access
    all_subcommands: Vec<Subcommand>,
    all_options: Vec<Option>,
}
```

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_simple() {
        let tokens = tokenize("git checkout main");
        assert_eq!(tokens, vec!["git", "checkout", "main"]);
    }

    #[test]
    fn test_tokenize_quoted() {
        let tokens = tokenize("git commit -m 'Initial commit'");
        assert_eq!(tokens, vec!["git", "commit", "-m", "Initial commit"]);
    }

    #[test]
    fn test_analyze_context_after_command() {
        let context = analyze_context(&["git", ""], 4);
        assert_eq!(context.command, "git");
        assert_eq!(context.expects, Expectation::Subcommand);
        assert_eq!(context.partial, "");
    }

    #[test]
    fn test_generate_suggestions_partial_subcommand() {
        let spec = load_git_spec();
        let context = ParseContext {
            command: "git".to_string(),
            partial: "che".to_string(),
            expects: Expectation::Subcommand,
            ..Default::default()
        };

        let suggestions = generate_suggestions(&context, &spec);

        assert!(suggestions.iter().any(|s| s.text == "checkout"));
        assert!(suggestions.iter().any(|s| s.text == "cherry"));
        assert!(suggestions.iter().any(|s| s.text == "cherry-pick"));
    }
}
```

### Property-Based Tests

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_tokenize_never_panics(s in "\\PC*") {
        let _ = tokenize(&s);
    }

    #[test]
    fn test_tokenize_preserves_content(s in "\\w+") {
        let tokens = tokenize(&s);
        let rejoined = tokens.join(" ");
        // Content should be preserved (modulo whitespace)
        assert!(rejoined.contains(&s) || s.split_whitespace().count() == tokens.len());
    }

    #[test]
    fn test_cursor_always_valid(
        buffer in "\\PC{0,100}",
        cursor in 0..100usize
    ) {
        let cursor = cursor.min(buffer.len());
        let context = analyze_context(&tokenize(&buffer), cursor);
        // Should never panic
        assert!(context.command.len() <= buffer.len());
    }
}
```

### Benchmark Tests

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_tokenize(c: &mut Criterion) {
    c.bench_function("tokenize simple", |b| {
        b.iter(|| tokenize(black_box("git checkout main")))
    });

    c.bench_function("tokenize complex", |b| {
        b.iter(|| tokenize(black_box(
            "git commit -m 'message' --author='Name <email>' --no-verify"
        )))
    });
}

fn bench_parse(c: &mut Criterion) {
    let parser = Parser::new();

    c.bench_function("parse full", |b| {
        b.iter(|| parser.parse(black_box("git checkout main"), black_box(17)))
    });
}

criterion_group!(benches, bench_tokenize, bench_parse);
criterion_main!(benches);
```

**Performance Targets:**

- Tokenize: <1ms for 100-char buffer
- Context analysis: <2ms
- Spec matching: <2ms
- Total: <5ms

## Error Handling

### Validation

```rust
pub fn validate_input(buffer: &str, cursor: usize) -> Result<()> {
    // Check buffer length
    if buffer.len() > MAX_BUFFER_LEN {
        return Err(Error::BufferTooLong);
    }

    // Check cursor bounds
    if cursor > buffer.len() {
        return Err(Error::CursorOutOfBounds);
    }

    // Check valid UTF-8 (String guarantees this)

    Ok(())
}
```

### Graceful Degradation

```rust
pub fn parse(buffer: &str, cursor: usize) -> Result<Vec<Suggestion>> {
    // Validate input
    validate_input(buffer, cursor)?;

    // Try to parse
    let tokens = match tokenize(buffer) {
        Ok(tokens) => tokens,
        Err(e) => {
            warn!("Tokenization failed: {}", e);
            return Ok(vec![]); // Return empty, don't fail
        }
    };

    // Try to analyze context
    let context = match analyze_context(&tokens, cursor) {
        Ok(ctx) => ctx,
        Err(e) => {
            warn!("Context analysis failed: {}", e);
            return Ok(vec![]);
        }
    };

    // Try to generate suggestions
    match generate_suggestions(&context) {
        Ok(suggestions) => Ok(suggestions),
        Err(e) => {
            warn!("Suggestion generation failed: {}", e);
            Ok(vec![]) // Empty suggestions, not error
        }
    }
}
```

## Future Enhancements

### Semantic Analysis

Understand command semantics:

```rust
pub enum CommandType {
    FileManipulation,  // cp, mv, rm
    NetworkOperation,  // ssh, curl, wget
    ProcessControl,    // ps, kill, top
    PackageManager,    // npm, cargo, apt
}

// Provide context-aware suggestions
```

### Learning from Usage

Track which completions user selects:

```rust
pub struct CompletionHistory {
    // command -> subcommand -> frequency
    history: HashMap<String, HashMap<String, u32>>,
}

// Sort suggestions by frequency
```

### Multi-Line Support

Handle multi-line commands (heredocs, continuations):

```rust
pub fn parse_multiline(
    lines: &[String],
    cursor: (usize, usize) // (line, column)
) -> Result<Vec<Suggestion>>
```

## Related Documents

- [Architecture Overview](overview.md) - System architecture
- [Daemon Architecture](daemon.md) - Daemon design
- [TUI Architecture](tui.md) - UI rendering
- [ADR-0003: Build-time Spec Parsing](../adr/0003-build-time-spec-parsing.md)
