# ADR-0002: Daemon Architecture with Unix Sockets

**Status:** Accepted **Date:** 2025-10-25 **Decision Makers:** Project Team
**Technical Story:** Choose communication architecture between shell and
autocomplete engine

## Context

We need to decide how the shell integration communicates with the autocomplete
engine. The system must:

- Respond to completion requests in <10ms
- Handle concurrent requests from multiple shells
- Be reliable (no lost requests)
- Start quickly when first needed
- Use minimal resources when idle

### Requirements

- **Latency:** <10ms for daemon response (part of 20ms total budget)
- **Concurrency:** Support multiple terminal tabs/windows
- **Reliability:** No crashes or data loss
- **Resource Efficiency:** Low memory and CPU when idle
- **Simplicity:** Easy to debug and maintain

## Decision

We will use a **persistent daemon** communicating via **Unix domain sockets**
with a **JSON protocol**.

### Architecture Components

1. **Persistent Daemon** (`autocomplete-rs daemon`)
   - Runs in background
   - Listens on Unix socket at `/tmp/autocomplete-rs.sock`
   - Loads specs once at startup
   - Handles concurrent connections

2. **Unix Domain Socket**
   - Local IPC mechanism
   - No network overhead
   - File-based permissions

3. **JSON Protocol**
   - Request: `{"buffer": "git che", "cursor": 7}`
   - Response: `{"suggestions": [{"text": "checkout", "description": "..."}]}`

## Consequences

### Positive

**Performance:**

- Daemon stays warm → no startup cost per request
- Specs loaded once → instant lookups
- Unix sockets faster than TCP (no network stack)
- Sub-millisecond IPC latency

**Simplicity:**

- Single daemon serves all shells
- No complex IPC mechanisms
- JSON is human-readable for debugging
- Standard Unix patterns

**Resource Efficiency:**

- One process vs one per shell
- Shared memory for specs
- ~20MB total vs ~20MB per shell

**Reliability:**

- Process isolation (daemon crash doesn't affect shell)
- Auto-restart on first request if crashed
- Socket cleanup on daemon exit

### Negative

**Complexity:**

- Need daemon lifecycle management
- Socket cleanup on ungraceful exit
- Potential for socket permission issues
- Need auto-start logic

**Debugging:**

- Separate process to monitor
- Need logging infrastructure
- Inter-process debugging harder

**Platform Limitations:**

- Unix sockets not on Windows (would need named pipes)
- Socket path length limits on some systems

## Alternatives Considered

### Option 1: Direct Execution (No Daemon)

**How It Works:**

- Shell calls `autocomplete-rs complete "buffer"` each time
- Process starts, parses, responds, exits

**Pros:**

- Simpler (no daemon management)
- No background processes
- No IPC complexity

**Cons:**

- Startup cost every request (~5-10ms)
- Specs loaded every request (~100ms)
- Can't meet <20ms latency requirement
- Wasteful (600+ processes per day)

**Why Not Chosen:** Cannot meet performance requirements. Would need <5ms total
including startup and spec loading, which is impossible.

### Option 2: HTTP Server

**How It Works:**

- Daemon listens on `localhost:PORT`
- Shell makes HTTP requests

**Pros:**

- Well-known protocol
- Easy testing (curl)
- Cross-platform

**Cons:**

- Network stack overhead (~1-2ms)
- Need to allocate port
- Port conflicts possible
- More attack surface
- Heavier than Unix sockets

**Why Not Chosen:** Unnecessary overhead. Unix sockets are faster and simpler
for local IPC.

### Option 3: Embedded Library

**How It Works:**

- Autocomplete engine as shared library
- Shell loads library directly

**Pros:**

- Fastest possible (function calls)
- No IPC overhead
- Process integrated

**Cons:**

- Each shell loads specs separately
- Memory duplication (N \* 10MB)
- Shell-specific integration complexity
- Can't share state across shells
- Harder to update

**Why Not Chosen:** Memory inefficient and complex integration. Would need
different bindings for each shell.

### Option 4: Named Pipes (FIFO)

**How It Works:**

- Create named pipe
- Shell writes request, reads response

**Pros:**

- Simple Unix primitive
- No daemon needed (could be)

**Cons:**

- Blocking I/O harder to manage
- No built-in connection model
- Less flexible than sockets
- Still need daemon anyway

**Why Not Chosen:** Unix sockets provide same benefits with better semantics.

### Option 5: gRPC

**How It Works:**

- Daemon runs gRPC server
- Shell uses gRPC client

**Pros:**

- Strong typing (protobuf)
- Efficient binary protocol
- Good tooling

**Cons:**

- Massive overkill for local IPC
- Large dependency
- Slower than Unix sockets
- More complex debugging

**Why Not Chosen:** Over-engineered for our needs. JSON over Unix sockets is
simpler and faster for local use.

## Comparison Matrix

| Criterion      | Unix Socket Daemon | Direct Exec | HTTP Server | Embedded Lib | gRPC      |
| -------------- | ------------------ | ----------- | ----------- | ------------ | --------- |
| Latency        | <1ms ✅            | ~100ms ❌   | ~2ms ⚠️     | <0.1ms ✅    | ~2ms ⚠️   |
| Memory         | ~20MB ✅           | 0 idle ✅   | ~25MB ⚠️    | N\*10MB ❌   | ~30MB ❌  |
| Simplicity     | Medium ⚠️          | High ✅     | Medium ⚠️   | Low ❌       | Low ❌    |
| Debuggability  | Medium ⚠️          | High ✅     | High ✅     | Low ❌       | Medium ⚠️ |
| Cross-platform | Unix only ⚠️       | Yes ✅      | Yes ✅      | Complex ❌   | Yes ✅    |
| Startup Cost   | 0 ✅               | High ❌     | 0 ✅        | Medium ⚠️    | 0 ✅      |

## Implementation Details

### Socket Location

```text
/tmp/autocomplete-rs.sock
```

- Standard tmp location
- Auto-cleanup on reboot
- Check before creating (handle existing daemon)

### Protocol

```json
// Request
{
  "buffer": "git checkout -b ",
  "cursor": 18
}

// Response
{
  "suggestions": [
    {
      "text": "main",
      "description": "Create branch from main",
      "type": "argument"
    }
  ]
}
```

### Daemon Lifecycle

1. Shell checks if socket exists
2. If not, start daemon (`autocomplete-rs daemon &`)
3. Daemon creates socket, starts listening
4. Shell connects and sends request
5. Daemon responds
6. Connection closes
7. Daemon stays running for future requests

### Error Handling

- Socket connection timeout: 1 second
- Auto-restart daemon if not running
- Graceful degradation (no completions if daemon fails)
- Log errors for debugging

## Future Considerations

- **Windows Support:** Use named pipes (`\\.\pipe\autocomplete-rs`)
- **Multiple Users:** Include UID in socket path
- **Security:** Socket permissions (0600) prevent other users
- **Monitoring:** Health check endpoint

## References

- [Unix Domain Sockets](https://man7.org/linux/man-pages/man7/unix.7.html)
- [Tokio Unix Sockets](https://docs.rs/tokio/latest/tokio/net/struct.UnixListener.html)
- Amazon Q's issues with Accessibility API positioning bugs (motivation)

## Review Notes

Decision prioritizes:

1. Performance (hard requirement)
2. Resource efficiency (one daemon vs many)
3. Simplicity (standard Unix patterns)

The daemon architecture with Unix sockets hits the sweet spot of performance,
efficiency, and maintainability.
