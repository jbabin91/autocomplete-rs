# Logging Infrastructure Design

## Overview

This document captures the architectural decisions and trade-offs for
autocomplete-rs's logging system. The design must balance performance
constraints (<10ms latency), debugging needs, and user privacy.

## Architecture

### Component Diagram

```sh
┌─────────────────────────────────────────────────────────────┐
│ Application Code (daemon, client, TUI)                     │
│  Uses: tracing macros (span!, info!, debug!, error!)       │
└────────────────┬────────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────────┐
│ Tracing Subscriber (Registry)                              │
│  Composes multiple layers for different outputs            │
├─────────────────┬────────┬──────────────┬───────────────────┤
│ EnvFilter       │ Spans  │ Fields       │ Performance       │
│ (RUST_LOG)      │ (async)│ (structured) │ (timing)          │
└─────────────────┴────────┴──────────────┴───────────────────┘
                  │
        ┌─────────┴──────────┬──────────────┐
        ▼                    ▼              ▼
┌───────────────┐  ┌──────────────┐  ┌──────────────┐
│ Console Layer │  │ File Layer   │  │ JSON Layer   │
│ (dev mode)    │  │ (production) │  │ (analysis)   │
│ Pretty,       │  │ Compact,     │  │ Structured,  │
│ Colored       │  │ Non-blocking │  │ Machine-read │
└───────────────┘  └──────┬───────┘  └──────────────┘
                          │
                          ▼
                  ┌──────────────┐
                  │ File Appender│
                  │ + Rotation   │
                  └──────────────┘
```

### Three-Tier Mode System

#### Production Mode (Default)

**Target:** End users running daemon in normal operation

- **Log Level:** INFO (errors and important events only)
- **Output:** File only (`~/.autocomplete-rs/logs/daemon.log`)
- **Format:** Compact single-line for efficient storage
- **Privacy:** Command buffers redacted (first/last 3 chars + length)
- **Rotation:** Daily, keep 7 days
- **Performance:** Minimal overhead via non-blocking writes

**Example output:**

```sh
[2024-11-14T12:34:56Z] INFO daemon request_id=abc123 buffer="git***mit" (10) latency_ms=2.3
[2024-11-14T12:34:57Z] ERROR connection request_id=def456 error="socket timeout"
```

#### Development Mode (`AUTOCOMPLETE_DEV=1`)

**Target:** Developers working on autocomplete-rs

- **Log Level:** DEBUG (detailed debugging information)
- **Output:** Console (colored, pretty) + File
- **Format:** Multi-line with span hierarchy
- **Privacy:** Full command buffers (local development)
- **Rotation:** Daily, keep 30 days
- **Performance:** Acceptable overhead for rich feedback

**Example output:**

```sh
  2024-11-14T12:34:56.123Z DEBUG autocomplete_rs::daemon
    ┌─ handle_connection
    │ ├─ request_id: abc123
    │ ├─ buffer: "git commit"
    │ ├─ cursor: 10
    │ │
    │ ├─ 1.2ms parse_command
    │ │   ├─ command: "git"
    │ │   └─ partial: "commit"
    │ │
    │ ├─ 0.8ms generate_suggestions
    │ │   └─ count: 3
    │ │
    │ └─ 0.3ms send_response
    └─ 2.3ms total
```

#### Troubleshooting Mode (`RUST_LOG=trace` or `autocomplete_rs=trace`)

**Target:** Diagnosing production issues reported by users

- **Log Level:** TRACE (everything)
- **Output:** File with extended retention
- **Format:** JSON Lines for machine analysis
- **Privacy:** Full buffers with user consent
- **Rotation:** Weekly, keep 90 days
- **Performance:** Higher overhead, short-term debugging only

**Example output:**

```json
{"timestamp":"2024-11-14T12:34:56.123456Z","level":"TRACE","target":"autocomplete_rs::daemon","span":{"name":"handle_connection","request_id":"abc123"},"fields":{"buffer":"git commit","cursor":10},"message":"received request"}
{"timestamp":"2024-11-14T12:34:56.124567Z","level":"TRACE","target":"autocomplete_rs::parser","span":{"name":"parse_command","request_id":"abc123"},"fields":{"command":"git","partial":"commit"},"message":"parsing command"}
```

### Performance Strategy

**Overhead Budget:**

- Total request latency budget: 10ms (10,000,000 ns)
- Logging overhead budget: 0.5% = 50,000 ns = 50 μs
- Typical logging call: 100-500 ns with non-blocking writer
- Budget allows: ~100-500 log points per request ✅

**Zero-Cost Abstractions:**

- Compile-time span filtering (disabled spans = 1-2ns)
- Lazy evaluation (only format when log level enabled)
- Non-blocking writes (I/O on separate thread)
- Static strings over dynamic formatting

**Example Performance-Aware Logging:**

```rust
// Fast: Static string + structured fields
info!(request_id = %req_id, latency_ms = latency, "request completed");

// Slow: Dynamic formatting
// info!("Request {} completed in {}ms", req_id, latency);  ❌
```

### Request Correlation

Every completion request gets a unique `request_id` (UUID v4) that flows through
all components:

```sh
Client (complete command)
  └─ request_id: abc123
      │
      ├─ send to daemon → [request_id=abc123]
      │   │
      │   ├─ daemon receive → [request_id=abc123]
      │   ├─ parse command → [request_id=abc123]
      │   ├─ generate suggestions → [request_id=abc123]
      │   └─ send response → [request_id=abc123]
      │
      ├─ receive response → [request_id=abc123]
      │
      └─ show TUI → [request_id=abc123]
          ├─ render → [request_id=abc123]
          └─ user selection → [request_id=abc123]
```

**Implementation:**

- Client generates UUID on request
- Passed in JSON request to daemon
- Daemon attaches to tracing span
- All logs within span inherit request_id
- Enables filtering: `grep abc123 daemon.log` shows full request lifecycle

### Privacy and Security

**Threat Model:**

- Command buffers may contain passwords, API keys, secrets
- Logs stored on disk may be readable by other processes
- Users may accidentally share logs publicly when reporting bugs

**Mitigation Strategy:**

**Production Mode (default):**

- Redact middle of command buffers
- Log only: first 3 chars + last 3 chars + total length
- Example: `git commit -m "secret password"` → `"git***ord" (32)`

**Development Mode:**

- Full buffers logged (developer's local machine)
- Clear warning in documentation

**Troubleshooting Mode:**

- Require explicit opt-in: `AUTOCOMPLETE_LOG_FULL_BUFFERS=1`
- Log warning on daemon start: "⚠️ Full buffer logging enabled"
- Auto-disable after daemon restart

**Log Dump Command:**

- `autocomplete-rs logs dump` warns user about redaction status
- Provides option to further redact before sharing
- Includes privacy notice in generated zip

### File Management

**Directory Structure:**

```sh
~/.autocomplete-rs/logs/
├── daemon.log                # Current daemon logs
├── daemon.log.2024-11-14     # Rotated by date
├── daemon.log.2024-11-13
├── daemon.log.2024-11-12
└── performance.jsonl         # Optional: Performance metrics
```

**Rotation Strategy:**

- **Daily rotation:** At midnight local time
- **Size limit:** Max 50MB per file before forced rotation
- **Retention:**
  - Production: 7 days
  - Development: 30 days
  - Troubleshooting: 90 days
- **Compression:** gzip old logs to save space
- **Cleanup:** Auto-delete beyond retention period

**Non-Blocking Writes:**

- File I/O runs on dedicated thread (via tracing-appender)
- Main daemon thread never blocks on log writes
- Bounded queue (10,000 messages) with backpressure
- If queue full: Drop DEBUG/TRACE, preserve ERROR/WARN

## Technology Choices

### tracing Ecosystem

**Core Dependencies:**

```toml
tracing = "0.1"                                      # Macro API
tracing-subscriber = { version = "0.3",              # Subscriber
  features = ["env-filter", "json", "fmt"]
}
tracing-appender = "0.2"                             # File rotation
uuid = { version = "1.0", features = ["v4"] }        # Request IDs
```

**Optional (dev mode):**

```toml
[dev-dependencies]
console-subscriber = "0.2"  # tokio-console debugging
```

**Why tracing:**

- ✅ Zero-cost when disabled (compile-time filtering)
- ✅ Async-aware (propagates context through `.await`)
- ✅ Structured (key-value fields, not just strings)
- ✅ Rich ecosystem (appenders, formatters, layers)
- ✅ Battle-tested (Tokio, Axum, Tower use it)
- ✅ Spans for performance tracking

### Layer Architecture

**CompactLayer (Production):**

- Single-line format for efficient storage and parsing
- Fixed field order for consistent grepping
- Minimal whitespace to reduce file size

**PrettyLayer (Development):**

- Multi-line with indentation showing span hierarchy
- ANSI colors for visual parsing
- File:line numbers for quick navigation
- Full context for debugging

**JsonLayer (Troubleshooting):**

- JSON Lines format (one object per line)
- All fields included for complete analysis
- Compatible with jq, log aggregation tools
- Machine-parseable for automated analysis

## API Design

### Public API (src/logging/mod.rs)

```rust
pub enum Mode {
    Production,
    Development,
    Troubleshooting,
}

pub struct LogConfig {
    pub mode: Mode,
    pub log_dir: Option<PathBuf>,
    pub retention_days: u32,
    pub enable_console: bool,
    pub redact_buffers: bool,
}

// Simple initialization (auto-detect mode from env)
pub fn init() -> Result<()>

// Advanced configuration
pub fn init_with_config(config: LogConfig) -> Result<()>

// Utility functions
pub fn new_request_id() -> Uuid
pub fn default_log_dir() -> PathBuf  // ~/.autocomplete-rs/logs
```

### Helper Macros

```rust
// Create span with request_id and auto-timing
span_request!(request_id, "operation_name", { code })

// Log with performance threshold warning
perf_warn!(threshold_ms, "operation", { code })

// Structured error with context
log_error!(err, context, { fields })
```

### Integration Example

```rust
use autocomplete_rs::logging::{new_request_id, span_request, perf_warn};

#[instrument(skip(request))]
async fn handle_connection(request: CompletionRequest) -> Result<()> {
    let request_id = new_request_id();

    span_request!(request_id, "handle_connection", {
        perf_warn!(10, "total_request", {
            let suggestions = parse_command(&request.buffer).await?;
            send_response(suggestions).await?;
        })?;
    });

    Ok(())
}
```

## Migration Path

### Phase 1: Core Module

1. Create `src/logging/` module structure
2. Implement mode detection and layer composition
3. Keep existing `tracing_subscriber::fmt::init()` as fallback
4. No breaking changes to existing code

### Phase 2: Integration

1. Replace `fmt::init()` with `logging::init()` in main.rs
2. Add spans to critical operations (instrument macro)
3. Add request_id generation and propagation
4. Backward compatible - old logging still works

### Phase 3: Enhancement

1. Add structured fields to all log points
2. Add performance monitoring with perf_warn!
3. Add log management CLI commands
4. Full feature rollout

## Testing Strategy

### Unit Tests

- Mode detection from environment variables
- Filter parsing and application
- Structured field helpers (request_id, etc.)
- Redaction logic for privacy

### Integration Tests

- Files are created and rotated correctly
- Different modes produce expected output formats
- Non-blocking writes don't block daemon
- Cleanup respects retention policies

### Performance Tests

- Benchmark logging overhead (<0.5% budget)
- High load testing (1000 req/s sustained)
- Memory usage under backpressure
- Verify non-blocking behavior

### Manual Checklist

- [ ] Production mode: Minimal console output
- [ ] Dev mode: Colorful, readable console + file
- [ ] Troubleshooting mode: Valid JSON output
- [ ] Log rotation works at midnight
- [ ] Cleanup removes old logs
- [ ] `logs dump` creates valid zip
- [ ] `logs analyze` detects common issues

## Future Enhancements

**Out of scope for initial implementation:**

- OpenTelemetry integration (distributed tracing)
- Remote log aggregation (Splunk, Datadog, etc.)
- Real-time log streaming over network
- Log compression beyond gzip
- Custom log analyzers for specific issues

These can be added later without breaking the core architecture.
