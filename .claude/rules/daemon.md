---
paths:
  - 'src/daemon/**'
---

# Daemon Development Rules

## IPC Protocol

- Line-based JSON protocol over Unix domain sockets (one JSON object per line, terminated with `\n`)
- Request type: `CompletionRequest { buffer: String, cursor: usize, version: u8 }`
- Response type: `CompletionResponse { suggestions: Vec<Suggestion> }`
- Error type: `ErrorResponse { error: String }` — returned for malformed JSON
- Protocol version field exists for future compatibility (default: 1)

## Async Patterns

- Tokio runtime with `tokio::spawn` for per-connection tasks
- `tokio::select!` for multiplexing connections + shutdown signal (Ctrl+C)
- Stream split into reader/writer: `stream.into_split()` with `BufReader` for line reading
- Individual connection failures must NOT crash the daemon

## Socket Lifecycle

- Default path: `/tmp/autocomplete-rs.sock` (override via `AUTOCOMPLETE_RS_SOCKET` env var)
- Remove stale socket file on startup before binding
- Clean up socket file on graceful shutdown
- Daemon must survive client disconnects and malformed requests

## Performance Budget

- Daemon startup: <5ms
- IPC round-trip: <1ms
- Must handle concurrent connections (no blocking on individual requests)
