---
applyTo: '**/*.rs'
---

# Rust Review Guidelines

For full architecture context, see `AGENTS.md`. For tooling and testing
conventions, see `.claude/rules/tooling.md` and `docs/development/testing.md`.

## Review Approach

- CI enforces rustfmt, clippy, and test passing — don't duplicate those checks
- Focus on what CI can't catch: race conditions, missing timeouts, unguarded resource
  cleanup, doc/code divergence, and flag consistency across CI/hooks/mise
- Hook parity: `check` and `fix` commands in hk.pkl must use identical flags (e.g. both
  must include `--locked` if CI uses it) — flag asymmetry between check/fix paths

## Error Handling

- Application-level code uses `anyhow::Result` — flag `Box<dyn Error>` or manual error
  type wrappers in binary/daemon code
- Library-facing errors use `thiserror` derive macros — flag raw `impl Error` when
  `thiserror` would be clearer
- Flag `unwrap()` and `expect()` outside of tests and known-safe const contexts
- Flag `let _ =` on `Result` values in production code — file operations must check
  `ErrorKind::NotFound` (expected) vs real errors (permission denied, disk full). Use
  `return Err(e).context(...)` in functions, `tracing::warn!` in Drop impls
- Flag `let _ =` on channel `send()`/`try_send()` — log the failure (at minimum
  `debug!`) so channel-closed vs channel-full conditions are diagnosable
- Flag `send().await` without `tokio::time::timeout` in shutdown paths — a stalled
  receiver or full channel can block shutdown indefinitely. Always wrap in a timeout
- Flag `try_send()` for lifecycle-critical events (session stop, flush) on shutdown
  paths — these should use `send().await` with timeout for reliable delivery since
  dropped events leave stale state in the database
- Flag `Err(_)` that discards error details when the error type has multiple failure
  modes (e.g. `io::Error` could be `PermissionDenied`, `NotFound`, `BrokenPipe`). Only
  discard when the error type is single-meaning (e.g. `tokio::time::Elapsed` always
  means "timed out", `TryAcquireError` for semaphore backpressure)
- Verify `?` propagation returns meaningful context — prefer `.context("msg")` from
  anyhow on fallible operations that cross module boundaries
- Error messages must distinguish failure modes — flag generic messages that conflate
  different causes (e.g. "unknown type" for both unrecognized types and valid types
  with malformed payloads)
- Protocol responses must be parsed structurally (deserialize into typed structs) — flag
  substring matching on JSON strings (e.g. `line.contains("shutting_down")` is fragile
  and can produce false positives from error messages containing the same text)

## Async & Tokio Patterns

- All async cancellation must use `tokio_util::sync::CancellationToken` — flag
  ad-hoc boolean flags or manual channel-based shutdown
- `tokio::select!` should use `biased;` when branch priority matters (shutdown checks
  must come first)
- Flag `tokio::spawn` without corresponding `JoinSet` or `JoinHandle` tracking — orphan
  tasks leak on shutdown
- Flag `sleep`-based polling when `tokio::sync::Notify` or channel-based waking is
  appropriate
- Timeouts must use `tokio::time::timeout` on both reads AND writes — flag any
  open-ended socket I/O (a stalled client can block handler tasks indefinitely)
- Signal futures (`signal::ctrl_c()`) must be created once outside loops and pinned
  — flag re-creation inside `select!` loops (wasteful re-registration per iteration)
- Long-running `select!` loops with a `JoinSet` must include a `tasks.join_next()`
  branch to reap completed tasks — flag loops that only drain at shutdown (the JoinSet
  grows without bound as finished tasks are retained until joined)
- After `JoinSet::abort_all()`, call `while tasks.join_next().await.is_some() {}`
  to observe cancelled tasks — flag bare `abort_all()` without draining
- Flag `timeout(join_handle).await` without an `AbortHandle` — dropping a `JoinHandle`
  detaches the task (it keeps running). Grab `handle.abort_handle()` before the timeout
  and call `abort()` on timeout to prevent silent task leaks

## Resource Management

- Socket paths and temp files must be cleaned up via RAII (Drop) — flag manual cleanup
  in happy-path-only code
- Flag stale-socket deletion that triggers on any connect error — only `ConnectionRefused`
  means "definitely stale". Other errors (`PermissionDenied`, transient FS) could indicate
  a live daemon; deleting its socket would break it
- Semaphore-based backpressure (`try_acquire_owned`) for connection limiting — flag
  unbounded accept loops
- PID files use `kill(pid, 0)` liveness checks — flag file-existence-only checks
- File creation for single-instance enforcement must use `OpenOptions::create_new(true)`
  for atomicity — flag check-then-write patterns (TOCTOU race)
- Directory validation (`ensure_*_dir` helpers) must check `metadata.is_dir()` after
  `path.exists()` — flag code that assumes an existing path is a directory without
  verifying (a file at the path produces a cryptic error later)

## Type Design

- Public types should derive `Debug` — flag public structs/enums missing it
- Serde types should derive both `Serialize` and `Deserialize` unless single-direction
- Tagged enums (`#[serde(tag = "type")]`) for protocol messages — flag untagged enums
  in IPC boundaries
- Prefer `Arc<dyn Trait>` for dependency injection over concrete types in daemon state

## Numeric Safety

- Flag `as` casts between integer types that could overflow (e.g. `u64 as i64`) — use
  `TryFrom`/`try_into()` with a fallback (e.g. `.unwrap_or(i64::MAX)`) for conversions
  where the source range exceeds the target type
- For lossless widening, flag `as` and prefer `From` trait (e.g. `i64::from(u32_val)`)
  — the compiler enforces that the conversion is actually lossless, whereas `as` silently
  compiles even when a future type change makes the cast lossy
- Flag `row.get(idx).ok()` in database code — this hides decode/type errors by
  converting them to `None`. For nullable columns, use typed extraction
  (`row.get::<Option<T>>(idx).context("column_name")?`) so only SQL NULL becomes `None`

## Code Style

- 100 character max line width (enforced by rustfmt)
- Rust 2024 edition idioms — flag deprecated patterns
- Structured logging via `tracing` macros (`info!`, `debug!`, `warn!`, `error!`) — flag
  `println!` or `eprintln!` outside of CLI user-facing output
- Use `#[instrument(skip_all)]` on async functions that take non-Debug parameters
- Constants for magic numbers — flag bare numeric literals in timeout durations, size
  limits, or protocol values (e.g. use `PROTOCOL_VERSION` not `1`)
- Protocol message fallback parsing must check for `"type"` field presence — if JSON
  has a `"type"` field but fails to parse as a known variant, return an error instead
  of silently falling back to a bare request type
- Doc comments must match actual behavior — flag "silently dropped" when code logs a
  warning, or "read-only" when the API doesn't enforce it. If a constraint is
  caller-enforced rather than API-enforced, document it as such
- Flag public RAII guards and builder methods missing `#[must_use]` — dropping a guard
  immediately negates its effect, and ignoring a builder return silently discards config
- Flag hardcoded Rust version numbers (e.g. "1.88+") in documentation or comments — the
  canonical MSRV is `rust-version` in `Cargo.toml`. Docs should reference the config file,
  not repeat the value (repeated values drift on every bump)

## Testing

See `docs/development/testing.md` for full testing patterns and conventions.

- Tests use `cargo nextest` (not `cargo test`) — each test runs as a separate process
- Integration tests that create socket paths must use atomic counters for uniqueness,
  not timestamps or random values
- `#[tokio::test]` for async tests
- Flag tests without assertions or with only `assert!(true)`
- Test helpers must assert on every intermediate result, not just the final outcome
  — flag `let _ =` patterns anywhere in test code (including inside helpers). A helper
  that asserts only the last step masks failures in earlier steps (e.g. asserting daemon
  exit but ignoring whether the shutdown response was a valid `ShutdownAck`)
- Test comments must match what the assertion actually checks — flag misleading
  comments that describe a weaker check than the code enforces
- Flag `sleep()`-based waits for spawned task/actor completion — await the
  `JoinHandle` after sending a shutdown signal for deterministic behavior.
  Fixed sleeps are flaky under CI load and slow the suite. `sleep` is only
  acceptable for polling loops with retry (e.g. waiting for a socket)
