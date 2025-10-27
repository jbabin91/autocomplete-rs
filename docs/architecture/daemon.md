# Daemon Architecture

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
│  │  - Parse CLI args                                      │ │
│  │  - Initialize logging                                  │ │
│  │  - Call daemon::start()                                │ │
│  └────────────────────────────────────────────────────────┘ │
│                           │                                  │
│                           ▼                                  │
│  ┌────────────────────────────────────────────────────────┐ │
│  │         Server (src/daemon/mod.rs)                     │ │
│  │                                                         │ │
│  │  ┌──────────────────────────────────────────────────┐ │ │
│  │  │  Tokio Runtime                                   │ │ │
│  │  │  - Async executor                                │ │ │
│  │  │  - Thread pool (default: # CPU cores)           │ │ │
│  │  └──────────────────────────────────────────────────┘ │ │
│  │                                                         │ │
│  │  ┌──────────────────────────────────────────────────┐ │ │
│  │  │  UnixListener                                    │ │ │
│  │  │  - Binds to socket path                         │ │ │
│  │  │  - Accepts connections                           │ │ │
│  │  │  - Spawns handler task per connection           │ │ │
│  │  └──────────────────────────────────────────────────┘ │ │
│  │                                                         │ │
│  │  ┌──────────────────────────────────────────────────┐ │ │
│  │  │  Connection Handler (per connection)            │ │ │
│  │  │  - Read JSON request                            │ │ │
│  │  │  - Parse and validate                           │ │ │
│  │  │  - Call parser                                  │ │ │
│  │  │  - Serialize response                           │ │ │
│  │  │  - Write JSON response                          │ │ │
│  │  └──────────────────────────────────────────────────┘ │ │
│  └────────────────────────────────────────────────────────┘ │
│                           │                                  │
│                           ▼                                  │
│  ┌────────────────────────────────────────────────────────┐ │
│  │            Parser (src/parser/mod.rs)                  │ │
│  │  - Tokenization                                        │ │
│  │  - Context analysis                                    │ │
│  │  - Spec matching                                       │ │
│  │  - Suggestion generation                               │ │
│  └────────────────────────────────────────────────────────┘ │
│                           │                                  │
│                           ▼                                  │
│  ┌────────────────────────────────────────────────────────┐ │
│  │         Spec Loader (src/specs/mod.rs)                │ │
│  │  - MessagePack embedded data                          │ │
│  │  - LRU cache (hot specs)                              │ │
│  │  - Lazy deserialization                               │ │
│  └────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## Lifecycle

### Startup

```rust
// src/main.rs
#[tokio::main]
async fn main() -> Result<()> {
    // 1. Parse arguments
    let cli = Cli::parse();

    // 2. Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // 3. Match command
    match cli.command {
        Commands::Daemon { socket } => {
            // 4. Start daemon
            daemon::start(&socket).await?;
        }
        // ... other commands
    }

    Ok(())
}
```

```rust
// src/daemon/mod.rs
pub async fn start(socket_path: &str) -> Result<()> {
    // 1. Remove stale socket if exists
    let _ = std::fs::remove_file(socket_path);

    // 2. Create Unix socket listener
    let listener = UnixListener::bind(socket_path)?;
    info!("Daemon listening on {}", socket_path);

    // 3. Set socket permissions (user-only)
    set_socket_permissions(socket_path)?;

    // 4. Initialize shared state
    let state = Arc::new(DaemonState::new());

    // 5. Accept loop
    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => {
                let state = Arc::clone(&state);
                // Spawn handler task
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, state).await {
                        error!("Connection error: {}", e);
                    }
                });
            }
            Err(e) => {
                error!("Accept error: {}", e);
            }
        }
    }
}
```

**Startup Time:** <5ms

- Socket creation: <1ms
- Permission setting: <1ms
- State initialization: <1ms
- Ready to accept: <2ms buffer

### Request Handling

```rust
async fn handle_connection(
    stream: UnixStream,
    state: Arc<DaemonState>
) -> Result<()> {
    // 1. Read request (with timeout)
    let request = tokio::time::timeout(
        Duration::from_millis(100),
        read_request(&stream)
    ).await??;

    debug!("Received request: {:?}", request);

    // 2. Validate request
    validate_request(&request)?;

    // 3. Parse buffer
    let suggestions = state.parser.parse(
        &request.buffer,
        request.cursor
    ).await?;

    // 4. Create response
    let response = Response { suggestions };

    // 5. Write response
    write_response(&stream, &response).await?;

    debug!("Sent {} suggestions", response.suggestions.len());

    Ok(())
}
```

**Request Time:** <10ms

- Read JSON: <1ms
- Parse buffer: <5ms
- Serialize response: <1ms
- Write response: <1ms
- Buffer: 2ms

### Shutdown

```rust
// Signal handler for graceful shutdown
tokio::signal::ctrl_c().await?;

info!("Shutting down daemon");

// 1. Stop accepting new connections
drop(listener);

// 2. Wait for in-flight requests (with timeout)
tokio::time::timeout(
    Duration::from_secs(5),
    wait_for_active_connections()
).await;

// 3. Clean up socket
std::fs::remove_file(socket_path)?;

info!("Daemon shut down cleanly");
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

**Shared State:**

```rust
pub struct DaemonState {
    // Parser is Send + Sync, can be shared
    parser: Parser,

    // Spec cache with interior mutability
    spec_cache: Arc<Mutex<LruCache<String, CompletionSpec>>>,

    // Metrics (atomic counters)
    total_requests: AtomicU64,
    active_connections: AtomicU32,
}
```

**Lock Strategy:**

- Minimize lock contention
- Use read-write locks where appropriate
- Async-aware locks (tokio::sync::Mutex)
- Hold locks for shortest time possible

### Backpressure

**Connection Limits:**

```rust
const MAX_CONCURRENT_CONNECTIONS: usize = 100;

let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_CONNECTIONS));

loop {
    // Acquire permit (blocks if at limit)
    let permit = semaphore.clone().acquire_owned().await?;

    let (stream, _) = listener.accept().await?;

    tokio::spawn(async move {
        let _permit = permit; // Hold until handler completes
        handle_connection(stream, state).await
    });
}
```

**Request Size Limits:**

- Max request size: 100KB
- Max buffer length: 10,000 chars
- Max cursor position: buffer length

## Protocol

### Request Format

```json
{
  "buffer": "git checkout -b ",
  "cursor": 18
}
```

**Schema:**

```rust
#[derive(Deserialize)]
struct Request {
    /// The complete command buffer
    buffer: String,

    /// Cursor position (0-indexed byte offset)
    cursor: usize,
}
```

**Validation:**

- `buffer` must be valid UTF-8
- `buffer.len()` <= 10,000
- `cursor` <= `buffer.len()`

### Response Format

```json
{
  "suggestions": [
    {
      "text": "feature/new",
      "description": "Create new feature branch",
      "type": "argument"
    },
    {
      "text": "main",
      "description": "Branch from main",
      "type": "argument"
    }
  ]
}
```

**Schema:**

```rust
#[derive(Serialize)]
struct Response {
    suggestions: Vec<Suggestion>,
}

#[derive(Serialize)]
struct Suggestion {
    /// Text to insert
    text: String,

    /// Description (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,

    /// Type: "command", "option", "argument"
    #[serde(rename = "type")]
    suggestion_type: String,
}
```

### Error Handling

**Error Response:**

```json
{
  "error": "Invalid cursor position",
  "code": "INVALID_CURSOR"
}
```

**Error Codes:**

- `INVALID_REQUEST` - Malformed JSON
- `INVALID_BUFFER` - Buffer validation failed
- `INVALID_CURSOR` - Cursor out of bounds
- `PARSE_ERROR` - Parser failure
- `INTERNAL_ERROR` - Unexpected error

**Strategy:**

- Return empty suggestions on recoverable errors
- Return error JSON on client errors
- Log internal errors, return generic error

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
const MAX_CONCURRENT_CONNECTIONS: usize = 100;
const MAX_REQUEST_SIZE: usize = 100 * 1024; // 100KB
const MAX_BUFFER_LEN: usize = 10_000;
const REQUEST_TIMEOUT: Duration = Duration::from_millis(100);
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

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_handle_valid_request() {
        let state = Arc::new(DaemonState::new());
        let request = Request {
            buffer: "git checkout ".to_string(),
            cursor: 13,
        };

        // Mock socket with request
        let (mut client, server) = UnixStream::pair()?;

        // Write request
        write_json(&client, &request).await?;

        // Handle connection
        tokio::spawn(async move {
            handle_connection(server, state).await.unwrap();
        });

        // Read response
        let response: Response = read_json(&mut client).await?;

        // Verify
        assert!(!response.suggestions.is_empty());
    }
}
```

### Integration Tests

```rust
// tests/daemon_test.rs
#[tokio::test]
async fn test_daemon_startup_and_shutdown() {
    let socket = "/tmp/test-daemon.sock";

    // Start daemon
    let handle = tokio::spawn(async move {
        daemon::start(socket).await
    });

    // Wait for startup
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect
    let stream = UnixStream::connect(socket).await?;
    assert!(stream.is_ok());

    // Shutdown
    handle.abort();

    // Verify socket cleaned up
    assert!(!Path::new(socket).exists());
}
```

### Load Tests

```rust
#[tokio::test]
async fn test_concurrent_requests() {
    let socket = start_test_daemon().await;

    // Spawn 100 concurrent clients
    let mut handles = vec![];
    for _ in 0..100 {
        let handle = tokio::spawn(async move {
            let mut stream = UnixStream::connect(socket).await?;
            let request = test_request();
            write_json(&stream, &request).await?;
            let response: Response = read_json(&mut stream).await?;
            Ok(response)
        });
        handles.push(handle);
    }

    // Wait for all
    for handle in handles {
        let response = handle.await??;
        assert!(!response.suggestions.is_empty());
    }
}
```

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
- [TUI Architecture](tui.md) - UI rendering
- [ADR-0002: Daemon Architecture](../adr/0002-daemon-architecture.md) - Design
  decision
