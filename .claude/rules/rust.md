---
paths:
  - '**/*.rs'
---

# Rust Patterns

General patterns for writing Rust in this codebase. For daemon-specific
rules see `daemon.md`. For tooling and CI see `tooling.md`.

## Error Handling

- `anyhow::Result` for application code, `thiserror` for library errors
- Never `let _ =` on a `Result` — check `ErrorKind::NotFound` vs real errors:
  - Functions: `return Err(e).context("what failed")`
  - Drop impls: `tracing::warn!` (can't propagate errors)
  - Tests: `panic!` (surfaces failures in test output)
- Never `Err(_)` when the error type has multiple failure modes (`io::Error`,
  `anyhow::Error`). Only discard single-meaning errors (`tokio::time::Elapsed`,
  `TryAcquireError`)
- Add `.context("msg")` on `?` when crossing module boundaries
- Error messages must distinguish failure modes — different causes need
  different messages
- Don't turn operational errors into destructive cleanup. Only delete a
  socket file on `ConnectionRefused` (definitely stale). Other connect
  errors (`PermissionDenied`, transient FS errors) could mean a live
  daemon — deleting its socket would break it

## Type Safety

- Use `TryFrom`/`try_into()` for numeric conversions that could overflow
  (e.g. `u32` to `i32`). Only use `as` when lossless is guaranteed
  (`u16 as u32`) or truncation is intentional and documented
- Public types derive `Debug`. Serde types derive both `Serialize` and
  `Deserialize` unless single-direction

## Protocol

- Parse protocol responses structurally (deserialize into typed structs).
  Never substring-match on JSON (`line.contains("status")` gives false
  positives from error messages)
- Handle zero-byte `read_line` as EOF — don't fall through to JSON parsing
- Tagged enums (`#[serde(tag = "type")]`) for IPC message boundaries

## Async

- `CancellationToken` for shutdown — not booleans or channels
- `tokio::select!` with `biased;` when branch priority matters
- `tokio::time::timeout` on all socket I/O (reads AND writes)
- Track spawned tasks in `JoinSet` — no orphan tasks
- Long-running `select!` loops with a `JoinSet` must include a
  `tasks.join_next()` branch to reap completed tasks — otherwise the
  JoinSet grows without bound (finished tasks are retained until joined)
- After `abort_all()`, drain with `while tasks.join_next().await.is_some() {}`
- Dropping a `JoinHandle` detaches the task (it keeps running). When using
  `timeout(handle).await`, grab an `AbortHandle` first so the task can be
  explicitly aborted on timeout instead of silently leaked

## Testing

- `cargo nextest` (not `cargo test`)
- Test helpers must assert every intermediate result — no `let _ =`
- Wrap I/O test helpers in `tokio::time::timeout` to prevent hangs
- Atomic counters for unique socket paths, not timestamps or random values

## Tooling

- Hook `check` and `fix` commands must use identical flags (e.g. both
  need `--locked` if CI uses it)
- Structured logging via `tracing` — `println!`/`eprintln!` only for
  CLI user-facing output
- Constants for magic numbers (timeouts, size limits, protocol values)
