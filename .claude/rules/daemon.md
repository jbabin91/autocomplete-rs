---
paths:
  - 'src/daemon/**'
  - 'src/protocol.rs'
  - 'src/engine.rs'
---

# Daemon Development Rules

## Architecture

- **Engine trait** (`src/engine.rs`): `CompletionEngine` trait at crate root — daemon consumes via `Arc<dyn CompletionEngine>`. Enables single-process path without the daemon.
- **Protocol** (`src/protocol.rs`): Shared types at crate root — both daemon and CLI client import from here.
- **Daemon modules**: `mod.rs` (facade), `server.rs` (accept loop), `handler.rs` (per-connection), `state.rs` (shared state), `pid.rs` (single-instance).

## IPC Protocol

- Line-based JSON protocol over Unix domain sockets (one JSON object per line, terminated with `\n`)
- **Envelope format**: `DaemonMessage` tagged enum with `"type"` field:
  - `{"type":"complete","buffer":"...","cursor":N}` — completion request
  - `{"type":"shutdown"}` — graceful shutdown request
- **Backward compatibility**: Bare `CompletionRequest` without `"type"` field is accepted (handler falls back). If JSON has a `"type"` field, the handler distinguishes unknown types ("unknown message type") from known types with invalid payloads ("invalid payload for message type") — prevents masking protocol bugs.
- **Dog-food the primary protocol**: The CLI client (`complete_command`, `stop_daemon`) must use `DaemonMessage` envelope format, not bare requests. The backward-compat path exists only for third-party integrations — our own code should exercise the primary code path.
- Response types:
  - `CompletionResponse { suggestions: Vec<Suggestion> }` — success
  - `ErrorResponse { error: String }` — validation failure or malformed JSON
  - `ShutdownAck { status: "shutting_down" }` — shutdown acknowledgement
- Protocol version field exists for future compatibility (default: 1)
- Request validation: buffer max 10,000 chars, cursor must be ≤ buffer length, max request size 100KB

## Async Patterns

- Tokio runtime with `tokio::spawn` for per-connection tasks tracked in `JoinSet`
- `tokio::select!` with biased polling: cancellation → Ctrl+C → accept. Signal future (`ctrl_c()`) is created once and pinned outside the loop — not recreated per iteration.
- `CancellationToken` (from tokio-util) for cross-task shutdown coordination
- Graceful shutdown drains in-flight tasks with a 5s timeout, then `abort_all()` + `join_next()` to observe cancelled tasks
- Stream split into reader/writer: `stream.into_split()` with `BufReader` for line reading
- Individual connection failures must NOT crash the daemon
- Semaphore-based backpressure (100 concurrent connections max)
- 1-second read timeout per connection, 1-second write timeout per response
- Always handle zero-byte `read_line` as a separate case (EOF/disconnect) — do not fall through to JSON parsing, which would give a confusing error

## Socket Lifecycle

- Default path: `/tmp/autocomplete-rs.sock` (override via `AUTOCOMPLETE_RS_SOCKET` env var)
- PID file for single-instance enforcement (`*.sock` → `*.pid`) — uses atomic `create_new(true)` to prevent TOCTOU races
- Socket permissions set to `0o600` after bind
- Remove stale socket file on startup before binding
- Clean up socket + PID file on graceful shutdown (PID file via RAII `Drop`)
- Daemon must survive client disconnects and malformed requests

## Storage Integration

- Storage is optional — daemon starts in degraded mode if init fails
- `DaemonState` carries `Option<StorageEventSender>` and `session_id`
- Events emitted via `emit_storage_event()` using `try_send()` (never
  awaits — non-blocking fire-and-forget with warning on failure)
- Session lifecycle: `SessionStart` before accept loop, `SessionStop`
  after, both correlated by `session_id` (UUID v4)
- Metrics snapshots emitted every 60 seconds from the server accept loop
- Diagnostic events emitted on handler error paths (timeout, protocol,
  validation) with privacy redaction applied before storage
- Session stop event sent directly on `handle.sender` (not through
  `DaemonState`) because state is moved into `server::run()`

## Performance Budget

- Daemon startup: <5ms
- IPC round-trip: <1ms
- Must handle concurrent connections (no blocking on individual requests)
