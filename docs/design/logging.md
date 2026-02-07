# Logging Infrastructure — Design Spec

> **This is a design specification, not documentation.** It describes the
> intended logging design. Actual documentation will be written after
> implementation.

This document details the design of the autocomplete-rs logging system.

## Overview

The logging system provides structured, async-aware logging with:

- Three-tier mode system (Production / Development / Troubleshooting)
- Non-blocking file writes with automatic rotation
- Privacy-first buffer redaction
- Request correlation across async boundaries
- Performance monitoring via tracing spans

**ADR:** [ADR-0007: Logging Infrastructure](../adr/0007-logging-infrastructure.md)

## Design Principles

1. **Privacy-First:** Redact command buffers by default — users type secrets
2. **Zero-Cost:** Disabled log levels compile to 1-2ns no-ops
3. **Non-Blocking:** File I/O must never block daemon request processing
4. **Structured:** Key-value fields, not just strings — enables machine analysis
5. **Async-Aware:** Context propagates through `.await` boundaries via spans

## Architecture

### Component Diagram

```text
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
│ EnvFilter       │ Spans  │ Redaction    │ Performance       │
│ (RUST_LOG)      │ (async)│ (privacy)    │ (timing)          │
└─────────────────┴────────┴──────────────┴───────────────────┘
                  │
        ┌─────────┴──────────┬──────────────┐
        ▼                    ▼              ▼
┌───────────────┐  ┌──────────────┐  ┌──────────────┐
│ Console Layer │  │ File Layer   │  │ JSON Layer   │
│ (dev mode)    │  │ (production) │  │ (troubleshoot│
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

### Module Structure

```text
src/
  logging/
    mod.rs          # Public API: init(), init_with_config(), Mode, LogConfig
    config.rs       # Mode detection from env, LogConfig builder
    layers.rs       # Custom layer implementations (compact, pretty, JSON)
    privacy.rs      # Buffer redaction, sensitive pattern detection
    fields.rs       # Structured field helpers (request_id, etc.)
```

## Three-Tier Mode System

### Production Mode (Default)

**Target:** End users running daemon in normal operation

- **Log Level:** INFO (errors and important events only)
- **Output:** File only (`~/.autocomplete-rs/logs/daemon.log`)
- **Format:** Compact single-line for efficient storage
- **Privacy:** Command buffers redacted (first 3 + last 3 chars + length)
- **Rotation:** Daily, keep 7 days
- **Performance:** Minimal overhead via non-blocking writes

```text
[2025-11-14T12:34:56Z] INFO daemon request_id=abc123 buffer="git***mit" (10) latency_ms=2.3
[2025-11-14T12:34:57Z] ERROR connection request_id=def456 error="socket timeout"
```

### Development Mode (`AUTOCOMPLETE_DEV=1`)

**Target:** Developers working on autocomplete-rs

- **Log Level:** DEBUG (detailed debugging information)
- **Output:** Console (colored, pretty) + File
- **Format:** Multi-line with span hierarchy
- **Privacy:** Full command buffers (local development)
- **Rotation:** Daily, keep 30 days

```text
  2025-11-14T12:34:56.123Z DEBUG autocomplete_rs::daemon
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

### Troubleshooting Mode (`RUST_LOG=trace`)

**Target:** Diagnosing production issues reported by users

- **Log Level:** TRACE (everything)
- **Output:** File with extended retention
- **Format:** JSON Lines for machine analysis
- **Privacy:** Redacted by default; full buffers require `AUTOCOMPLETE_LOG_FULL_BUFFERS=1`
- **Rotation:** Weekly, keep 90 days

```json
{
  "timestamp": "2025-11-14T12:34:56.123456Z",
  "level": "TRACE",
  "target": "autocomplete_rs::daemon",
  "span": { "name": "handle_connection", "request_id": "abc123" },
  "fields": { "buffer": "git***mit", "cursor": 10 },
  "message": "received request"
}
```

## Performance Strategy

**Overhead Budget:**

- Total request latency budget: 10ms (10,000,000 ns)
- Logging overhead budget: 0.5% = 50,000 ns = 50us
- Typical logging call: 100-500 ns with non-blocking writer
- Budget allows: ~100-500 log points per request

**Zero-Cost Patterns:**

```rust
// Fast: Static string + structured fields (100-500ns)
info!(request_id = %req_id, latency_ms = latency, "request completed");

// Slow: Dynamic formatting (avoid)
// info!("Request {} completed in {}ms", req_id, latency);
```

**Non-Blocking Writes:**

- File I/O runs on dedicated thread (via tracing-appender)
- Main daemon thread never blocks on log writes
- Bounded queue (10,000 messages) with backpressure
- If queue full: drop DEBUG/TRACE, preserve ERROR/WARN

## Privacy and Redaction

### Threat Model

- Command buffers may contain passwords, API keys, secrets
- Logs stored on disk may be readable by other processes
- Users may accidentally share logs publicly when reporting bugs

### Buffer Redaction

| Buffer Length | Input                    | Redacted Output  |
| ------------- | ------------------------ | ---------------- |
| >= 6 chars    | `git commit -m "secret"` | `git***et"` (23) |
| < 6 chars     | `ls`                     | `***` (2)        |

Rule: Preserve first 3 + last 3 characters, replace middle with `***`, append
length.

### Sensitive Pattern Detection

Always redacted (even in dev mode):

- `password=`, `pwd=`, `passwd=` values
- `api_key=`, `apikey=`, `token=`, `secret=` values
- URL credentials: `https://user:pass@host` -> `https://[REDACTED]@host`
- `export SECRET=value` -> `export SECRET=[REDACTED]`

### Mode Overrides

| Mode            | Buffer Redaction | Pattern Redaction | Override                          |
| --------------- | ---------------- | ----------------- | --------------------------------- |
| Production      | Yes              | Yes               | None (always redacted)            |
| Development     | No               | Yes               | —                                 |
| Troubleshooting | Yes              | Yes               | `AUTOCOMPLETE_LOG_FULL_BUFFERS=1` |

`AUTOCOMPLETE_LOG_FULL_BUFFERS` is env-var-only (cannot be set in config file)
and does not persist across daemon restarts.

## Request Correlation

Every completion request gets a unique `request_id` (UUID v4) that flows through
all components:

```text
Client (complete command)
  └─ request_id: abc123
      ├─ send to daemon → [request_id=abc123]
      │   ├─ daemon receive → [request_id=abc123]
      │   ├─ parse command → [request_id=abc123]
      │   ├─ generate suggestions → [request_id=abc123]
      │   └─ send response → [request_id=abc123]
      ├─ receive response → [request_id=abc123]
      └─ show TUI → [request_id=abc123]
```

**Implementation:** Client generates UUID, passes in request JSON, daemon
attaches to tracing span, all nested logs inherit request_id.

**Debugging:** `grep abc123 daemon.log` shows the full request lifecycle.

## File Management

### Directory Structure

```text
~/.autocomplete-rs/logs/
├── daemon.log                # Current daemon logs
├── daemon.log.2025-11-14     # Rotated by date
├── daemon.log.2025-11-13
└── daemon.log.2025-11-12
```

### Rotation and Retention

| Setting    | Production | Development | Troubleshooting |
| ---------- | ---------- | ----------- | --------------- |
| Rotation   | Daily      | Daily       | Weekly          |
| Size limit | 50MB       | 50MB        | 50MB            |
| Retention  | 7 days     | 30 days     | 90 days         |

### Permissions

- Log directory: `0700` (owner access only)
- Log files: `0600` (owner read/write only)
- Rotated files inherit restrictive permissions
- If existing permissions are wrong: warn to stderr, attempt auto-correction

### Error Handling

- If log directory cannot be created (permissions, disk full): return `Result::Err`
  with actionable message (e.g., "cannot create ~/.autocomplete-rs/logs/: Permission
  denied. Try: mkdir -p ~/.autocomplete-rs/logs && chmod 700 ~/.autocomplete-rs/logs")
- Daemon should fall back to stderr-only logging rather than refusing to start
- File rotation failures: warn and continue writing to current file

## Public API

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

/// Auto-detect mode from environment and initialize logging.
pub fn init() -> Result<()>

/// Initialize with explicit configuration (ignores environment).
pub fn init_with_config(config: LogConfig) -> Result<()>

/// Generate a new UUID v4 for request correlation.
pub fn new_request_id() -> Uuid

/// Default log directory: ~/.autocomplete-rs/logs
pub fn default_log_dir() -> PathBuf
```

### Integration Example

```rust
use autocomplete_rs::logging;

#[instrument(skip(request))]
async fn handle_connection(request: CompletionRequest) -> Result<()> {
    let request_id = logging::new_request_id();
    let _span = info_span!("handle_request", %request_id).entered();

    info!(buffer = %redact(&request.buffer), cursor = request.cursor, "received");

    let suggestions = parse_command(&request.buffer).await?;
    info!(count = suggestions.len(), "suggestions generated");

    send_response(suggestions).await?;
    Ok(())
}
```

## Environment Variables

| Variable                          | Effect                                        |
| --------------------------------- | --------------------------------------------- |
| `AUTOCOMPLETE_DEV=1`              | Enable development mode (DEBUG, console+file) |
| `RUST_LOG=trace`                  | Enable troubleshooting mode (TRACE, JSON)     |
| `RUST_LOG=autocomplete_rs=X`      | Module-specific log level                     |
| `AUTOCOMPLETE_LOG_DIR=/path`      | Custom log directory                          |
| `AUTOCOMPLETE_CONSOLE=1`          | Force console output in any mode              |
| `AUTOCOMPLETE_LOG_FULL_BUFFERS=1` | Disable buffer redaction (requires opt-in)    |

## Testing Strategy

### Unit Tests

- Mode detection from environment variables
- Buffer redaction at various lengths
- Sensitive pattern detection
- Config construction and defaults

### Integration Tests

- Files created with correct permissions (0600/0700)
- Different modes produce expected output formats
- Non-blocking writes don't block daemon
- Rotation works at size and time boundaries

### Performance Tests

- Benchmark logging overhead with criterion (<50us)
- Non-blocking verification under load (1000 req/s)
- Backpressure behavior when queue full

## Future Enhancements (Post-MVP)

These are deferred to keep the initial implementation focused:

- **CLI commands:** `logs tail`, `logs dump`, `logs analyze`, `logs search`,
  `logs clean`, `logs config`, `logs list`, `logs audit-privacy`
- **Custom macros:** `span_request!`, `perf_warn!`, `log_error!`
- **Log compression:** gzip rotated files
- **Performance threshold layer:** Auto-warn when spans exceed 10ms
- **OpenTelemetry:** Distributed tracing integration
