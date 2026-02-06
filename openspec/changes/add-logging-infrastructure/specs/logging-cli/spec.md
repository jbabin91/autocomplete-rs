# Logging CLI Specification

## ADDED Requirements

### Requirement: Log Tail Command

The CLI SHALL provide a command to stream logs in real-time.

#### Scenario: Tail current log file

- **WHEN** running `autocomplete-rs logs tail`
- **THEN** current log file is streamed to stdout
- **AND** new log lines appear as they are written

#### Scenario: Tail with level filter

- **GIVEN** command `autocomplete-rs logs tail --level error`
- **WHEN** logs are streamed
- **THEN** only ERROR level logs are displayed
- **AND** other levels are filtered out

#### Scenario: Tail with follow

- **GIVEN** command `autocomplete-rs logs tail -f`
- **WHEN** daemon writes new logs
- **THEN** new lines appear immediately
- **AND** command continues running until Ctrl-C

#### Scenario: Tail specific number of lines

- **GIVEN** command `autocomplete-rs logs tail -n 50`
- **WHEN** command runs
- **THEN** last 50 lines are displayed
- **AND** command exits (no follow)

#### Scenario: Tail with timestamp filter

- **GIVEN** command `autocomplete-rs logs tail --since "1 hour ago"`
- **WHEN** logs are displayed
- **THEN** only logs from last hour are shown

### Requirement: Log Dump Command

The CLI SHALL provide a command to collect and package logs for bug reports.

#### Scenario: Basic dump

- **WHEN** running `autocomplete-rs logs dump`
- **THEN** zip file is created with current logs
- **AND** filename is `autocomplete-rs-logs-YYYY-MM-DD-HHMMSS.zip`
- **AND** privacy notice is included in zip

#### Scenario: Dump with rotation history

- **GIVEN** command `autocomplete-rs logs dump --include-rotated`
- **WHEN** dump is created
- **THEN** all rotated log files are included
- **AND** compressed logs are decompressed before zipping

#### Scenario: Dump with redaction warning

- **GIVEN** production mode logs (redacted buffers)
- **WHEN** dump is created
- **THEN** console shows: "✓ Logs are redacted (production mode)"
- **AND** README in zip explains redaction

#### Scenario: Dump with full buffers warning

- **GIVEN** troubleshooting mode logs (full buffers)
- **WHEN** dump is created
- **THEN** console shows: "⚠️ Logs contain full command buffers - review before sharing"
- **AND** user must confirm with --yes flag or interactive prompt

#### Scenario: Dump with system info

- **GIVEN** command `autocomplete-rs logs dump --include-system-info`
- **WHEN** dump is created
- **THEN** system-info.txt is included with:
- **AND** OS version, Rust version, daemon version
- **AND** shell type and version
- **AND** installation method

#### Scenario: Dump output path

- **GIVEN** command `autocomplete-rs logs dump -o /custom/path/report.zip`
- **WHEN** dump is created
- **THEN** zip is saved to specified path
- **AND** parent directory is created if needed

### Requirement: Log Analyze Command

The CLI SHALL provide a command to automatically detect common issues.

#### Scenario: Analyze current logs

- **WHEN** running `autocomplete-rs logs analyze`
- **THEN** current log file is scanned for known issues
- **AND** report is printed to console

#### Scenario: Detect high latency

- **GIVEN** logs contain requests exceeding 10ms
- **WHEN** analyze runs
- **THEN** report includes: "⚠️ High latency detected (N requests > 10ms)"
- **AND** shows request_ids and durations

#### Scenario: Detect connection failures

- **GIVEN** logs contain socket connection errors
- **WHEN** analyze runs
- **THEN** report includes: "❌ Connection failures detected (N occurrences)"
- **AND** suggests checking daemon status

#### Scenario: Detect parsing errors

- **GIVEN** logs contain JSON parse errors
- **WHEN** analyze runs
- **THEN** report includes: "❌ Protocol errors detected"
- **AND** suggests version mismatch between client/daemon

#### Scenario: Detect log backpressure

- **GIVEN** logs contain "log queue full" warnings
- **WHEN** analyze runs
- **THEN** report includes: "⚠️ Log backpressure detected"
- **AND** suggests reducing log level or increasing queue size

#### Scenario: Performance summary

- **WHEN** analyze runs
- **THEN** report includes performance summary:
- **AND** average, median, p95, p99 request latencies
- **AND** slowest requests with request_ids
- **AND** performance trend (improving/degrading)

#### Scenario: Clean bill of health

- **GIVEN** no issues found in logs
- **WHEN** analyze runs
- **THEN** report shows: "✓ No issues detected"
- **AND** shows summary stats (total requests, uptime, etc.)

### Requirement: Log Clean Command

The CLI SHALL provide a command to manually clean old logs.

#### Scenario: Clean with default retention

- **WHEN** running `autocomplete-rs logs clean`
- **THEN** logs older than current mode's retention are deleted
- **AND** console shows: "Deleted N log files (freed X MB)"

#### Scenario: Clean with custom retention

- **GIVEN** command `autocomplete-rs logs clean --older-than "30 days"`
- **WHEN** command runs
- **THEN** logs older than 30 days are deleted
- **AND** current mode's retention is ignored

#### Scenario: Clean with dry-run

- **GIVEN** command `autocomplete-rs logs clean --dry-run`
- **WHEN** command runs
- **THEN** files to be deleted are listed
- **AND** no files are actually deleted

#### Scenario: Clean all logs

- **GIVEN** command `autocomplete-rs logs clean --all`
- **WHEN** user confirms interactive prompt
- **THEN** all log files except current daemon.log are deleted
- **AND** daemon.log is truncated to zero bytes

#### Scenario: Clean requires confirmation

- **GIVEN** command `autocomplete-rs logs clean --all`
- **WHEN** running without --yes flag
- **THEN** interactive prompt asks: "Delete all logs? This cannot be undone. [y/N]"
- **AND** command aborts if user responds 'n' or presses Enter

### Requirement: Log Config Command

The CLI SHALL provide a command to show current logging configuration.

#### Scenario: Show current config

- **WHEN** running `autocomplete-rs logs config`
- **THEN** console displays current configuration:
- **AND** mode (Production/Development/Troubleshooting)
- **AND** log level
- **AND** output targets (console/file)
- **AND** log directory
- **AND** retention period
- **AND** redaction status

#### Scenario: Config with JSON output

- **GIVEN** command `autocomplete-rs logs config --json`
- **WHEN** command runs
- **THEN** configuration is output as JSON
- **AND** JSON includes all config fields

#### Scenario: Config shows environment variables

- **WHEN** `logs config` runs
- **THEN** console shows which env vars are set:
- **AND** AUTOCOMPLETE_DEV=1 (if set)
- **AND** RUST_LOG=value (if set)
- **AND** AUTOCOMPLETE_LOG_DIR=path (if set)
- **AND** AUTOCOMPLETE_CONSOLE=1 (if set)

#### Scenario: Config shows daemon status

- **WHEN** `logs config` runs
- **THEN** console shows if daemon is running
- **AND** shows daemon PID (if running)
- **AND** shows uptime (if running)

### Requirement: Log List Command

The CLI SHALL provide a command to list all log files.

#### Scenario: List log files

- **WHEN** running `autocomplete-rs logs list`
- **THEN** all log files are listed with:
- **AND** filename
- **AND** size (human readable)
- **AND** modification time
- **AND** age (e.g., "2 days ago")

#### Scenario: List with size total

- **WHEN** `logs list` runs
- **THEN** footer shows total: "Total: N files (X MB)"

#### Scenario: List sorted by time

- **GIVEN** command `autocomplete-rs logs list --sort time`
- **WHEN** command runs
- **THEN** files are listed newest first

#### Scenario: List sorted by size

- **GIVEN** command `autocomplete-rs logs list --sort size`
- **WHEN** command runs
- **THEN** files are listed largest first

### Requirement: Log Search Command

The CLI SHALL provide a command to search across all logs.

#### Scenario: Search by request_id

- **GIVEN** command `autocomplete-rs logs search --request-id abc123`
- **WHEN** command runs
- **THEN** all logs with matching request_id are displayed
- **AND** logs are displayed in chronological order
- **AND** shows which file each log came from

#### Scenario: Search by text pattern

- **GIVEN** command `autocomplete-rs logs search "connection error"`
- **WHEN** command runs
- **THEN** all logs containing text are displayed
- **AND** matching text is highlighted

#### Scenario: Search with grep-like regex

- **GIVEN** command `autocomplete-rs logs search -E "latency_ms=[0-9]{2,}"`
- **WHEN** command runs
- **THEN** all logs matching regex are displayed

#### Scenario: Search with context lines

- **GIVEN** command `autocomplete-rs logs search "error" -C 3`
- **WHEN** command runs
- **THEN** matching lines plus 3 lines before and after are shown

#### Scenario: Search across rotated logs

- **WHEN** search runs
- **THEN** current and all rotated logs are searched
- **AND** compressed logs are automatically decompressed

### Requirement: Help and Documentation

The CLI SHALL provide help text for all log commands.

#### Scenario: Logs command help

- **WHEN** running `autocomplete-rs logs --help`
- **THEN** help text shows all subcommands
- **AND** brief description of each subcommand

#### Scenario: Subcommand help

- **WHEN** running `autocomplete-rs logs tail --help`
- **THEN** help text shows all options
- **AND** examples of common usage
- **AND** related commands

#### Scenario: Logs command without arguments

- **WHEN** running `autocomplete-rs logs`
- **THEN** shows help text (same as --help)
- **AND** suggests common commands
