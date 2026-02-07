---
paths:
  - 'src/storage/**'
---

# Storage Layer Rules

## Architecture

- **turso** (pure-Rust SQLite rewrite) for local-only embedded storage —
  no cmake or C compilation required for `cargo install`. Note:
  `turso_sdk_kit` declares `bindgen` as a build-dep but has `build = false`
  and no `build.rs`, so it never actually runs
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

## turso API Notes

Key differences from the `libsql` crate (turso is API-compatible but
not identical):

- `Builder::new_local()` takes `&str` not `&Path` — convert with
  `path.to_str().context("database path is not valid UTF-8")?`
- `Transaction` has a lifetime: `turso::transaction::Transaction<'conn>`
  (not `turso::Transaction`, which is an empty struct at the crate root)
- `conn.transaction()` requires `&mut Connection` — use
  `conn.unchecked_transaction()` for shared `&Connection` (e.g. in the
  actor which owns a non-mut connection)
- `Transaction<'_>` implements `Deref<Target=Connection>`, so `execute`
  / `query` work directly on `&tx`

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
- `ensure_data_dir()` rejects group/other access (checks `perms & 0o077`)
  and creates new directories with 0700 — use a subdirectory within
  `tempdir()` for integration tests since tempdir creates with 0755
- Permission tests should assert exact `0o700` for directories created
  by `ensure_data_dir()`, not just `mode & 0o077 == 0`
