# Daemon Development Rules

## Architecture

- **Engine trait** (`src/engine.rs`): `CompletionEngine` trait at crate root — daemon consumes via `Arc<dyn CompletionEngine>`. Enables single-process path without the daemon.
- **Protocol** (`src/protocol.rs`): Shared types at crate root — both daemon and CLI client import from here.
- **Daemon modules**: `mod.rs` (facade), `server.rs` (accept loop), `handler.rs` (per-connection), `state.rs` (shared state), `pid.rs` (single-instance).
- **Overlay integration**: `DaemonState` carries `Option<OverlayChannel>`.
  `OverlayChannel` wraps `std::sync::mpsc::Sender<OverlayMessage>` +
  `WakeFn` (`Arc<dyn Fn() + Send + Sync>`) to keep winit imports out of
  daemon code. Handler forwards completions and shutdown signals to the
  overlay via this channel.

## Entry Points

- `start_with_engine()` — pure async, no UI. Used by tests and headless mode.
- `start_with_overlay()` — winit on main thread + Tokio on background thread.
  Used by the `daemon` CLI command. Both delegate to `run_daemon()` (shared
  startup/shutdown logic) to avoid duplication.
- `start()` — convenience wrapper for `start_with_engine()` with `StubEngine`.

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

- Default path: `~/.autocomplete-rs/daemon.sock` (override via `AUTOCOMPLETE_RS_SOCKET` env var)
- PID file for single-instance enforcement (`*.sock` → `*.pid`) — uses atomic `create_new(true)` to prevent TOCTOU races
- Socket permissions set to `0o600` after bind
- Remove stale socket file on startup before binding
- Clean up socket + PID file on graceful shutdown (PID file via RAII `Drop`)
- Daemon must survive client disconnects and malformed requests

## Request Tracing

- Every non-empty read gets a `request_id` (UUID v4) recorded on the
  tracing span — generate it **before** parsing, not after, so parse
  errors and truncated-request diagnostics include the ID for correlation
- `DaemonState` carries `logging::Mode` to control buffer redaction.
  The handler checks `should_redact(&state.mode)` before logging buffers
- Lifecycle debug logs: `"request received"` (with buffer + cursor) and
  `"response sent"` (with suggestion count) bracket the engine call
- Pre-read events (read timeout, empty disconnect) keep `request_id: None`
  since there's no data to correlate yet

## Storage Integration

- Storage is optional — daemon starts in degraded mode if init fails
- `DaemonState` carries `Option<StorageEventSender>` and `session_id`
- Hot-path events emitted via `emit_storage_event()` using `try_send()`
  (never awaits — non-blocking fire-and-forget with warning on failure).
  When storage is `None` (degraded mode), the call is a silent no-op
- Shutdown-path events (`SessionStop`) use `send().await` with timeout
  for reliable delivery — sessions should not remain "running" in the DB
- Session lifecycle: `SessionStart` before accept loop, `SessionStop`
  after, both correlated by `session_id` (UUID v4)
- Session `mode` is determined by `logging::detect_mode()` — reuses the
  same logic as the logging subsystem (production/development/troubleshooting).
  `logging::Mode` implements `Display` for serialization
- Metrics snapshots emitted every 60 seconds from the server accept loop,
  using `state.session_id` (no separate session_id parameter)
- Diagnostic events emitted on handler error paths (timeout, protocol,
  validation) with privacy redaction applied before storage
- Session stop event sent directly on `handle.sender` (not through
  `DaemonState`) because state is moved into `server::run()`

## Performance Budget

- Daemon startup: <5ms
- IPC round-trip: <1ms
- Must handle concurrent connections (no blocking on individual requests)
