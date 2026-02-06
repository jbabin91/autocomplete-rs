# Logging Core Specification

## ADDED Requirements

### Requirement: Mode Detection

The logging system SHALL automatically detect the appropriate logging mode based
on environment variables.

#### Scenario: Production mode by default

- **WHEN** no logging environment variables are set
- **THEN** Production mode is activated (INFO level, file only)

#### Scenario: Development mode via AUTOCOMPLETE_DEV

- **GIVEN** environment variable `AUTOCOMPLETE_DEV=1`
- **WHEN** logging is initialized
- **THEN** Development mode is activated (DEBUG level, console + file)

#### Scenario: Troubleshooting mode via RUST_LOG

- **GIVEN** environment variable `RUST_LOG=trace`
- **WHEN** logging is initialized
- **THEN** Troubleshooting mode is activated (TRACE level, JSON format)

#### Scenario: Module-specific trace logging

- **GIVEN** environment variable `RUST_LOG=autocomplete_rs::parser=trace`
- **WHEN** logging is initialized
- **THEN** TRACE level enabled only for parser module

### Requirement: Log File Management

The logging system SHALL write logs to files with automatic rotation and
cleanup.

#### Scenario: Default log directory

- **WHEN** logging is initialized without custom directory
- **THEN** logs are written to `~/.autocomplete-rs/logs/daemon.log`

#### Scenario: Custom log directory

- **GIVEN** environment variable `AUTOCOMPLETE_LOG_DIR=/custom/path`
- **WHEN** logging is initialized
- **THEN** logs are written to `/custom/path/daemon.log`

#### Scenario: Daily log rotation

- **GIVEN** daemon has been running across midnight
- **WHEN** date changes
- **THEN** current log file is renamed to `daemon.log.YYYY-MM-DD`
- **AND** new `daemon.log` file is created

#### Scenario: Size-based rotation

- **GIVEN** log file reaches 50MB
- **WHEN** daemon attempts to write next log
- **THEN** file is rotated immediately (regardless of time)

#### Scenario: Retention cleanup

- **GIVEN** log files older than retention period exist
- **WHEN** daemon starts OR rotation occurs
- **THEN** old log files are deleted
- **AND** retention is 7 days (production), 30 days (dev), 90 days
  (troubleshooting)

### Requirement: Non-Blocking Writes

The logging system SHALL never block the daemon on file I/O operations.

#### Scenario: Log write during request processing

- **GIVEN** daemon is processing a completion request
- **WHEN** log event is generated
- **THEN** log is queued on separate thread
- **AND** request processing continues immediately

#### Scenario: Backpressure handling

- **GIVEN** log queue is full (10,000 messages)
- **WHEN** new log event is generated
- **THEN** DEBUG and TRACE events are dropped
- **AND** WARN and ERROR events are preserved

#### Scenario: Performance overhead

- **GIVEN** logging is enabled in production mode
- **WHEN** measured across 1000 requests
- **THEN** average overhead is less than 50μs per request (<0.5% of 10ms budget)

### Requirement: Request Correlation

The logging system SHALL provide utilities for correlating logs across async
operations.

#### Scenario: Generate unique request ID

- **WHEN** new completion request arrives
- **THEN** unique UUID v4 is generated
- **AND** UUID is attached to tracing span

#### Scenario: Request ID propagates through async

- **GIVEN** request span with request_id field
- **WHEN** async operations are spawned within span
- **THEN** all nested logs include request_id
- **AND** request_id is preserved across `.await` boundaries

#### Scenario: Filter logs by request

- **GIVEN** log file contains multiple requests
- **WHEN** searching for specific request_id
- **THEN** all logs for that request can be found with simple grep

### Requirement: Structured Logging

The logging system SHALL support structured key-value logging for machine
analysis.

#### Scenario: Log with structured fields

- **WHEN** logging event with fields `info!(request_id = %id, "message")`
- **THEN** output includes both message and fields
- **AND** fields are formatted consistently

#### Scenario: JSON output format

- **GIVEN** troubleshooting mode enabled
- **WHEN** log event is written
- **THEN** output is valid JSON Lines format
- **AND** all fields are included in JSON object

#### Scenario: Field types

- **GIVEN** structured field of any supported type (string, number, bool)
- **WHEN** field is logged
- **THEN** field is formatted correctly for the output mode

### Requirement: Performance Monitoring

The logging system SHALL provide spans for automatic performance tracking.

#### Scenario: Span timing

- **GIVEN** tracing span wraps an operation
- **WHEN** operation completes
- **THEN** span duration is automatically logged

#### Scenario: Nested span hierarchy

- **GIVEN** multiple nested spans
- **WHEN** viewing in development mode
- **THEN** hierarchy is visible with indentation
- **AND** timing for each level is shown

#### Scenario: Performance threshold warnings

- **GIVEN** operation exceeds configured threshold (10ms)
- **WHEN** span completes
- **THEN** WARN level log is generated
- **AND** log includes operation name and actual duration

### Requirement: Error Context

The logging system SHALL enrich error logs with contextual information.

#### Scenario: Error with span context

- **GIVEN** error occurs within traced span
- **WHEN** error is logged
- **THEN** log includes all span fields (request_id, etc.)

#### Scenario: Error chain

- **GIVEN** error with multiple levels of context (anyhow chain)
- **WHEN** error is logged
- **THEN** full error chain is included in log

#### Scenario: Error in production

- **GIVEN** production mode enabled
- **WHEN** error occurs
- **THEN** error is always logged (not filtered by level)
- **AND** includes request_id for diagnosis

### Requirement: Configuration API

The logging system SHALL provide a public API for initialization and
configuration.

#### Scenario: Simple initialization

- **WHEN** calling `logging::init()`
- **THEN** logging is configured with environment-based defaults
- **AND** no errors are returned on success

#### Scenario: Custom configuration

- **GIVEN** custom `LogConfig` struct
- **WHEN** calling `logging::init_with_config(config)`
- **THEN** logging uses custom configuration
- **AND** environment variables are ignored

#### Scenario: Initialization failure

- **GIVEN** log directory cannot be created (permissions)
- **WHEN** calling `logging::init()`
- **THEN** error is returned with helpful message

### Requirement: Multiple Output Targets

The logging system SHALL support writing to multiple outputs simultaneously.

#### Scenario: Development mode dual output

- **GIVEN** development mode enabled
- **WHEN** log event is generated
- **THEN** event is written to both console and file
- **AND** console uses pretty format
- **AND** file uses compact format

#### Scenario: Console-only override

- **GIVEN** environment variable `AUTOCOMPLETE_CONSOLE=1`
- **WHEN** logging is initialized in any mode
- **THEN** console output is enabled in addition to file

#### Scenario: File-only in production

- **GIVEN** production mode (default)
- **WHEN** log event is generated
- **THEN** event is written to file only
- **AND** no console output

### Requirement: Format Customization

The logging system SHALL support different output formats per mode.

#### Scenario: Compact format (production)

- **GIVEN** production mode enabled
- **WHEN** log event is written
- **THEN** format is single-line with fixed field order
- **AND** format: `[timestamp] LEVEL module field=value message`

#### Scenario: Pretty format (development)

- **GIVEN** development mode enabled
- **WHEN** log event is written to console
- **THEN** format is multi-line with span hierarchy
- **AND** includes ANSI colors for readability

#### Scenario: JSON format (troubleshooting)

- **GIVEN** troubleshooting mode or `AUTOCOMPLETE_LOG_FORMAT=json`
- **WHEN** log event is written
- **THEN** format is JSON Lines (one object per line)
- **AND** JSON is valid and parseable
