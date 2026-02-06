# Logging Privacy Specification

## ADDED Requirements

### Requirement: Buffer Redaction

The logging system SHALL redact sensitive data from command buffers by default.

#### Scenario: Production mode redaction

- **GIVEN** production mode enabled (default)
- **WHEN** logging command buffer "git commit -m 'secret password'"
- **THEN** logged value is "git\*\*\*ord" with length (32)
- **AND** middle portion is replaced with "\*\*\*"
- **AND** first 3 and last 3 characters are preserved

#### Scenario: Short buffer redaction

- **GIVEN** buffer length less than 10 characters
- **WHEN** logging buffer "git log"
- **THEN** logged value is "git\*\*\*log" (8)
- **AND** first 3 and last 3 chars preserved even for short buffers

#### Scenario: Very short buffer

- **GIVEN** buffer length less than 6 characters
- **WHEN** logging buffer "ls"
- **THEN** logged value is "\*\*\*" (2)
- **AND** entire buffer replaced with "\*\*\*"

#### Scenario: Development mode no redaction

- **GIVEN** AUTOCOMPLETE_DEV=1 environment variable
- **WHEN** logging any buffer
- **THEN** full buffer is logged without redaction
- **AND** logs show complete command text

#### Scenario: Troubleshooting mode explicit opt-in

- **GIVEN** RUST_LOG=trace but no AUTOCOMPLETE_LOG_FULL_BUFFERS
- **WHEN** logging buffer
- **THEN** buffer is still redacted (same as production)

#### Scenario: Troubleshooting mode full buffers

- **GIVEN** AUTOCOMPLETE_LOG_FULL_BUFFERS=1 environment variable
- **WHEN** daemon starts
- **THEN** warning is logged: "⚠️ Full buffer logging enabled - may expose sensitive data"
- **AND** subsequent buffers are logged without redaction

#### Scenario: Redaction in structured fields

- **GIVEN** structured log with buffer field: `info!(buffer = %buf, "request")`
- **WHEN** production mode
- **THEN** field value is redacted: `buffer="git***mit" (10)`

### Requirement: Sensitive Field Detection

The logging system SHALL automatically detect and redact common sensitive patterns.

#### Scenario: Password detection in buffer

- **GIVEN** buffer contains "password=" or "pwd=" pattern
- **WHEN** logging in any mode
- **THEN** value after = is replaced with "[REDACTED]"
- **AND** pattern examples: "password=secret123" → "password=[REDACTED]"

#### Scenario: API key detection

- **GIVEN** buffer contains "api_key=", "apikey=", or "token=" pattern
- **WHEN** logging in any mode
- **THEN** value is replaced with "[REDACTED]"

#### Scenario: URL credentials

- **GIVEN** buffer contains URL with credentials: "<https://user:pass@example.com>"
- **WHEN** logging in any mode
- **THEN** credentials are redacted: "https://[REDACTED]@example.com"

#### Scenario: SSH key paths

- **GIVEN** buffer contains path to SSH keys: "~/.ssh/id_rsa"
- **WHEN** logging in any mode
- **THEN** path is logged but key contents never captured

#### Scenario: Environment variable values

- **GIVEN** buffer contains "export SECRET=value"
- **WHEN** logging in any mode
- **THEN** value is redacted: "export SECRET=[REDACTED]"

#### Scenario: Detection override in dev mode

- **GIVEN** development mode enabled
- **WHEN** buffer contains password pattern
- **THEN** full value is logged (dev needs full context)

### Requirement: Redaction Configuration

The logging system SHALL provide configuration for redaction behavior.

#### Scenario: Custom redaction patterns

- **GIVEN** config with custom patterns: `redact_patterns = ["custom_secret=.*"]`
- **WHEN** buffer matches custom pattern
- **THEN** value is redacted using same rules

#### Scenario: Disable redaction warning

- **GIVEN** AUTOCOMPLETE_LOG_FULL_BUFFERS=1
- **WHEN** daemon starts
- **THEN** warning appears in both console and log file
- **AND** warning includes timestamp and mode

#### Scenario: Redaction status in config

- **WHEN** running `autocomplete-rs logs config`
- **THEN** output shows redaction status:
- **AND** "Buffer redaction: ENABLED" (production)
- **AND** "Buffer redaction: DISABLED (dev mode)" (development)

### Requirement: Log File Protection

The logging system SHALL ensure log files have secure permissions.

#### Scenario: Log file permissions

- **WHEN** log file is created
- **THEN** file permissions are set to 0600 (owner read/write only)
- **AND** directory permissions are 0700 (owner access only)

#### Scenario: Log directory creation

- **WHEN** logging is initialized
- **THEN** log directory is created with secure permissions
- **AND** intermediate directories inherit restrictive permissions

#### Scenario: Rotation preserves permissions

- **GIVEN** log file is rotated
- **WHEN** new log file is created
- **THEN** new file has same restrictive permissions (0600)

#### Scenario: Permission failure

- **GIVEN** log directory has wrong permissions
- **WHEN** logging is initialized
- **THEN** warning is logged to stderr
- **AND** permissions are corrected automatically if possible

### Requirement: Dump Privacy Protection

The log dump command SHALL ensure users are aware of privacy implications.

#### Scenario: Production dump auto-safe

- **GIVEN** production mode logs
- **WHEN** running `logs dump`
- **THEN** console shows: "✓ Logs are redacted (safe to share)"
- **AND** dump completes without additional confirmation

#### Scenario: Dev mode dump warning

- **GIVEN** development mode logs
- **WHEN** running `logs dump`
- **THEN** console shows: "⚠️ Logs may contain full command buffers"
- **AND** user must confirm: "Share these logs? [y/N]"

#### Scenario: Troubleshooting mode dump warning

- **GIVEN** troubleshooting mode with full buffers
- **WHEN** running `logs dump`
- **THEN** console shows: "⚠️ WARNING: Logs contain UNREDACTED data"
- **AND** user must type "yes" to confirm (not just 'y')
- **AND** instructions suggest reviewing zip contents before sharing

#### Scenario: Privacy notice in dump

- **WHEN** dump zip is created
- **THEN** PRIVACY_NOTICE.txt is included
- **AND** notice explains what data is included
- **AND** notice shows redaction status
- **AND** notice warns about potential sensitive data

#### Scenario: Bypass confirmation with flag

- **GIVEN** command `logs dump --yes`
- **WHEN** command runs
- **THEN** confirmation prompts are skipped
- **AND** warnings are still printed to console

### Requirement: Structured Field Privacy

The logging system SHALL apply redaction to all structured fields, not just messages.

#### Scenario: Structured buffer field

- **GIVEN** log with field: `info!(request_buffer = %buffer, "request")`
- **WHEN** production mode
- **THEN** field value is redacted: `request_buffer="git***mit" (10)`

#### Scenario: Error context with buffer

- **GIVEN** error logged with buffer in context
- **WHEN** error is formatted
- **THEN** buffer in context is redacted
- **AND** error chain is preserved

#### Scenario: Span fields with sensitive data

- **GIVEN** span created with buffer field: `span!(buffer = %buf)`
- **WHEN** span is logged
- **THEN** buffer field is redacted in all span logs

### Requirement: Documentation and Warnings

The logging system SHALL clearly document privacy behavior.

#### Scenario: Startup mode notice

- **WHEN** daemon starts in production mode
- **THEN** no privacy warning (safe defaults)

#### Scenario: Startup dev mode notice

- **WHEN** daemon starts with AUTOCOMPLETE_DEV=1
- **THEN** log shows: "Development mode: Full buffers logged"

#### Scenario: Startup full buffers warning

- **WHEN** daemon starts with AUTOCOMPLETE_LOG_FULL_BUFFERS=1
- **THEN** console shows: "⚠️ Full buffer logging enabled - may expose sensitive data"
- **AND** log file contains same warning

#### Scenario: Documentation privacy section

- **GIVEN** user reads README or documentation
- **THEN** privacy section explains:
- **AND** default redaction behavior
- **AND** how to enable full logging for troubleshooting
- **AND** how to verify logs before sharing

### Requirement: Audit and Compliance

The logging system SHALL provide tools to audit privacy compliance.

#### Scenario: Scan logs for potential secrets

- **GIVEN** command `autocomplete-rs logs audit-privacy`
- **WHEN** command runs
- **THEN** logs are scanned for common secret patterns
- **AND** potential issues are reported with line numbers

#### Scenario: Verify redaction applied

- **GIVEN** production mode logs
- **WHEN** audit runs
- **THEN** tool verifies buffers follow redaction format
- **AND** confirms no full buffers are present

#### Scenario: Report full buffer usage

- **GIVEN** logs created with AUTOCOMPLETE_LOG_FULL_BUFFERS=1
- **WHEN** audit runs
- **THEN** report shows: "⚠️ Full buffer logging was used"
- **AND** shows time range when full logging was active

### Requirement: Privacy by Default

The logging system SHALL use privacy-preserving defaults requiring explicit opt-in for full logging.

#### Scenario: Default initialization privacy-safe

- **WHEN** calling `logging::init()` with no environment variables
- **THEN** production mode is activated
- **AND** buffer redaction is enabled
- **AND** sensitive pattern detection is enabled

#### Scenario: Explicit opt-in required

- **GIVEN** user wants full buffer logging
- **WHEN** user sets AUTOCOMPLETE_LOG_FULL_BUFFERS=1
- **THEN** full logging is enabled
- **AND** warning is displayed
- **AND** setting is not persisted (requires manual re-enable after daemon restart)

#### Scenario: No config file storage of sensitive settings

- **GIVEN** configuration system
- **WHEN** redaction settings are considered
- **THEN** full buffer mode cannot be enabled via config file
- **AND** must be set via environment variable per session
- **AND** prevents accidental persistent full logging

### Requirement: Log Analysis Privacy

Log analysis tools SHALL respect privacy when displaying results.

#### Scenario: Analyze respects redaction

- **WHEN** running `logs analyze`
- **THEN** displayed log snippets show redacted buffers
- **AND** no full buffers are exposed in output

#### Scenario: Search respects redaction

- **WHEN** running `logs search <pattern>`
- **THEN** results show redacted buffers
- **AND** search can still find patterns in non-redacted fields

#### Scenario: Tail respects redaction

- **WHEN** running `logs tail`
- **THEN** streamed logs show redacted buffers
- **AND** matches original log file content
