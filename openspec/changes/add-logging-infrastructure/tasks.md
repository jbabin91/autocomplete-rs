# Implementation Tasks

## Phase 1: Core Module Setup

### Task 1.1: Create module structure

- Create `src/logging/mod.rs` with public API
- Create `src/logging/config.rs` for configuration
- Create `src/logging/layers.rs` for custom layers
- Create `src/logging/fields.rs` for structured field helpers
- Create `src/logging/privacy.rs` for redaction logic
- Export public API from `src/logging/mod.rs`

### Task 1.2: Add dependencies

- Add `tracing-appender = "0.2"` to Cargo.toml
- Add `uuid = { version = "1.0", features = ["v4"] }` to Cargo.toml
- Update existing `tracing-subscriber` with features: `["env-filter", "json", "fmt"]`
- Verify `cargo check` passes

### Task 1.3: Implement Mode enum and detection

- Define `Mode` enum (Production, Development, Troubleshooting)
- Implement `Mode::detect()` from environment variables
  - Check `AUTOCOMPLETE_DEV` for Development
  - Check `RUST_LOG` for Troubleshooting
  - Default to Production
- Add unit tests for mode detection

### Task 1.4: Implement LogConfig struct

- Define `LogConfig` with all configuration fields
- Implement `Default` trait for production defaults
- Implement `LogConfig::from_env()` for environment detection
- Add builder pattern for custom configuration
- Add unit tests for config construction

## Phase 2: Privacy and Redaction

### Task 2.1: Implement buffer redaction

- Create `redact_buffer()` function in `privacy.rs`
  - Handle buffers > 10 chars (first 3 + "\*\*\*" + last 3)
  - Handle buffers 6-10 chars (first 3 + "\*\*\*" + last 3)
  - Handle buffers < 6 chars (all "\*\*\*")
- Include buffer length in output: `"git***mit" (10)`
- Add unit tests for various buffer lengths

### Task 2.2: Implement sensitive pattern detection

- Create `detect_and_redact_patterns()` function
- Detect patterns: password=, api_key=, token=, etc.
- Detect URL credentials: `https://user:pass@host`
- Detect environment variable assignments
- Add unit tests for each pattern type
- Make patterns configurable

### Task 2.3: Implement redaction decision logic

- Create `should_redact()` function checking mode and env vars
- Check `AUTOCOMPLETE_LOG_FULL_BUFFERS` override
- Return true for Production, false for Development
- Add unit tests for each mode combination

### Task 2.4: Create RedactedField wrapper

- Implement `RedactedField<T>` struct for automatic redaction
- Implement `Display` trait applying redaction
- Implement `Debug` trait applying redaction
- Add tests for structured field redaction

## Phase 3: Custom Layers

### Task 3.1: Implement CompactLayer

- Create compact formatter in `layers.rs`
- Format: `[timestamp] LEVEL module field=value message`
- Fixed field order for consistent parsing
- Minimal whitespace
- Add tests for format output

### Task 3.2: Implement PrettyLayer

- Create pretty formatter with ANSI colors
- Multi-line format with span hierarchy
- Indentation showing nesting
- File:line numbers
- Add tests for format output

### Task 3.3: Implement JsonLayer

- Create JSON Lines formatter
- One JSON object per line
- All fields included
- Valid JSON output
- Add tests for valid JSON parsing

### Task 3.4: Implement performance threshold layer

- Create layer tracking span durations
- Emit WARN when span exceeds threshold (10ms)
- Include operation name and actual duration
- Add tests for threshold detection

## Phase 4: File Management

### Task 4.1: Implement log directory setup

- Create `default_log_dir()` function → `~/.autocomplete-rs/logs/`
- Create directory with secure permissions (0700)
- Handle custom `AUTOCOMPLETE_LOG_DIR` env var
- Add error handling for permission failures
- Add tests for directory creation

### Task 4.2: Configure file appender

- Set up `tracing_appender::rolling::RollingFileAppender`
- Configure daily rotation
- Configure size-based rotation (50MB)
- Set file permissions to 0600
- Add tests for file creation

### Task 4.3: Implement retention cleanup

- Create `cleanup_old_logs()` function
- Determine retention period based on mode
  - Production: 7 days
  - Development: 30 days
  - Troubleshooting: 90 days
- Delete files older than retention
- Add tests for cleanup logic

### Task 4.4: Implement log compression

- Compress rotated logs with gzip
- Decompress for analysis commands
- Add tests for compression/decompression

## Phase 5: Request Correlation

### Task 5.1: Implement request_id generation

- Create `new_request_id()` → UUID v4
- Add to public API
- Add tests for UUID format

### Task 5.2: Create span helper macros

- Implement `span_request!(request_id, name, { code })` macro
- Automatically attach request_id to span
- Automatically log span timing on completion
- Add tests for macro expansion

### Task 5.3: Create performance warning macro

- Implement `perf_warn!(threshold_ms, name, { code })` macro
- Emit WARN if operation exceeds threshold
- Include actual duration in warning
- Add tests for threshold detection

### Task 5.4: Create error logging helper

- Implement `log_error!(err, context, { fields })` macro
- Include full error chain
- Include span context
- Add tests for error formatting

## Phase 6: Subscriber Composition

### Task 6.1: Implement layer composition for Production

- Create `build_production_subscriber()` function
- Compose: EnvFilter + CompactLayer + FileAppender
- Set level to INFO
- Return composed subscriber
- Add integration test

### Task 6.2: Implement layer composition for Development

- Create `build_development_subscriber()` function
- Compose: EnvFilter + PrettyLayer (console) + CompactLayer (file)
- Set level to DEBUG
- Enable ANSI colors for console
- Add integration test

### Task 6.3: Implement layer composition for Troubleshooting

- Create `build_troubleshooting_subscriber()` function
- Compose: EnvFilter + JsonLayer + FileAppender
- Set level to TRACE
- Extended retention (90 days)
- Add integration test

### Task 6.4: Implement init() functions

- Create `init()` → auto-detect mode and initialize
- Create `init_with_config(config)` → custom configuration
- Set global default subscriber
- Return Result for error handling
- Add integration tests

## Phase 7: CLI Commands

### Task 7.1: Add logs subcommand to CLI

- Add `Logs` enum to clap command structure
- Add subcommand routing in main.rs
- Create `src/cli/logs.rs` module
- Add help text for logs command

### Task 7.2: Implement logs tail

- Implement `logs::tail()` function
- Support `-f` (follow), `-n NUM` (lines), `--level` (filter)
- Support `--since` (time filter)
- Stream file to stdout
- Add tests for tail functionality

### Task 7.3: Implement logs dump

- Implement `logs::dump()` function
- Collect current and rotated logs
- Create zip with timestamp filename
- Include PRIVACY_NOTICE.txt
- Include system-info.txt (optional)
- Warn based on redaction status
- Add tests for zip creation

### Task 7.4: Implement logs analyze

- Implement `logs::analyze()` function
- Detect high latency (>10ms)
- Detect connection failures
- Detect parsing errors
- Detect log backpressure
- Generate performance summary
- Print report to console
- Add tests for issue detection

### Task 7.5: Implement logs clean

- Implement `logs::clean()` function
- Support `--older-than`, `--dry-run`, `--all`
- Require confirmation for destructive operations
- Show deleted count and freed space
- Add tests for cleanup

### Task 7.6: Implement logs config

- Implement `logs::config()` function
- Show current mode, level, output targets
- Show environment variables
- Show daemon status and uptime
- Support `--json` output
- Add tests for config display

### Task 7.7: Implement logs list

- Implement `logs::list()` function
- List all log files with size and age
- Support sorting by time or size
- Show total files and size
- Add tests for listing

### Task 7.8: Implement logs search

- Implement `logs::search()` function
- Support `--request-id`, text pattern, regex
- Support `-C NUM` (context lines)
- Search across rotated and compressed logs
- Highlight matches
- Add tests for search

### Task 7.9: Implement logs audit-privacy

- Implement `logs::audit_privacy()` function
- Scan for potential secret patterns
- Verify redaction applied
- Report full buffer usage periods
- Add tests for audit

## Phase 8: Integration

### Task 8.1: Update main.rs

- Replace `tracing_subscriber::fmt::init()` with `logging::init()`
- Add privacy warning in troubleshooting mode
- Handle initialization errors
- Add logs subcommand routing

### Task 8.2: Add spans to daemon

- Add `#[instrument]` to `handle_connection()`
- Generate request_id for each connection
- Add span fields: request_id, buffer (redacted), cursor
- Log request lifecycle events
- Add performance tracking

### Task 8.3: Add spans to TUI

- Add `#[instrument]` to `CompletionUI::run()`
- Add span fields for selected suggestion
- Track rendering performance
- Log user selection

### Task 8.4: Add structured logging to existing code

- Replace print statements with log macros
- Add structured fields (request_id, etc.)
- Use appropriate log levels
- Add error context

## Phase 9: Testing

### Task 9.1: Unit tests for redaction

- Test buffer redaction at various lengths
- Test sensitive pattern detection
- Test redaction decision logic
- Achieve >90% coverage for privacy.rs

### Task 9.2: Unit tests for configuration

- Test mode detection from env vars
- Test config construction
- Test default values
- Achieve >90% coverage for config.rs

### Task 9.3: Integration tests for logging modes

- Test Production mode creates file-only output
- Test Development mode creates console + file
- Test Troubleshooting mode creates JSON output
- Verify output formats match specifications

### Task 9.4: Integration tests for file management

- Test log rotation (daily and size-based)
- Test retention cleanup
- Test directory creation with permissions
- Test compression/decompression

### Task 9.5: Integration tests for CLI commands

- Test logs tail with various options
- Test logs dump creates valid zip
- Test logs analyze detects issues
- Test logs clean deletes correct files
- Test logs config shows correct info

### Task 9.6: Performance tests

- Benchmark logging overhead with criterion
- Verify <0.5% of 10ms budget (50μs)
- Test non-blocking writes don't block daemon
- Test backpressure handling

### Task 9.7: Privacy compliance tests

- Test redaction is applied in production
- Test full buffers only with explicit opt-in
- Test privacy notice included in dumps
- Test audit command detects secrets

## Phase 10: Documentation

### Task 10.1: Write logging module documentation

- Document public API with rustdoc
- Add examples for init() and init_with_config()
- Document helper macros
- Add privacy section

### Task 10.2: Update README

- Add logging section explaining modes
- Document environment variables
- Add troubleshooting guide
- Add privacy and security section

### Task 10.3: Write user guide for log commands

- Document each logs subcommand
- Add examples of common workflows
- Add tips for performance debugging
- Add FAQ section

### Task 10.4: Write developer guide

- Document how to add logging to new code
- Best practices for structured logging
- Performance considerations
- Privacy guidelines

### Task 10.5: Add changelog entry

- Document new logging infrastructure
- Note performance impact
- Document breaking changes (if any)
- Add migration guide

## Task Dependencies

```sh
1.1 → 1.2 → 1.3 → 1.4
       ↓
2.1 → 2.2 → 2.3 → 2.4
                   ↓
            3.1 → 3.2 → 3.3 → 3.4
                                ↓
                  4.1 → 4.2 → 4.3 → 4.4
                                     ↓
                        5.1 → 5.2 → 5.3 → 5.4
                                           ↓
                              6.1 → 6.2 → 6.3 → 6.4
                                                  ↓
                    7.1 → 7.2, 7.3, 7.4, 7.5, 7.6, 7.7, 7.8, 7.9
                                                  ↓
                                  8.1 → 8.2 → 8.3 → 8.4
                                                     ↓
                          9.1, 9.2, 9.3, 9.4, 9.5, 9.6, 9.7
                                                     ↓
                                10.1, 10.2, 10.3, 10.4, 10.5
```

## Estimated Timeline

- **Phase 1-2 (Core + Privacy):** 2-3 days
- **Phase 3-4 (Layers + Files):** 2-3 days
- **Phase 5-6 (Correlation + Subscribers):** 1-2 days
- **Phase 7 (CLI Commands):** 3-4 days
- **Phase 8 (Integration):** 1 day
- **Phase 9 (Testing):** 2-3 days
- **Phase 10 (Documentation):** 1-2 days

**Total: ~12-18 days** (full-time equivalent)

## Success Criteria

- ✅ All tests pass (unit + integration + performance)
- ✅ Performance overhead < 0.5% (50μs per request)
- ✅ Privacy: Production mode redacts buffers by default
- ✅ All three modes work correctly (Production/Dev/Troubleshooting)
- ✅ File rotation and cleanup work automatically
- ✅ CLI commands provide useful troubleshooting tools
- ✅ Documentation complete and accurate
- ✅ No breaking changes to existing API
