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
  - Channel sends (`send()`, `try_send()`): at minimum `debug!`/`warn!`
    the failure — a closed or full channel is diagnosable info, not noise
- Choose `try_send()` vs `send().await` based on path criticality:
  - **Hot path** (per-request): `try_send()` — non-blocking, drop on
    backpressure, warn and move on
  - **Shutdown path** (session stop, flush): `send().await` with
    `tokio::time::timeout` — reliable delivery matters, but must not
    block shutdown indefinitely
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
- Keep doc comments in sync with behavior — if a function logs on error,
  don't document it as "silently dropped" (and vice versa)

## Type Safety

- Use `TryFrom`/`try_into()` for numeric conversions that could overflow
  (e.g. `u64` to `i64`). Prefer clamping (`unwrap_or(i64::MAX)`) over
  panicking when overflow is non-critical (e.g. metrics storage)
- For lossless widening, prefer `From` trait (`i64::from(u32_val)`) over
  `as` — the compiler enforces that the conversion is actually lossless.
  Reserve `as` for cases where `From` isn't available
- Public types derive `Debug`. Serde types derive both `Serialize` and
  `Deserialize` unless single-direction
- Add `#[must_use]` to RAII guards whose Drop has side effects (e.g.
  `ConnectionGuard` decrementing a counter) and builder methods that
  return `Self` — prevents silent no-ops when the return value is ignored

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
- Avoid `timeout(join_handle).await` — it consumes the handle, so on timeout
  the task is detached and can't be observed. Use `select!` with `sleep` +
  `&mut handle` instead, then `abort()` + `handle.await` to observe cancellation

## Testing

- `cargo nextest` (not `cargo test`)
- Test helpers must assert every intermediate result — no `let _ =`
- Wrap I/O test helpers in `tokio::time::timeout` to prevent hangs
- Atomic counters for unique socket paths, not timestamps or random values
- When testing spawned tasks/actors, await the `JoinHandle` after sending
  a shutdown signal — never use `sleep()` to "wait for processing". Fixed
  sleeps are flaky under load and slow down the suite
- `sleep`-based waits are acceptable only for polling loops with retry
  (e.g. waiting for a socket to become ready) or async file cleanup

## Tooling

- Hook `check` and `fix` commands must use identical flags (e.g. both
  need `--locked` if CI uses it)
- Structured logging via `tracing` — `println!`/`eprintln!` only for
  CLI user-facing output
- Constants for magic numbers (timeouts, size limits, protocol values)
