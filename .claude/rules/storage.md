---
paths:
  - 'src/storage/**'
---

# Storage Layer Rules

## Architecture

- **turso** (pure-Rust SQLite rewrite) for local-only embedded storage —
  no native build dependencies (no cmake, cc, or bindgen). Zero C
  compilation required for `cargo install`
- **Channel+Actor pattern** keeps DB writes off the completion hot path:
  `mpsc::Sender<StorageEvent>` in `DaemonState`, background actor owns
  the `Connection` and batches writes into transactions
- **Degraded mode** — daemon continues if storage init fails
  (`Option<StorageEventSender>`). Storage is observability, not
  business-critical
- **`try_send()` for hot-path event emission** — synchronous,
  non-blocking. If the channel is full (1024 cap), the event is dropped
  with a warning
- **`send().await` with timeout for shutdown events** — session stop and
  flush use awaitable send for reliable delivery, but always wrapped in
  `tokio::time::timeout` to prevent blocking shutdown if the actor is
  stalled

## Database Queries

- **Always use parameterized queries** (`turso::params![]`) — never
  interpolate user data into SQL strings
- **Nullable columns**: decode as `row.get::<Option<T>>(idx)` with
  `.context()` — never use `.ok()` to convert errors to `None`, which
  hides type/decode errors and silently drops data. Only SQL `NULL`
  should produce `None`
- **Schema versions**: use `u32::try_from(i64_val)` for version numbers
  read from the DB — never `as u32`, which wraps on negative values

## Numeric Conversions for DB Params

- SQLite/turso stores integers as `i64`. When writing Rust `u64`
  counters (metrics), use `i64::try_from(val).unwrap_or(i64::MAX)` to
  clamp on overflow instead of `as i64` which wraps silently
- For `u32` → `i64` (e.g. PID), use `i64::from(val)` — lossless and
  compiler-checked

## Actor Patterns

- **Batching**: accumulate up to 50 events or 500ms timeout, then flush
  in a single transaction for ~100x better throughput vs per-event commits
- **Flush sentinel**: `StorageEvent::Flush` triggers final drain + exit
- **Error resilience**: individual event write failures warn but don't
  abort the batch — storage must never crash the daemon
- **Shutdown**: `StorageHandle::shutdown()` sends `Flush` (with timeout
  on the send itself), then awaits the actor with a 5s timeout before
  aborting. Both the send and the await are independently guarded
  against stalls

## Testing

- Actor tests must capture the `JoinHandle` from `tokio::spawn` and
  await it after sending `Flush` — never use `sleep()` for
  synchronization
- Use file-based tempdir DBs for actor tests (not `:memory:`) — in-memory
  turso doesn't share state across multiple `db.connect()` calls
- `ensure_data_dir()` enforces 0700 permissions — use a subdirectory
  within `tempdir()` for integration tests since tempdir creates with
  0755
