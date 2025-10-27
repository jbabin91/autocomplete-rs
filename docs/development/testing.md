# Testing Guide

This guide covers testing practices, patterns, and tools for autocomplete-rs
development.

## Testing Philosophy

We follow these principles:

1. **Fast feedback:** Unit tests run in <1s, full suite in <10s
2. **Reliable:** Tests are deterministic, no flaky tests
3. **Comprehensive:** Test happy paths, edge cases, and errors
4. **Maintainable:** Tests are clear and easy to update
5. **Performance-aware:** Benchmark critical paths

## Test Types

### Unit Tests

**What:** Test individual functions and small components in isolation

**Location:** Inline in source files using `#[cfg(test)]` modules

**Example:**

```rust
// src/parser/tokenizer.rs
pub fn tokenize(buffer: &str) -> Vec<String> {
    buffer.split_whitespace().map(String::from).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_simple() {
        let tokens = tokenize("git checkout main");
        assert_eq!(tokens, vec!["git", "checkout", "main"]);
    }

    #[test]
    fn test_tokenize_empty() {
        let tokens = tokenize("");
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_tokenize_multiple_spaces() {
        let tokens = tokenize("git  checkout   main");
        assert_eq!(tokens, vec!["git", "checkout", "main"]);
    }
}
```

**Run:**

```bash
# All unit tests
cargo test

# Specific module
cargo test tokenizer

# With output
cargo test -- --nocapture
```

**Best Practices:**

- One assertion per test when possible
- Use descriptive test names (`test_<scenario>_<expected>`)
- Test edge cases (empty input, max values, invalid data)
- Use `assert_eq!` for equality, `assert!` for conditions
- Mock external dependencies

### Integration Tests

**What:** Test multiple components working together

**Location:** `tests/` directory

**Example:**

```rust
// tests/completion_flow.rs
use autocomplete_rs::{daemon, parser, tui};
use std::path::Path;
use tokio::net::UnixStream;

#[tokio::test]
async fn test_end_to_end_completion() {
    // Start daemon
    let socket_path = "/tmp/test-autocomplete.sock";
    tokio::spawn(async {
        daemon::start(socket_path).await.unwrap();
    });

    // Wait for daemon to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect and send request
    let mut stream = UnixStream::connect(socket_path).await.unwrap();
    let request = json!({
        "buffer": "git checkout ",
        "cursor": 13
    });
    stream.write_all(request.to_string().as_bytes()).await.unwrap();

    // Read response
    let mut response = String::new();
    stream.read_to_string(&mut response).await.unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
    assert!(parsed["suggestions"].is_array());
    assert!(!parsed["suggestions"].as_array().unwrap().is_empty());

    // Cleanup
    std::fs::remove_file(socket_path).unwrap();
}
```

**Run:**

```bash
# All integration tests
cargo test --test '*'

# Specific test file
cargo test --test completion_flow
```

**Best Practices:**

- Test realistic scenarios
- Clean up resources (sockets, temp files)
- Use tokio::test for async tests
- Test error conditions
- Verify end-to-end behavior

### Performance Benchmarks

**What:** Measure execution time of critical paths

**Location:** `benches/` directory

**Example:**

```rust
// benches/parser_bench.rs
use autocomplete_rs::parser::Parser;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_parse_simple_command(c: &mut Criterion) {
    let parser = Parser::new();
    c.bench_function("parse git checkout", |b| {
        b.iter(|| {
            parser.parse(black_box("git checkout main"), black_box(17))
        })
    });
}

fn bench_parse_complex_command(c: &mut Criterion) {
    let parser = Parser::new();
    c.bench_function("parse kubectl with flags", |b| {
        b.iter(|| {
            parser.parse(
                black_box("kubectl get pods --namespace=prod --selector=app=web"),
                black_box(55)
            )
        })
    });
}

criterion_group!(benches, bench_parse_simple_command, bench_parse_complex_command);
criterion_main!(benches);
```

**Setup:**

```toml
# Cargo.toml
[dev-dependencies]
criterion = "0.5"

[[bench]]
name = "parser_bench"
harness = false
```

**Run:**

```bash
# All benchmarks
cargo bench

# Specific benchmark
cargo bench parser

# With detailed output
cargo bench -- --verbose
```

**Performance Targets:**

- Daemon startup: <5ms
- IPC round-trip: <1ms
- Parser: <5ms per request
- TUI render: <10ms
- Total latency: <20ms

**Best Practices:**

- Benchmark realistic inputs
- Use `black_box` to prevent optimization
- Run benchmarks on quiet system
- Compare before/after when optimizing
- Track performance over time

## Test Organization

### Directory Structure

```sh
autocomplete-rs/
├── src/
│   ├── parser/
│   │   ├── mod.rs
│   │   └── #[cfg(test)] mod tests { ... }
│   └── daemon/
│       ├── mod.rs
│       └── #[cfg(test)] mod tests { ... }
├── tests/
│   ├── completion_flow.rs
│   ├── daemon_integration.rs
│   └── fixtures/
│       ├── test-specs/
│       └── sample-buffers.txt
└── benches/
    ├── daemon_bench.rs
    ├── parser_bench.rs
    └── tui_bench.rs
```

### Naming Conventions

**Unit tests:**

```rust
#[test]
fn test_<function>_<scenario>() { }
#[test]
fn test_<function>_<scenario>_<expected>() { }
```

Examples:

- `test_tokenize_empty()`
- `test_tokenize_multiple_spaces()`
- `test_parse_git_checkout_returns_branches()`

**Integration tests:**

```rust
#[tokio::test]
async fn test_<feature>_<scenario>() { }
```

Examples:

- `test_daemon_handles_concurrent_connections()`
- `test_completion_flow_end_to_end()`

## Mocking and Test Doubles

### Mocking External Dependencies

Use `mockall` for mocking:

```rust
// src/specs/loader.rs
use mockall::automock;

#[automock]
pub trait SpecProvider {
    fn load_spec(&self, name: &str) -> Result<CompletionSpec>;
}

// In tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_with_mocked_specs() {
        let mut mock_provider = MockSpecProvider::new();
        mock_provider
            .expect_load_spec()
            .with(eq("git"))
            .times(1)
            .returning(|_| Ok(create_test_git_spec()));

        let parser = Parser::new(Box::new(mock_provider));
        let result = parser.parse("git checkout", 12);
        assert!(result.is_ok());
    }
}
```

### Test Fixtures

Create reusable test data:

```rust
// tests/fixtures/mod.rs
pub fn sample_git_spec() -> CompletionSpec {
    CompletionSpec {
        name: "git".to_string(),
        subcommands: vec![
            Subcommand {
                name: "checkout".to_string(),
                description: Some("Switch branches".to_string()),
                options: vec![
                    Option {
                        names: vec!["-b".to_string()],
                        description: Some("Create new branch".to_string()),
                    }
                ],
            }
        ],
    }
}

// Use in tests
use crate::fixtures::sample_git_spec;

#[test]
fn test_with_git_spec() {
    let spec = sample_git_spec();
    // ... test logic
}
```

## Testing Async Code

### Using Tokio Test Runtime

```rust
#[tokio::test]
async fn test_daemon_startup() {
    let socket = "/tmp/test.sock";

    // Spawn daemon in background
    let handle = tokio::spawn(async move {
        daemon::start(socket).await
    });

    // Wait for startup
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Test connection
    let stream = UnixStream::connect(socket).await;
    assert!(stream.is_ok());

    // Cleanup
    handle.abort();
    std::fs::remove_file(socket).unwrap();
}
```

### Testing Timeouts

```rust
#[tokio::test]
async fn test_request_timeout() {
    let result = tokio::time::timeout(
        Duration::from_millis(100),
        slow_operation()
    ).await;

    assert!(result.is_err(), "Expected timeout");
}
```

## Property-Based Testing

Use `proptest` for generative testing:

```toml
[dev-dependencies]
proptest = "1.0"
```

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_tokenize_preserves_content(s in "\\PC+") {
        let tokens = tokenize(&s);
        let rejoined = tokens.join(" ");
        // Property: tokenizing and rejoining preserves words
        prop_assert_eq!(s.split_whitespace().collect::<Vec<_>>(), tokens);
    }

    #[test]
    fn test_parser_never_panics(buffer in ".*", cursor in 0..1000usize) {
        // Property: parser should never panic, even with random input
        let _ = parse_buffer(&buffer, cursor);
    }
}
```

## Testing Shell Integration

### Manual Testing

Create test script:

```bash
#!/bin/bash
# test-zsh-integration.sh

# Source the integration
source ./shell-integration/zsh.zsh

# Simulate buffer and cursor
BUFFER="git checkout "
CURSOR=13

# Trigger widget
_autocomplete_rs_widget

# Check result
echo "New buffer: $BUFFER"
echo "New cursor: $CURSOR"
```

### Automated Testing

Use `expect` for automated shell testing:

```bash
#!/usr/bin/expect
# tests/zsh-integration.exp

spawn zsh
expect "% "

# Source integration
send "source ./shell-integration/zsh.zsh\r"
expect "% "

# Type command
send "git checkout "
expect "git checkout "

# Trigger completion (Alt+Space)
send "\033 "

# Verify dropdown appears
expect "Suggestions"

# Select first item
send "\r"

expect "% "
send "echo done\r"
expect "done"
```

## Coverage

### Generate Coverage Report

Install tarpaulin:

```bash
cargo install cargo-tarpaulin
```

Generate coverage:

```bash
# HTML report
cargo tarpaulin --out Html

# Console output
cargo tarpaulin --out Stdout

# With colored output
cargo tarpaulin --out Lcov | genhtml -o coverage/
```

**Coverage Goals:**

- Overall: >80%
- Critical paths (parser, daemon): >90%
- New code: >85%

## Continuous Integration

### GitHub Actions Workflow

```yaml
# .github/workflows/test.yml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest]
        rust: [stable, nightly]

    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          toolchain: ${{ matrix.rust }}

      - name: Run tests
        run: cargo test --all-features

      - name: Run clippy
        run: cargo clippy -- -D warnings

      - name: Check formatting
        run: cargo fmt --check

  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Run benchmarks
        run: cargo bench --no-fail-fast
```

## Test-Driven Development (TDD)

### TDD Workflow

1. **Write failing test:**

```rust
#[test]
fn test_parse_git_checkout_suggests_branches() {
    let parser = Parser::new();
    let result = parser.parse("git checkout ", 13);

    assert!(result.is_ok());
    let suggestions = result.unwrap();
    assert!(!suggestions.is_empty());
    assert!(suggestions.iter().any(|s| s.text == "main"));
}
```

1. **Run test (should fail):**

```bash
cargo test test_parse_git_checkout
# Should see: test result: FAILED
```

1. **Implement minimum code to pass:**

```rust
pub fn parse(&self, buffer: &str, cursor: usize) -> Result<Vec<Suggestion>> {
    if buffer.starts_with("git checkout") {
        return Ok(vec![
            Suggestion { text: "main".to_string(), .. }
        ]);
    }
    Ok(vec![])
}
```

1. **Run test (should pass):**

```bash
cargo test test_parse_git_checkout
# Should see: test result: ok
```

1. **Refactor:**

```rust
pub fn parse(&self, buffer: &str, cursor: usize) -> Result<Vec<Suggestion>> {
    let tokens = self.tokenizer.tokenize(buffer);
    let context = self.analyze_context(&tokens, cursor);
    self.generate_suggestions(context)
}
```

1. **Rerun tests (should still pass):**

```bash
cargo test
```

## Debugging Failing Tests

### Run Single Test with Output

```bash
cargo test test_name -- --nocapture --test-threads=1
```

### Use dbg! Macro

```rust
#[test]
fn test_parser() {
    let result = parser.parse(buffer, cursor);
    dbg!(&result);  // Prints debug representation
    assert!(result.is_ok());
}
```

### Print Test Values

```rust
#[test]
fn test_tokenize() {
    let tokens = tokenize(input);
    println!("Input: {:?}", input);
    println!("Tokens: {:?}", tokens);
    assert_eq!(tokens.len(), 3);
}
```

## Common Testing Patterns

### Table-Driven Tests

```rust
#[test]
fn test_parse_various_buffers() {
    let test_cases = vec![
        ("git checkout", 13, vec!["main", "develop"]),
        ("git commit -m", 13, vec![]),
        ("npm install", 12, vec!["--save", "--save-dev"]),
    ];

    for (buffer, cursor, expected) in test_cases {
        let result = parser.parse(buffer, cursor).unwrap();
        let texts: Vec<_> = result.iter().map(|s| &s.text).collect();
        assert_eq!(texts, expected, "Failed for buffer: {}", buffer);
    }
}
```

### Snapshot Testing

```rust
use insta::assert_debug_snapshot;

#[test]
fn test_parser_output() {
    let result = parser.parse("git checkout main", 17);
    assert_debug_snapshot!(result);
}
```

## Best Practices Summary

1. **Write tests first** (TDD) when adding new features
2. **Test edge cases:** empty input, max values, invalid data
3. **One logical assertion per test** when possible
4. **Use descriptive test names** that explain scenario
5. **Clean up resources** in tests (files, sockets)
6. **Mock external dependencies** for unit tests
7. **Measure performance** for critical paths
8. **Run tests before committing:**
   `cargo test && cargo clippy && cargo fmt --check`
9. **Aim for >80% coverage** overall
10. **Keep tests fast:** <10s for full suite

## Next Steps

- Read [Contributing Guide](contributing.md) for contribution workflow
- Read [Project Structure](project-structure.md) to understand codebase
- Check [ROADMAP.md](../../openspec/ROADMAP.md) for current priorities
- Start with tests tagged `good-first-issue`
