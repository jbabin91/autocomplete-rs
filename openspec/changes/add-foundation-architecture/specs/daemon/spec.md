# Daemon Specification

## ADDED Requirements

### Requirement: Unix Socket Server

The daemon SHALL listen on a Unix domain socket for completion requests.

#### Scenario: Socket creation

- **WHEN** daemon starts
- **THEN** Unix socket is created at `/tmp/autocomplete-rs.sock`

#### Scenario: Concurrent connections

- **GIVEN** daemon is running
- **WHEN** multiple clients connect simultaneously
- **THEN** all connections are handled concurrently

#### Scenario: Socket cleanup

- **WHEN** daemon exits (gracefully or via signal)
- **THEN** socket file is removed from filesystem

### Requirement: Request Processing

The daemon SHALL receive JSON requests containing command buffer and cursor
position.

#### Scenario: Valid request

- **GIVEN** JSON request `{"buffer": "git che", "cursor": 7}`
- **WHEN** daemon receives request
- **THEN** request is parsed successfully

#### Scenario: Invalid JSON

- **GIVEN** malformed JSON request
- **WHEN** daemon receives request
- **THEN** error response is sent to client

#### Scenario: Missing fields

- **GIVEN** JSON request missing required field
- **WHEN** daemon validates request
- **THEN** error response indicates missing field

### Requirement: Response Generation

The daemon SHALL return JSON responses containing completion suggestions.

#### Scenario: Successful completion

- **GIVEN** valid request for `git che`
- **WHEN** daemon generates suggestions
- **THEN** response contains
  `{"suggestions": [{"text": "checkout", "description": "Switch branches"}]}`

#### Scenario: No suggestions

- **GIVEN** request with no matching completions
- **WHEN** daemon generates suggestions
- **THEN** response contains `{"suggestions": []}`

### Requirement: Performance

The daemon SHALL process requests and return responses within 10 milliseconds.

#### Scenario: Fast response time

- **GIVEN** typical completion request
- **WHEN** measured from request receipt to response send
- **THEN** total time is less than 10ms

### Requirement: Daemon Lifecycle

The daemon SHALL support start, stop, and status operations.

#### Scenario: Start daemon

- **WHEN** user runs `autocomplete-rs daemon`
- **THEN** daemon starts in background and creates socket

#### Scenario: Check if running

- **WHEN** checking daemon status
- **THEN** system can determine if daemon is running via socket existence

#### Scenario: Prevent duplicate daemons

- **GIVEN** daemon is already running
- **WHEN** attempt to start another daemon
- **THEN** new process detects existing daemon and exits

### Requirement: Error Recovery

The daemon SHALL handle connection errors gracefully without crashing.

#### Scenario: Client disconnects mid-request

- **GIVEN** client connection
- **WHEN** client disconnects before receiving response
- **THEN** daemon cleans up connection and continues serving other clients

#### Scenario: Malformed request handling

- **GIVEN** malformed or oversized request
- **WHEN** daemon processes request
- **THEN** daemon sends error response and continues operation
