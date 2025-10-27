# Architecture Overview

This document provides a high-level overview of autocomplete-rs system
architecture.

## Design Goals

1. **Fast:** <20ms total latency from trigger to display
2. **Accurate:** Zero positioning bugs (unlike Amazon Q)
3. **Universal:** Works across all terminals and shells
4. **Lightweight:** <50MB memory, <10MB binary
5. **Reliable:** No crashes, graceful degradation
6. **Maintainable:** Clear separation of concerns

## System Architecture

### High-Level View

```text
┌─────────────────────────────────────────────────────────────┐
│                        User Terminal                         │
│  ┌────────────────────────────────────────────────────────┐ │
│  │  Shell (zsh/bash/fish)                                 │ │
│  │  ┌──────────────────────────────────────────────────┐  │ │
│  │  │  Shell Integration (ZLE widget / readline)      │  │ │
│  │  │  - Captures buffer & cursor                      │  │ │
│  │  │  - Sends to daemon via Unix socket               │  │ │
│  │  │  - Renders UI in terminal                        │  │ │
│  │  └──────────────────────────────────────────────────┘  │ │
│  └────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ Unix Socket
                              │ JSON Protocol
                              ▼
┌─────────────────────────────────────────────────────────────┐
│              Persistent Daemon Process                       │
│                                                               │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐     │
│  │   Server    │───▶│   Parser    │───▶│   Specs     │     │
│  │  (Tokio)    │    │  (Analyze)  │    │  (Lookup)   │     │
│  └─────────────┘    └─────────────┘    └─────────────┘     │
│        │                   │                   │             │
│        ▼                   ▼                   ▼             │
│  [Connection]        [Tokenize]          [MessagePack]      │
│  [Concurrent]        [Context]           [LRU Cache]        │
│  [Async I/O]         [Match]             [600+ Specs]       │
└─────────────────────────────────────────────────────────────┘
```

### Component Interaction

```text
User Types: "git che" + Alt+Space
       │
       ▼
┌──────────────────────────────────────────────────────────┐
│ 1. SHELL INTEGRATION (shell-integration/zsh.zsh)        │
│    - ZLE captures: buffer="git che", cursor=7           │
│    - Creates JSON: {"buffer":"git che","cursor":7}      │
│    - Opens Unix socket: /tmp/autocomplete-rs.sock       │
│    - Sends request                                       │
└──────────────────────────────────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────────────────────────┐
│ 2. DAEMON SERVER (src/daemon/mod.rs)                     │
│    - Tokio async listener receives connection           │
│    - Deserializes JSON request                          │
│    - Spawns handler task                                │
│    - <1ms overhead                                       │
└──────────────────────────────────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────────────────────────┐
│ 3. PARSER (src/parser/mod.rs)                           │
│    - Tokenizes: ["git", "che"]                          │
│    - Identifies: command="git", partial="che"           │
│    - Determines context: subcommand position            │
│    - <5ms parsing                                        │
└──────────────────────────────────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────────────────────────┐
│ 4. SPEC LOADER (src/specs/mod.rs) [Phase 2]            │
│    - Loads git.msgpack from embedded data               │
│    - Deserializes MessagePack to CompletionSpec         │
│    - Caches in LRU (keep hot specs in memory)          │
│    - <1ms lookup                                         │
└──────────────────────────────────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────────────────────────┐
│ 5. MATCHER (src/parser/matcher.rs) [Phase 2]           │
│    - Matches "che" against git subcommands              │
│    - Finds: ["checkout", "cherry", "cherry-pick"]      │
│    - Generates suggestions with descriptions            │
│    - <2ms matching                                       │
└──────────────────────────────────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────────────────────────┐
│ 6. DAEMON RESPONSE (src/daemon/mod.rs)                  │
│    - Serializes suggestions to JSON                     │
│    - Sends response through socket                      │
│    - Closes connection                                  │
│    - <1ms serialization                                  │
└──────────────────────────────────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────────────────────────┐
│ 7. SHELL INTEGRATION (shell-integration/zsh.zsh)        │
│    - Receives JSON response                             │
│    - Parses suggestions array                           │
│    - Invokes TUI rendering                              │
└──────────────────────────────────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────────────────────────┐
│ 8. TUI RENDERING (src/tui/mod.rs)                       │
│    - Ratatui renders dropdown below cursor              │
│    - Shows: checkout, cherry, cherry-pick               │
│    - Handles keyboard: arrows, Enter, Esc               │
│    - <10ms rendering                                     │
└──────────────────────────────────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────────────────────────┐
│ 9. USER SELECTION                                        │
│    - User presses Enter on "checkout"                   │
│    - Shell integration updates BUFFER                   │
│    - New buffer: "git checkout "                        │
│    - Clears UI, redraws prompt                          │
└──────────────────────────────────────────────────────────┘

Total Time: <20ms (design goal)
- IPC: <1ms
- Parser: <5ms
- Spec lookup: <1ms
- Matching: <2ms
- Response: <1ms
- TUI: <10ms
```

## Key Architectural Decisions

### 1. Persistent Daemon Architecture

**Decision:** Run a single background daemon serving all shells

**Rationale:**

- Zero startup cost per request
- Specs loaded once, shared across sessions
- ~20MB total vs N\*20MB for per-shell embedding
- Sub-millisecond IPC with Unix sockets

**Trade-offs:**

- More complex than direct execution
- Need lifecycle management
- Requires IPC design

See [ADR-0002](../adr/0002-daemon-architecture.md) for full details.

### 2. Direct Terminal Control

**Decision:** Use ZLE widgets and terminal rendering, not Accessibility API

**Rationale:**

- Zero positioning bugs (terminal handles positioning)
- Universal across all terminals
- No Accessibility permissions needed
- Native feel, integrated with terminal

**Trade-offs:**

- Shell-specific integration code
- Limited to terminal capabilities
- Can't overlay (pushes content down)

See [ADR-0004](../adr/0004-direct-terminal-control.md) for full details.

### 3. Build-time Spec Parsing

**Decision:** Parse TypeScript specs at build time, embed as MessagePack

**Rationale:**

- <5ms daemon startup (no parsing overhead)
- <1ms spec lookup (binary format)
- All specs available immediately
- Build-time validation

**Trade-offs:**

- Longer build times (~60s)
- Can't update specs without rebuild
- Need deno_ast build dependency

See [ADR-0003](../adr/0003-build-time-spec-parsing.md) for full details.

### 4. Async I/O with Tokio

**Decision:** Use Tokio for async Unix socket server

**Rationale:**

- Handle concurrent connections efficiently
- Non-blocking I/O for multiple shells
- Mature, battle-tested runtime
- Standard for Rust async

**Trade-offs:**

- Async complexity
- Runtime overhead (~100KB)

### 5. Immediate-Mode UI with Ratatui

**Decision:** Use Ratatui for terminal UI rendering

**Rationale:**

- Rich widget system (List, Block, etc.)
- Active maintenance and community
- <10ms render time
- Declarative UI code

**Trade-offs:**

- ~2MB binary size
- Framework abstraction

See [ADR-0005](../adr/0005-ratatui-for-tui.md) for full details.

## Data Flow

### Request Flow

```rust
// 1. Shell sends JSON
{
  "buffer": "git checkout -b ",
  "cursor": 18
}

// 2. Parser tokenizes
["git", "checkout", "-b", ""]

// 3. Parser analyzes context
ParseContext {
  command: "git",
  subcommands: ["checkout"],
  current_flag: Some("-b"),
  expects: Argument(BranchName),
  cursor_position: 18
}

// 4. Spec loader fetches
CompletionSpec {
  name: "git",
  subcommands: [
    Subcommand {
      name: "checkout",
      options: [
        Option { names: ["-b"], args: [...] }
      ]
    }
  ]
}

// 5. Matcher generates suggestions
vec![
  Suggestion {
    text: "feature/new",
    description: Some("Create new feature branch"),
    suggestion_type: Argument
  },
  Suggestion {
    text: "main",
    description: Some("Branch from main"),
    suggestion_type: Argument
  }
]

// 6. Daemon responds
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

### State Management

**Daemon State:**

- Unix socket listener (persistent)
- Connection pool (per-connection state)
- Spec cache (LRU, shared across connections)
- No user session state (stateless requests)

**Shell Integration State:**

- Current buffer (from ZLE)
- Cursor position (from ZLE)
- UI state (selected item, scroll position)
- No persistent state between requests

**UI State:**

- Suggestions list
- Selected index
- Scroll offset
- Theme colors

## Performance Budget

### Total Latency: <20ms

Breakdown:

- **IPC (socket):** <1ms
  - Connect + send + receive
  - Unix sockets are extremely fast

- **Parser:** <5ms
  - Tokenization: <1ms
  - Context analysis: <2ms
  - Spec matching: <2ms

- **Spec lookup:** <1ms
  - LRU cache hit: <0.1ms
  - MessagePack deserialize: <1ms (cache miss)

- **TUI rendering:** <10ms
  - Layout calculation: <2ms
  - Widget rendering: <5ms
  - Terminal output: <3ms

- **Buffer:** 3ms (safety margin)

### Memory Budget: <50MB

- **Daemon base:** ~10MB (Rust runtime + Tokio)
- **All specs (600+):** ~15MB (MessagePack compressed)
- **LRU cache:** ~5MB (50 hot specs in memory)
- **Connection overhead:** ~100KB per connection
- **Peak usage:** ~30MB typical, <50MB maximum

### Binary Size: ~8-15MB

- **Rust core:** ~3MB
- **Tokio + dependencies:** ~2MB
- **Ratatui + Crossterm:** ~2MB
- **Embedded specs:** ~10MB (MessagePack)
- **Stripped release:** ~8MB

## Error Handling

### Strategy

1. **Graceful degradation:** If daemon fails, no completions (not shell crash)
2. **Auto-recovery:** Shell integration restarts daemon if needed
3. **Timeouts:** 1s connection timeout, 100ms request timeout
4. **Logging:** Debug logs for troubleshooting, silent in production

### Error Propagation

```text
Shell Integration
    │
    ├─▶ Daemon unreachable
    │   └─▶ Start daemon, retry once
    │       └─▶ If fails: silent, no completions
    │
    ├─▶ Request timeout
    │   └─▶ Cancel, no completions
    │
    └─▶ Invalid response
        └─▶ Log error, no completions

Daemon
    │
    ├─▶ Parse error
    │   └─▶ Return empty suggestions
    │
    ├─▶ Spec not found
    │   └─▶ Return empty suggestions
    │
    └─▶ Internal panic
        └─▶ Catch at task boundary, log, continue serving
```

## Security Considerations

### Unix Socket Permissions

```bash
/tmp/autocomplete-rs.sock
  - Owner: user
  - Permissions: 0600 (user read/write only)
  - No other users can connect
```

### Input Validation

- **Buffer length:** Limited to 10,000 characters
- **Cursor position:** Validated against buffer length
- **JSON parsing:** Strict schema, reject invalid

### Resource Limits

- **Max connections:** 100 concurrent
- **Max request size:** 100KB
- **Rate limiting:** 1000 requests/second per connection

### No Code Execution

- Specs are data, not code
- No dynamic evaluation
- No user-provided specs (Phase 1)

## Scalability

### Current Design (Single User)

- One daemon per user
- Handles 10-100 terminal tabs easily
- ~30MB memory total
- <0.1% CPU when idle

### Future Considerations (Phase 4+)

**Multi-user system:**

- Socket path includes UID: `/tmp/autocomplete-rs-$UID.sock`
- Separate daemon per user

**Custom specs:**

- Load from `~/.config/autocomplete-rs/specs/`
- Override embedded specs
- Dynamic loading

**Spec updates:**

- Check for updates on startup
- Download from CDN
- Cache in `~/.cache/autocomplete-rs/`

## Testing Strategy

### Unit Tests

- Each module has inline `#[cfg(test)]` tests
- Test pure functions (tokenizer, matcher, etc.)
- Mock external dependencies
- Fast (<1s for all unit tests)

### Integration Tests

- `tests/` directory
- Test daemon + parser + specs together
- Test shell integration via expect scripts
- Real socket communication
- Slower (~5s for all integration tests)

### Benchmarks

- `benches/` directory
- Criterion.rs for accurate timing
- Measure each component
- Track performance over time
- Compare before/after optimizations

### Performance Tests

- Verify <20ms total latency
- Verify <5ms daemon startup
- Verify <50MB memory usage
- Run in CI on every commit

## Deployment

### Installation

1. **Binary:** Single executable (~8-15MB)
2. **Shell integration:** One script per shell (~5KB)
3. **Config:** Optional TOML file
4. **No runtime dependencies**

### Upgrade

1. Replace binary
2. Restart daemon (automatically or manual)
3. Shell integration auto-updates via binary
4. No data migration needed

## Monitoring

### Metrics (Future)

- Requests per second
- Average latency
- P50, P95, P99 latency
- Cache hit rate
- Daemon uptime
- Memory usage

### Logging

- `RUST_LOG=debug` for development
- `RUST_LOG=error` for production
- Structured logging with tracing
- Logs to stderr (captured if needed)

## Disaster Recovery

### Daemon Crashes

- Shell integration auto-restarts on next request
- No data loss (stateless)
- User sees brief delay (~5ms startup)

### Corrupt Specs

- Build-time embedding prevents corruption
- If binary corrupt, won't run (OS protection)
- Reinstall fixes

### Socket Issues

- Socket auto-recreated on daemon start
- Stale socket cleaned up
- If permissions wrong, fails gracefully

## Future Architecture Changes

### Phase 2: Spec Parsing

- Enable deno_ast
- Implement MessagePack serialization
- Embed 600+ Fig specs

### Phase 3: Themes & Config

- Add TOML config parsing
- Implement theme system
- Add user customization

### Phase 4: Multi-Shell

- Add bash readline integration
- Add fish completion integration
- Unify shell abstraction

### Post-1.0: Advanced Features

- GPU-accelerated rendering (Kitty graphics protocol)
- Plugin system for custom specs
- Cloud sync for custom specs
- LSP integration for smart completions

## Related Documents

- [Daemon Architecture](daemon.md) - Detailed daemon design
- [Parser Architecture](parser.md) - Parser algorithms
- [TUI Architecture](tui.md) - UI rendering details
- [ADRs](../adr/) - Architecture decision records
- [Development Guide](../development/getting-started.md) - How to build
- [Project Structure](../development/project-structure.md) - Code organization
