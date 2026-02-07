# ADR-0007: Logging Infrastructure with tracing Ecosystem

**Status:** Accepted **Date:** 2026-02-06 **Decision Makers:** Project Team
**Technical Story:** Choose logging architecture for debugging, troubleshooting,
and performance monitoring

## Context

We need structured logging before implementing complex features like the parser
and inline dropdown. The system must:

- Add <0.5% overhead to the 10ms latency budget (~50us per request)
- Never block the daemon on file I/O
- Provide request correlation across async boundaries
- Protect user privacy (command buffers may contain passwords, API keys)
- Support three audiences: end users, developers, and troubleshooters

### Requirements

- **Performance:** <50us overhead per request (non-blocking writes)
- **Privacy:** Redact command buffers by default in production
- **Correlation:** Trace a single request across all components via request_id
- **Modes:** Production (quiet), Development (verbose), Troubleshooting (full)
- **File management:** Automatic rotation and retention cleanup
- **Zero-cost:** Disabled log levels compile to 1-2ns no-ops

## Decision

We will use the **tracing ecosystem** with a **three-tier mode system** and
**privacy-first defaults**.

### Architecture

1. **tracing + tracing-subscriber** for structured, async-aware logging
2. **tracing-appender** for non-blocking file rotation
3. **Three modes** auto-detected from environment variables:
   - Production (default): INFO, file-only, compact format, buffers redacted
   - Development (`AUTOCOMPLETE_DEV=1`): DEBUG, console+file, pretty format
   - Troubleshooting (`RUST_LOG=trace`): TRACE, JSON Lines format
4. **Privacy layer** that redacts command buffers (first 3 + last 3 chars)
5. **Request correlation** via UUID v4 attached to tracing spans

### Key Design Choices

- **Privacy-first:** Production redacts buffers by default. Full buffer logging
  requires explicit `AUTOCOMPLETE_LOG_FULL_BUFFERS=1` (env var only, not
  persisted in config, auto-resets on daemon restart).
- **Non-blocking writes:** File I/O on a dedicated thread via tracing-appender.
  Bounded queue (10,000 messages) with backpressure: drop DEBUG/TRACE, preserve
  ERROR/WARN.
- **Mode detection from env:** No config file needed. Environment variables
  determine mode, matching Rust ecosystem conventions (`RUST_LOG`).

## Consequences

### Positive

**Performance:**

- Zero-cost when disabled (compile-time span filtering, 1-2ns)
- Lazy field evaluation (only format when level enabled)
- Non-blocking writes (<50us overhead per request)
- Budget allows ~100-500 log points per request

**Developer Experience:**

- Async-aware (context propagates through `.await` boundaries)
- Structured fields (key-value, not just strings)
- `#[instrument]` macro for automatic span creation
- Rich dev mode output with span hierarchy and colors

**Privacy:**

- Safe to share production logs (buffers redacted)
- No telemetry, logs stay on user's machine
- Explicit opt-in for full buffer logging

**Ecosystem:**

- Battle-tested (Tokio, Axum, Tower all use tracing)
- Rich layer ecosystem (formatters, appenders, filters)
- Already a dependency (tracing-subscriber used in daemon)

### Negative

**Complexity:**

- Custom redaction layer adds code
- Three-mode composition requires careful layer stacking
- File rotation and retention add operational concerns

**Dependencies:**

- tracing-appender (new, but same ecosystem)
- uuid (new, for request correlation)

## Alternatives Considered

### Option 1: slog

**Pros:** Compile-time type safety, very low overhead
**Cons:** More verbose API, less async support, smaller ecosystem
**Why Not:** tracing has better Tokio integration and we already use it.

### Option 2: env_logger (log crate)

**Pros:** Simple, widely used
**Cons:** Not structured, no async context, no spans
**Why Not:** Need structured logging for request correlation across async
boundaries.

### Option 3: Custom logging solution

**Pros:** Full control, tailored to needs
**Cons:** Reinventing the wheel, maintenance burden
**Why Not:** tracing is battle-tested and zero-cost. Not worth the maintenance.

## Comparison Matrix

| Criterion      | tracing   | slog      | env_logger | Custom   |
| -------------- | --------- | --------- | ---------- | -------- |
| Async support  | Native ✅ | Plugin ⚠️ | None ❌    | DIY ❌   |
| Structured     | Yes ✅    | Yes ✅    | No ❌      | DIY ⚠️   |
| Zero-cost      | Yes ✅    | Yes ✅    | Yes ✅     | Maybe ⚠️ |
| Ecosystem      | Large ✅  | Medium ⚠️ | Large ✅   | None ❌  |
| Spans          | Native ✅ | No ❌     | No ❌      | DIY ❌   |
| Already in use | Yes ✅    | No ❌     | No ❌      | No ❌    |
| Maintenance    | Community | Community | Community  | Us ❌    |

## Implementation Scope

### Phase 1 (MVP — unblocks parser)

- Core logging module with mode detection and layer composition
- Privacy redaction for command buffers
- Non-blocking file appender with daily rotation
- Replace `tracing_subscriber::fmt::init()` in main.rs
- Add `#[instrument]` spans to daemon handler

### Deferred (post-MVP)

- CLI log management commands (tail, dump, analyze, search, clean)
- Custom helper macros (span_request!, perf_warn!)
- Log compression (gzip rotated files)
- Performance threshold layer (auto-warn on >10ms)
- Privacy audit command

## References

- [tracing crate](https://docs.rs/tracing)
- [tracing-subscriber](https://docs.rs/tracing-subscriber)
- [tracing-appender](https://docs.rs/tracing-appender)
- [Design Spec: Logging Infrastructure](../design/logging.md)
