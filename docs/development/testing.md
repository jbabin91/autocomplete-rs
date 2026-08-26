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

```sh
# All unit tests (via nextest)
mise run test

# Specific module
cargo nextest run -E 'test(tokenizer)'

# With output visible
cargo nextest run --no-capture
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

**Example:** See `tests/daemon_integration.rs` for the real tests. Key patterns:

```rust
// Unique socket paths via atomic counter (not timestamps or random values)
static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);
fn temp_socket_path() -> PathBuf {
    let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    // The daemon refuses a group/other-accessible socket directory, so sockets live in a
    // per-process 0700 directory that is removed when the test binary exits.
    test_socket_dir().join(format!("{}.sock", id))
}

// Helper that asserts every step — never discard intermediate results.
// Grabs an AbortHandle before awaiting so the task is cancelled on timeout
// instead of silently leaked (dropping a JoinHandle detaches the task).
async fn shutdown_daemon(socket_path: &Path, handle: JoinHandle<()>) {
    let resp = send_request(socket_path, r#"{"type":"shutdown"}"#).await;
    let ack: serde_json::Value =
        serde_json::from_str(&resp).expect("shutdown response is valid JSON");
    assert_eq!(ack["status"], "shutting_down", "expected ShutdownAck, got: {resp}");
    let abort = handle.abort_handle();
    match tokio::time::timeout(Duration::from_secs(2), handle).await {
        Ok(result) => result.expect("daemon task panicked"),
        Err(_) => {
            abort.abort();
            panic!("daemon did not exit within timeout — task aborted");
        }
    }
}
```

**Run:**

```sh
# All integration tests (flags defined in .mise.toml)
mise run test

# Specific test file
cargo nextest run --test completion_flow
```

**Test helper timeouts:** Wrap I/O helpers in `tokio::time::timeout` to convert hangs
into deterministic failures:

```rust
async fn send_request(socket_path: &Path, json: &str) -> String {
    tokio::time::timeout(Duration::from_secs(5), async {
        let stream = UnixStream::connect(socket_path).await.unwrap();
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        writer.write_all(json.as_bytes()).await.unwrap();
        writer.write_all(b"\n").await.unwrap();
        writer.flush().await.unwrap();
        drop(writer);

        let mut response = String::new();
        reader.read_line(&mut response).await.unwrap();
        response.trim().to_string()
    })
    .await
    .expect("send_request timed out after 5s")
}
```

**Best Practices:**

- Test realistic scenarios
- Clean up resources (sockets, temp files)
- Use tokio::test for async tests
- Test error conditions
- Verify end-to-end behavior
- Wrap test helpers that do I/O in timeouts to prevent CI hangs

### Performance Benchmarks

**What:** Measure execution time of critical paths

**Location:** `benches/` directory (Criterion, `harness = false`)

**Current benchmark suites:**

| Suite    | File                  | What it measures                                                                                                       |
| -------- | --------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| engine   | `benches/engine.rs`   | `StubEngine::complete()` via `Arc<dyn CompletionEngine>` (short/medium/long inputs)                                    |
| protocol | `benches/protocol.rs` | JSON deserialization (`CompletionRequest`, `DaemonMessage`, shutdown, malformed) + `validate_request()`                |
| privacy  | `benches/privacy.rs`  | `redact_buffer()` (short/medium/long/unicode) + `redact_sensitive_patterns()` (clean, password, URL, export, combined) |
| handler  | `benches/handler.rs`  | Full `handle_connection()` async roundtrip with in-memory I/O                                                          |

**Example:**

```rust
use std::hint::black_box;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use autocomplete_rs::protocol::{CompletionRequest, PROTOCOL_VERSION, validate_request};

fn bench_validation(c: &mut Criterion) {
    let req = CompletionRequest {
        buffer: "git commit -m 'message'".into(),
        cursor: 23,
        version: PROTOCOL_VERSION, // use constants, not literals
    };

    let mut group = c.benchmark_group("protocol/validate");
    group.bench_with_input(BenchmarkId::new("request", "valid"), &req, |b, req| {
        b.iter(|| validate_request(black_box(req))); // black_box inputs
    });
    group.finish();
}

criterion_group!(benches, bench_validation);
criterion_main!(benches);
```

**Run:**

```sh
# All benchmarks (flags defined in .mise.toml)
mise run bench

# Specific suite
cargo bench --bench engine

# HTML reports (auto-generated)
open target/criterion/**/report/index.html
```

**Performance Targets:**

- Daemon startup: <5ms
- IPC round-trip: <1ms
- Parser: <5ms per request
- Inline dropdown render: <10ms
- Total latency: <20ms

**Best Practices:**

- **`black_box` inputs** — `b.iter()` auto-black-boxes the return value,
  but inputs captured by reference can be optimized away. Wrap them in
  `std::hint::black_box()`
- **Never discard results** — don't `let _ =` a `Result` inside `b.iter()`.
  Return it so Criterion can black-box it
- **Use constants** — `PROTOCOL_VERSION` not `1`, `MAX_BUFFER_LEN` not `10_000`
- **Async benchmarks** — create the tokio `Runtime` once per group, use
  `rt.block_on()` inside `b.iter()`. Use `tokio::io::sink()` for the write
  side to measure logic without I/O overhead
- Run benchmarks on a quiet system
- Compare before/after when optimizing
- Not in CI (noisy on shared runners) — run locally

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
    ├── engine.rs          # CompletionEngine::complete() benchmarks
    ├── protocol.rs        # JSON deserialization + validation benchmarks
    ├── privacy.rs         # Redaction function benchmarks
    └── handler.rs         # Full handler roundtrip benchmarks
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
    let socket = temp_socket_path(); // atomic counter, not hardcoded

    let handle = start_daemon(&socket).await;

    // Test connection
    let stream = UnixStream::connect(&socket).await;
    assert!(stream.is_ok());

    // Graceful shutdown — grab AbortHandle before timeout to avoid task leak
    shutdown_daemon(&socket, handle).await;
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
        // Property: parser should never panic, even with random input.
        // Result intentionally discarded — we only care about no-panic.
        drop(parse_buffer(&buffer, cursor));
    }
}
```

## Testing Shell Integration

### Manual Testing

Create test script:

```sh
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

```sh
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

```sh
cargo install cargo-tarpaulin
```

Generate coverage:

```sh
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

CI's Tests job runs `mise run test`. See `.mise.toml` for the exact commands and
`docs/conventions/github-actions.md` for full CI/CD documentation.

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

```sh
cargo nextest run -E 'test(test_parse_git_checkout)'
# Should see: FAIL
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

```sh
cargo nextest run -E 'test(test_parse_git_checkout)'
# Should see: PASS
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

```sh
mise run test
```

## Debugging Failing Tests

### Run Single Test with Output

```sh
cargo nextest run -E 'test(test_name)' --no-capture
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
   `mise run ci`
9. **Aim for >80% coverage** overall
10. **Keep tests fast:** <10s for full suite

## Next Steps

- Read [Contributing Guide](contributing.md) for contribution workflow
- Read [Project Structure](project-structure.md) to understand codebase
- Check [GitHub Issues](https://github.com/jbabin91/autocomplete-rs/issues) for current priorities
- Start with tests tagged `good-first-issue`
