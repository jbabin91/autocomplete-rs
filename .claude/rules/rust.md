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

## Unsafe Code

- **Rust 2024 Edition**: `unsafe fn` no longer implies an `unsafe` block for
  the entire function body. All unsafe operations inside `unsafe fn` must be
  wrapped in explicit `unsafe { ... }` blocks. The compiler enforces this via
  `unsafe_op_in_unsafe_fn`
- **CoreFoundation ref management** (macOS): when acquiring a `CFTypeRef`
  (e.g. `AXUIElementCreateApplication`, `AXUIElementCopyAttributeValue`),
  separate acquisition from usage into an outer function (acquire + release)
  and an inner helper (use). The outer function unconditionally calls
  `CFRelease` after the inner returns, ensuring no leaks on error paths.
  Never `?`-return between acquisition and release without a cleanup guard

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
- **String width**: use `.chars().count()` for display-width calculations,
  not `.len()` (byte count). `.len()` overcounts for multi-byte UTF-8
  sequences and breaks alignment in rendering code

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
- The `timeout(join_handle).await` rule from Async (above) applies to
  tests too — use `select!` with `&mut handle` in test shutdown helpers,
  not `timeout(handle)` which detaches the task and leaks resources
- Polling loops that wait for a spawned task to become ready should
  check `handle.is_finished()` each iteration to surface startup
  errors immediately instead of timing out with a generic panic
- Never discard protocol responses in tests (`let _ = send_request(...)`)
  — always parse and assert the response to catch silent failures

## Benchmarking

- Criterion with `harness = false` — each bench file in `benches/` is a
  standalone binary with `criterion_main!`
- **Always `black_box` inputs** — `b.iter()` auto-black-boxes the closure
  return value, but inputs captured by reference can still be optimized
  away. Wrap inputs in `std::hint::black_box()` for reliable measurements
- **Never discard results** — don't `let _ =` a `Result` inside `b.iter()`.
  Return it so Criterion can black-box it. Same rule as production code
- **Use constants, not literals** — prefer `PROTOCOL_VERSION` over
  hardcoded `1`, `MAX_BUFFER_LEN` over `10_000`, etc. Benchmarks should
  stay correct when constants change
- **Extract computed inputs** — don't inline computed values in array
  literals (e.g. `&"x".repeat(1000)` borrows a temporary). Build them
  in a `let` binding first, then reference from the array
- **Async benchmarks** — create the tokio `Runtime` once per group (not
  per iteration), use `rt.block_on()` inside `b.iter()`. Use
  `tokio::io::sink()` for the write side to measure handler logic without
  I/O overhead
- **Run locally, not in CI** — benchmarks are noisy on shared runners.
  Use `mise run bench` for local regression detection

## Shared Helpers

- Cross-module utilities live in `src/paths.rs` (`pub(crate)`) — e.g.
  `home_dir()` for resolving `$HOME` with `/tmp` fallback. Don't
  duplicate small helpers across modules; extract to a shared location
- When adding behavior that multiple modules need (mode detection,
  path resolution), prefer re-exporting from the owning module rather
  than duplicating the logic

## Filesystem

- Never use `Path::exists()` for control flow — it returns `false`
  for permission errors, masking the real issue. Use `fs::metadata()`
  with explicit `ErrorKind::NotFound` matching instead
- Create secure directories atomically with
  `DirBuilder::new().mode(0o700).create()` — avoids TOCTOU windows
  between `create_dir_all` + `set_permissions`
- Doc comments on permission checks must describe the actual check,
  not an idealized one (e.g. "rejects group/other access" not
  "ensures 0700" if the check is `perms & 0o077 != 0`)
- **Unix sockets:** Always remove stale socket files before `bind()`
  (ignore `NotFound`, treat other errors as real). After a successful
  bind, set permissions to `0o600` to restrict access to the owning
  user. These rules apply everywhere sockets are created — daemon,
  examples, and tests

## Logging

- Structured logging via `tracing` — `println!`/`eprintln!` only for
  CLI user-facing output
- **Log level must match operational importance**: operational failures
  that matter in production (write timeouts, connection failures) use
  `warn!` or `error!`, not `debug!`. Production filters at `info` level,
  so `debug!` events are invisible unless troubleshooting mode is active
- On hot paths, avoid allocating just to log — prefer branched `debug!`
  calls over building an intermediate `String`. E.g. branch on redaction
  mode and log `%reference` directly instead of cloning into a variable
- Correlation IDs belong on the tracing span (`Span::current().record()`)
  so all downstream logs inherit them without explicit threading
- **Never log raw user input** (buffers, prefixes, command text) — log
  lengths or counts instead (`buffer_len`, `prefix_len`). User input
  may contain secrets (passwords in env vars, tokens in URLs). Debug
  logs can end up in crash reports, log aggregators, or shared terminals

## String Safety

- **Clamp byte offsets from untrusted input to char boundaries** before
  slicing strings. Protocol messages carry `cursor: usize` as a byte
  offset — if the remote sends an offset mid-UTF-8 sequence,
  `&buffer[..cursor]` panics. Always validate:

  ```rust
  let mut cursor = cursor.min(buffer.len());
  while cursor > 0 && !buffer.is_char_boundary(cursor) {
      cursor -= 1;
  }
  ```

  `str::floor_char_boundary()` does this but is only stable since 1.91 —
  check MSRV before using it (see `Cargo.toml` `rust-version`)

## Tooling

- Hook `check` and `fix` commands must use identical flags (e.g. both
  need `--locked` if CI uses it)
- Pre-push/pre-commit hook commands must match CI commands exactly —
  don't wrap in shell capture or add flags that CI doesn't use
- Constants for magic numbers (timeouts, size limits, protocol values)
