---
applyTo: '**/*.rs'
---

# Rust Review Guidelines

For full architecture context, see `AGENTS.md`. For tooling and testing
conventions, see `.claude/rules/tooling.md` and `docs/development/testing.md`.

## Error Handling

- Application-level code uses `anyhow::Result` — flag `Box<dyn Error>` or manual error
  type wrappers in binary/daemon code
- Library-facing errors use `thiserror` derive macros — flag raw `impl Error` when
  `thiserror` would be clearer
- Flag `unwrap()` and `expect()` outside of tests and known-safe const contexts
- Verify `?` propagation returns meaningful context — prefer `.context("msg")` from
  anyhow on fallible operations that cross module boundaries

## Async & Tokio Patterns

- All async cancellation must use `tokio_util::sync::CancellationToken` — flag
  ad-hoc boolean flags or manual channel-based shutdown
- `tokio::select!` should use `biased;` when branch priority matters (shutdown checks
  must come first)
- Flag `tokio::spawn` without corresponding `JoinSet` or `JoinHandle` tracking — orphan
  tasks leak on shutdown
- Flag `sleep`-based polling when `tokio::sync::Notify` or channel-based waking is
  appropriate
- Timeouts must use `tokio::time::timeout` — flag open-ended reads/writes on sockets

## Resource Management

- Socket paths and temp files must be cleaned up via RAII (Drop) — flag manual cleanup
  in happy-path-only code
- Semaphore-based backpressure (`try_acquire_owned`) for connection limiting — flag
  unbounded accept loops
- PID files use `kill(pid, 0)` liveness checks — flag file-existence-only checks

## Type Design

- Public types should derive `Debug` — flag public structs/enums missing it
- Serde types should derive both `Serialize` and `Deserialize` unless single-direction
- Tagged enums (`#[serde(tag = "type")]`) for protocol messages — flag untagged enums
  in IPC boundaries
- Prefer `Arc<dyn Trait>` for dependency injection over concrete types in daemon state

## Code Style

- 100 character max line width (enforced by rustfmt)
- Rust 2024 edition idioms — flag deprecated patterns
- Structured logging via `tracing` macros (`info!`, `debug!`, `warn!`, `error!`) — flag
  `println!` or `eprintln!` outside of CLI user-facing output
- Use `#[instrument(skip_all)]` on async functions that take non-Debug parameters
- Constants for magic numbers — flag bare numeric literals in timeout durations, size
  limits, or protocol values

## Testing

See `docs/development/testing.md` for full testing patterns and conventions.

- Tests use `cargo nextest` (not `cargo test`) — each test runs as a separate process
- Integration tests that create socket paths must use atomic counters for uniqueness,
  not timestamps or random values
- `#[tokio::test]` for async tests
- Flag tests without assertions or with only `assert!(true)`
