# Add Logging Infrastructure

**Priority:** 1.5 (Between Phase 1A and 1B)
**Phase:** Infrastructure
**Dependencies:** add-foundation-architecture (Phase 1A)
**Blocks:** implement-mvp-parser (Phase 1B), all future debugging

## Why

We need a comprehensive logging system before implementing complex features like
the parser. Logging infrastructure is critical for:

1. **Developer productivity** - Debug parser logic and async operations
2. **User troubleshooting** - Diagnose issues without requiring code changes
3. **Performance monitoring** - Track if we're meeting <10ms latency goals
4. **Production debugging** - Investigate issues reported by users

Building this now (before Phase 1B) prevents retrofit work and ensures we have
debugging tools ready when parser complexity increases.

## What Changes

- Three-tier logging system (Production/Development/Troubleshooting modes)
- Non-blocking file appender with automatic rotation
- Request correlation across async boundaries
- Performance monitoring with automatic threshold warnings
- Privacy-safe defaults (redact sensitive command buffers)
- Log management CLI (tail, dump, analyze, clean)
- Structured logging with JSON export for analysis

## Impact

- Affected specs:
  - `logging-core` (new capability) - Core logging module and modes
  - `logging-cli` (new capability) - Log management commands
  - `logging-privacy` (new capability) - Data redaction and privacy
- Affected code:
  - `src/logging/mod.rs` - New module for logging setup
  - `src/logging/config.rs` - Mode detection and configuration
  - `src/logging/layers.rs` - Custom layer implementations
  - `src/logging/fields.rs` - Structured field helpers
  - `src/main.rs` - Replace basic logging setup, add logs subcommand
  - `src/daemon/mod.rs` - Add spans and structured logging
  - `src/tui/mod.rs` - Add performance tracking
  - `Cargo.toml` - Add tracing-appender, uuid dependencies
- Dependencies added:
  - tracing-appender = "0.2" (non-blocking file writes)
  - uuid = { version = "1.0", features = ["v4"] } (request correlation)
- Performance impact: <0.5% overhead (50μs of 10ms budget)
- Migration: Backward compatible, enhances existing tracing setup

## Design Decisions

- **tracing ecosystem over alternatives** - Already integrated, async-aware,
  zero-cost when disabled, rich ecosystem
- **Three-tier modes** - Balances user privacy, developer productivity, and
  troubleshooting needs
- **Non-blocking writes** - File I/O must not block daemon request processing
- **Privacy-first defaults** - Redact command buffers in production to protect
  sensitive data
- **Structured logging** - Enables machine-parseable analysis and correlation
- **Built-in log analyzer** - Users can self-diagnose before filing issues
- **Local-only by default** - No telemetry, logs stay on user's machine

## Alternatives Considered

**slog ecosystem:**

- Pros: Compile-time type safety, very low overhead
- Cons: More verbose API, less async support, smaller ecosystem
- Rejected: tracing has better Tokio integration

**env_logger (log crate):**

- Pros: Simple, widely used
- Cons: Not structured, no async context, no spans
- Rejected: Need structured logging for request correlation

**Custom logging solution:**

- Pros: Full control, tailored to our needs
- Cons: Reinventing the wheel, maintenance burden
- Rejected: tracing is battle-tested and zero-cost
