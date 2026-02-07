# Daemon Architecture

> **Status:** Core daemon implemented (Phase 1). Parser integration and spec
> loading are Phase 2.

This document details the design and implementation of the autocomplete-rs
daemon.

## Overview

The daemon is a persistent background process that:

- Listens on a Unix domain socket
- Handles concurrent completion requests
- Coordinates parser and spec lookup
- Maintains spec cache for performance
- Runs with <50MB memory footprint

## Design Principles

1. **Single Responsibility:** Coordinate completion requests, delegate to
   specialized modules
2. **Async I/O:** Non-blocking socket handling with Tokio
3. **Stateless:** Each request is independent
4. **Fail-Safe:** Errors don't crash the daemon
5. **Observable:** Debug logging for troubleshooting

## Architecture

### Component Diagram

```text
┌─────────────────────────────────────────────────────────────┐
│                    Daemon Process                            │
│                                                               │
│  ┌────────────────────────────────────────────────────────┐ │
│  │              Main (src/main.rs)                        │ │
│  │  - Parse CLI args (Clap derive + env vars)            │ │
│  │  - Initialize logging (tracing)                       │ │
│  │  - Call daemon::start(socket_path)                    │ │
│  └────────────────────────────────────────────────────────┘ │
│                           │                                  │
│                           ▼                                  │
│  ┌────────────────────────────────────────────────────────┐ │
│  │     Daemon Facade (src/daemon/mod.rs)                 │ │
│  │  - start() / start_with_engine()                      │ │
│  │  - Acquires PID file (single-instance enforcement)    │ │
│  │  - Binds UnixListener, delegates to server::run()     │ │
│  └────────────────────────────────────────────────────────┘ │
│                           │                                  │
│                           ▼                                  │
│  ┌────────────────────────────────────────────────────────┐ │
│  │     Server (src/daemon/server.rs)                     │ │
│  │  - Accept loop with tokio::select! (biased)          │ │
│  │  - CancellationToken + Ctrl+C signal handling        │ │
│  │  - Semaphore backpressure (100 max connections)      │ │
│  │  - JoinSet for task tracking + 5s drain timeout      │ │
│  │  - Socket permissions (0o600)                        │ │
│  └────────────────────────────────────────────────────────┘ │
│                           │                                  │
│                           ▼                                  │
│  ┌────────────────────────────────────────────────────────┐ │
│  │     Handler (src/daemon/handler.rs)                   │ │
│  │  - Per-connection request handling                    │ │
│  │  - 1s read timeout, 100KB size limit                 │ │
│  │  - DaemonMessage envelope (Complete | Shutdown)      │ │
│  │  - Bare CompletionRequest fallback (backward compat) │ │
│  │  - ConnectionGuard RAII for active_connections       │ │
│  └────────────────────────────────────────────────────────┘ │
│                           │                                  │
│                           ▼                                  │
│  ┌────────────────────────────────────────────────────────┐ │
│  │   CompletionEngine (src/engine.rs)                    │ │
│  │  - Trait: Send + Sync, consumed via Arc<dyn ...>     │ │
│  │  - StubEngine returns empty suggestions (Phase 1)    │ │
│  │  - Parser will implement this trait (Phase 2)        │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                               │
│  ┌────────────────────────────────────────────────────────┐ │
│  │     Shared State (src/daemon/state.rs)                │ │
│  │  - Arc<dyn CompletionEngine>                         │ │
│  │  - Arc<Semaphore> (100 permits)                      │ │
│  │  - CancellationToken (cross-task shutdown)           │ │
│  │  - AtomicU64 total_requests, AtomicU64 active_conns  │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                               │
│  ┌────────────────────────────────────────────────────────┐ │
│  │     PID File (src/daemon/pid.rs)                      │ │
│  │  - RAII PidFile with Drop cleanup                    │ │
│  │  - Derives path: *.sock → *.pid                      │ │
│  │  - kill(pid, 0) liveness check (handles EPERM)       │ │
│  └────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## Lifecycle

### Startup

```rust
// src/daemon/mod.rs — thin facade
pub async fn start(socket_path: &str) -> Result<()> {
    start_with_engine(socket_path, Arc::new(StubEngine)).await
}

pub async fn start_with_engine(
    socket_path: &str,
    engine: Arc<dyn CompletionEngine>,
) -> Result<()> {
    let path = Path::new(socket_path);

    // 1. Acquire PID file (single-instance enforcement)
    let _pid_file = PidFile::acquire(path)?;

    // 2. Remove stale socket + bind
    let _ = std::fs::remove_file(path);
    let listener = UnixListener::bind(path)?;

    // 3. Create shared state (DaemonState is Clone, not wrapped in Arc)
    let state = DaemonState::new(engine);

    // 4. Run server accept loop (blocks until shutdown)
    server::run(listener, state, path).await
}
```

**Startup Time:** <5ms

- PID file acquire: <1ms
- Socket creation: <1ms
- Permission setting: <1ms
- State initialization: <1ms
- Ready to accept: <1ms buffer

### Request Handling

```rust
// src/daemon/handler.rs — generic over AsyncRead/AsyncWrite for testability
pub async fn handle_connection<R, W>(
    reader: R,
    mut writer: W,
    state: &DaemonState,
    conn_id: u64,
) -> Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let _guard = state.connection_guard();

    // 1. Read with 1s timeout + 100KB size limit
    // 2. Parse: try DaemonMessage envelope first; if JSON has a "type" field
    //    but fails, return error (don't fall back to bare CompletionRequest).
    //    Only bare JSON without "type" gets backward-compat fallback.
    // 3. Validate request fields
    // 4. Call state.engine.complete()
    // 5. Write response with 1s timeout (prevents stalled clients from
    //    blocking handler tasks indefinitely)

    // Shutdown variant cancels the shared CancellationToken:
    //   state.cancel.cancel();
    //   write_json(&mut writer, &ShutdownAck { status: "shutting_down" })
}
```

**Request Time:** <10ms

- Read JSON: <1ms
- Parse/validate: <1ms
- Engine complete: <5ms (currently instant with StubEngine)
- Serialize response: <1ms
- Write response: <1ms

### Shutdown

Shutdown is triggered by either Ctrl+C (SIGINT) or a `DaemonMessage::Shutdown`
sent over the socket. Both paths cancel the shared `CancellationToken`.

```rust
// src/daemon/server.rs — signal pinned once, biased select in accept loop
let mut sigint = std::pin::pin!(signal::ctrl_c());

loop {
    tokio::select! {
        biased;  // Check cancellation first
        _ = state.cancel.cancelled() => break,  // Shutdown message
        result = &mut sigint => {
            state.cancel.cancel();
            break;
        }
        result = listener.accept() => { /* handle connection */ }
    }
}

// After loop exits:
// 1. Drain in-flight tasks (5s timeout via JoinSet)
// 2. abort_all() + join remaining tasks on timeout
// 3. Clean up socket file
// 4. PID file cleaned up automatically via Drop
```

## Concurrency

### Threading Model

**Tokio Runtime:**

- Work-stealing scheduler
- Thread pool size: `num_cpus` (default)
- Each task runs on any thread
- Efficient for I/O-bound workload

**Connection Handling:**

- Each connection = one async task
- Tasks are lightweight (~2KB stack)
- Can handle 1000+ concurrent connections
- No thread-per-connection overhead

### Synchronization

**Shared State (implemented):**

```rust
// src/daemon/state.rs
pub struct DaemonState {
    pub engine: Arc<dyn CompletionEngine>,
    pub semaphore: Arc<Semaphore>,
    pub cancel: CancellationToken,
    pub total_requests: Arc<AtomicU64>,
    pub active_connections: Arc<AtomicU64>,
}
```

**Lock Strategy:**

- No locks needed currently — engine is `Send + Sync` via trait bound
- Semaphore for connection limiting (no lock contention)
- Atomics for metrics (lock-free)
- Future spec cache will use `tokio::sync::Mutex` with minimal hold times

### Backpressure

**Connection Limits (implemented):**

```rust
// src/daemon/server.rs — non-blocking semaphore in accept loop
match state.semaphore.clone().try_acquire_owned() {
    Ok(permit) => {
        tasks.spawn(async move {
            let _permit = permit; // Held until handler completes
            handle_connection(reader, writer, state, conn_id).await
        });
    }
    Err(_) => {
        warn!("Connection limit reached, dropping connection");
        drop(stream);
    }
}
```

Uses `try_acquire_owned()` (non-blocking) instead of `acquire_owned()` to avoid
blocking the accept loop when at capacity. Excess connections are dropped
immediately rather than queued.

**Request Size Limits:**

- Max request size: 100KB
- Max buffer length: 10,000 chars
- Max cursor position: buffer length

## Protocol

### DaemonMessage Envelope

All requests should use the `DaemonMessage` tagged enum envelope. Bare
`CompletionRequest` (without `"type"` field) is also accepted for backward
compatibility.

```json
// Complete request (preferred)
{ "type": "complete", "buffer": "git checkout -b ", "cursor": 18 }

// Shutdown request
{ "type": "shutdown" }

// Bare request (backward compat — no "type" field)
{ "buffer": "git checkout -b ", "cursor": 18 }
```

**Schema:**

```rust
// src/protocol.rs
#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonMessage {
    Complete(CompletionRequest),
    Shutdown,
}

#[derive(Serialize, Deserialize)]
pub struct CompletionRequest {
    pub buffer: String,
    pub cursor: usize,
    #[serde(default = "default_version")]
    pub version: u8,  // defaults to PROTOCOL_VERSION (1)
}
```

**Validation:**

- `buffer` must be valid UTF-8
- `buffer.len()` <= 10,000 (`MAX_BUFFER_LEN`)
- `cursor` <= `buffer.len()`
- Total request size <= 100KB (`MAX_REQUEST_SIZE`)

### Response Format

**Completion response:**

```json
{
  "suggestions": [
    {
      "text": "feature/new",
      "description": "Create new feature branch"
    }
  ]
}
```

**Shutdown acknowledgment:**

```json
{ "status": "shutting_down" }
```

**Schema:**

```rust
// src/protocol.rs
#[derive(Serialize, Deserialize)]
pub struct CompletionResponse {
    pub suggestions: Vec<Suggestion>,
}

#[derive(Serialize, Deserialize)]
pub struct Suggestion {
    pub text: String,
    pub description: String,
}

#[derive(Serialize, Deserialize)]
pub struct ShutdownAck {
    pub status: String,
}
```

### Error Handling

**Error Response:**

```json
{ "error": "cursor position 99 exceeds buffer length 2" }
```

```rust
// src/protocol.rs
#[derive(Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}
```

Validation errors are human-readable strings from `thiserror`-derived
`ValidationError` variants. No error codes — the error message is the contract.

## Performance Optimization

### Zero-Copy Parsing

```rust
// Avoid copying buffer string
pub fn parse(&self, buffer: &str, cursor: usize) -> Result<Vec<Suggestion>> {
    // Work with string slices, not copies
    let before_cursor = &buffer[..cursor];
    let after_cursor = &buffer[cursor..];

    // ...
}
```

### Spec Caching

```rust
pub struct SpecCache {
    cache: Mutex<LruCache<String, Arc<CompletionSpec>>>,
}

impl SpecCache {
    pub async fn get(&self, name: &str) -> Result<Arc<CompletionSpec>> {
        // Fast path: cache hit
        {
            let cache = self.cache.lock().await;
            if let Some(spec) = cache.peek(name) {
                return Ok(Arc::clone(spec));
            }
        }

        // Slow path: load and cache
        let spec = self.load_from_embedded(name)?;
        let spec = Arc::new(spec);

        let mut cache = self.cache.lock().await;
        cache.put(name.to_string(), Arc::clone(&spec));

        Ok(spec)
    }
}
```

**Cache Strategy:**

- LRU eviction (keep hot specs)
- Default size: 50 specs (~5MB)
- Arc for cheap cloning
- Lock-free reads with Arc

### Connection Pooling

**Current:** Create new parser context per request (cheap)

**Future:** Connection pooling for parser state

```rust
struct ParserPool {
    pool: Pool<Parser>,
}

// Reuse parser instances
let parser = pool.get().await?;
let suggestions = parser.parse(buffer, cursor)?;
pool.return(parser);
```

## Monitoring

### Metrics

```rust
pub struct Metrics {
    // Request counters
    total_requests: AtomicU64,
    successful_requests: AtomicU64,
    failed_requests: AtomicU64,

    // Timing histograms
    request_duration: Histogram,
    parse_duration: Histogram,

    // Resource usage
    active_connections: AtomicU32,
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
}
```

**Exposure:**

- Log metrics periodically (debug mode)
- Expose via `/metrics` endpoint (future)
- Prometheus format (future)

### Logging

```rust
// Request tracing
#[instrument(skip(stream, state))]
async fn handle_connection(
    stream: UnixStream,
    state: Arc<DaemonState>
) -> Result<()> {
    let start = Instant::now();

    let request = read_request(&stream).await?;
    debug!("Request: buffer_len={}, cursor={}",
           request.buffer.len(), request.cursor);

    let suggestions = state.parser.parse(&request.buffer, request.cursor).await?;
    debug!("Generated {} suggestions", suggestions.len());

    write_response(&stream, &Response { suggestions }).await?;

    let duration = start.elapsed();
    info!("Request handled in {:?}", duration);

    Ok(())
}
```

**Log Levels:**

- `ERROR`: Unexpected failures
- `WARN`: Recoverable errors
- `INFO`: Request completed
- `DEBUG`: Request details
- `TRACE`: Verbose internals

## Security

### Socket Permissions

```rust
use std::os::unix::fs::PermissionsExt;

fn set_socket_permissions(path: &str) -> Result<()> {
    let metadata = std::fs::metadata(path)?;
    let mut permissions = metadata.permissions();

    // Set to 0600 (user read/write only)
    permissions.set_mode(0o600);

    std::fs::set_permissions(path, permissions)?;

    Ok(())
}
```

**Protection:**

- Only socket owner can connect
- No other users can read/write
- Standard Unix DAC

### Input Validation

```rust
fn validate_request(request: &Request) -> Result<()> {
    // Check buffer length
    if request.buffer.len() > MAX_BUFFER_LEN {
        return Err(Error::BufferTooLong);
    }

    // Check cursor bounds
    if request.cursor > request.buffer.len() {
        return Err(Error::InvalidCursor);
    }

    // Check valid UTF-8 (already enforced by String type)

    Ok(())
}
```

### Resource Limits

```rust
// src/protocol.rs
pub const MAX_BUFFER_LEN: usize = 10_000;
pub const MAX_REQUEST_SIZE: u64 = 100 * 1024; // 100KB

// src/daemon/state.rs
pub const MAX_CONCURRENT_CONNECTIONS: usize = 100;

// src/daemon/handler.rs
const READ_TIMEOUT: Duration = Duration::from_secs(1);
```

**DOS Protection:**

- Limit concurrent connections
- Timeout slow requests
- Limit request size
- Limit buffer length

## Error Recovery

### Panic Recovery

```rust
tokio::spawn(async move {
    // Catch panics at task boundary
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        handle_connection(stream, state).await
    }));

    match result {
        Ok(Ok(_)) => info!("Request handled successfully"),
        Ok(Err(e)) => error!("Request error: {}", e),
        Err(panic) => {
            error!("Handler panicked: {:?}", panic);
            // Daemon continues running
        }
    }
});
```

**Strategy:**

- Catch panics per-connection
- Log panic details
- Continue serving other requests
- Don't bring down daemon

### Connection Errors

```rust
match listener.accept().await {
    Ok((stream, _)) => {
        // Spawn handler
    }
    Err(e) if e.kind() == ErrorKind::WouldBlock => {
        // Expected with non-blocking I/O
        continue;
    }
    Err(e) => {
        error!("Accept failed: {}", e);
        // Log and continue, don't crash
        continue;
    }
}
```

### Graceful Degradation

**If spec loading fails:**

- Return empty suggestions
- Log error
- Continue serving other requests

**If parser crashes:**

- Catch panic
- Return error response
- Continue serving

## Testing

### Unit Tests (34 inline tests)

Tests use `#[cfg(test)]` modules within each source file. Handler tests use
`tokio::net::UnixStream::pair()` for testability without real sockets.

- `protocol` (12): serde round-trips, validation edge cases, `DaemonMessage`
  variants, backward compat parsing
- `handler` (8): valid request, malformed JSON, cursor OOB, buffer too long,
  empty disconnect, shutdown message, bare request fallback
- `pid` (8): path derivation, live/dead process detection, acquire/release,
  stale cleanup, double-acquire rejection
- `state` (4): connection guard, metrics, semaphore permits
- `engine` (2): stub returns empty, trait object behind `Arc`

### Integration Tests (7 tests in `tests/daemon_integration.rs`)

Real daemon on temp socket paths using `AtomicU64` counter for uniqueness:

- `start_connect_complete` — Start daemon, send request, verify response
- `shutdown_message_clean_exit` — Send shutdown, verify clean exit + cleanup
- `socket_permissions` — Verify socket is `0o600` (owner-only)
- `concurrent_connections` — 10 simultaneous connections
- `malformed_json_returns_error` — Error response for bad input
- `envelope_and_bare_request_both_work` — Both `DaemonMessage` and bare request
- `pid_file_path_derivation` — PID file path from socket path

**Test pattern:** Tests use an atomic counter (not timestamps) for unique temp
socket paths. This avoids collisions even when tests run as parallel threads in
one process (`cargo test`) or as separate processes (`cargo nextest`).

## Future Enhancements

### HTTP Endpoint (Phase 3+)

Optionally expose HTTP for testing/debugging:

```rust
// Start HTTP server alongside Unix socket
let http_server = warp::serve(routes)
    .run(([127, 0, 0, 1], 3000));

tokio::select! {
    _ = unix_server => {}
    _ = http_server => {}
}
```

**Endpoints:**

- `POST /complete` - Completion request
- `GET /health` - Health check
- `GET /metrics` - Prometheus metrics
- `GET /specs` - List available specs

### Spec Hot Reloading (Development)

Watch specs directory and reload on change:

```rust
let watcher = notify::watcher()?;
watcher.watch("specs/", RecursiveMode::Recursive)?;

loop {
    match watcher.recv() {
        Ok(event) => {
            info!("Spec changed: {:?}", event);
            state.spec_cache.clear();
        }
        Err(e) => error!("Watch error: {}", e),
    }
}
```

### Connection Pooling

Reuse connections from shell integration:

```rust
// Keep connection open, reuse
let conn = Connection::new(socket)?;

loop {
    // On completion request
    conn.send_request(request)?;
    let response = conn.receive_response()?;
}
```

**Benefits:**

- Eliminate connection overhead
- Reduce latency by ~0.5ms

**Trade-offs:**

- More complex client code
- Need keep-alive logic
- Handle connection drops

## Related Documents

- [Architecture Overview](overview.md) - System architecture
- [Parser Architecture](parser.md) - Parser design
- [Inline Dropdown](tui.md) - UI rendering (planned)
- [ADR-0002: Daemon Architecture](../adr/0002-daemon-architecture.md) - Design
  decision
