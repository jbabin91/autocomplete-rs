# Shell Integration (Fish) Specification

## ADDED Requirements

### Requirement: Fish Completion System

The fish integration SHALL use fish's native completion system.

#### Scenario: Function registration

- **WHEN** fish sources integration script
- **THEN** completion function `__autocomplete_rs_complete` is defined

#### Scenario: Command line capture

- **GIVEN** user is typing `git checkout`
- **WHEN** completion is triggered
- **THEN** function captures command via `commandline -c`

#### Scenario: Cursor position

- **WHEN** completion is triggered
- **THEN** cursor position is determined via `commandline -C`

### Requirement: Completion Registration

The fish integration SHALL register completions using fish's `complete` command.

#### Scenario: Dynamic completions

- **WHEN** integration is loaded
- **THEN** completions are registered for supported commands

#### Scenario: Completion generation

- **GIVEN** daemon returns suggestions for `git che`
- **WHEN** fish requests completions
- **THEN** suggestions are provided via completion function

### Requirement: Daemon Communication

The fish integration SHALL communicate with daemon via Unix socket.

#### Scenario: Send request

- **GIVEN** command line buffer and cursor position
- **WHEN** completion function runs
- **THEN** JSON request is sent to daemon

#### Scenario: Parse response

- **GIVEN** daemon responds with suggestions
- **WHEN** function processes response
- **THEN** suggestions are converted to fish completion format

### Requirement: Description Support

The fish integration SHALL display suggestion descriptions in fish UI.

#### Scenario: Completion with description

- **GIVEN** suggestion has description "Switch branches"
- **WHEN** displaying in fish
- **THEN** description appears alongside suggestion

### Requirement: Installation

The fish integration SHALL provide easy installation for fish users.

#### Scenario: Auto-install

- **WHEN** user runs `autocomplete-rs install fish`
- **THEN** integration is added to fish config directory

#### Scenario: Config file location

- **WHEN** installing
- **THEN** script is placed in `~/.config/fish/conf.d/autocomplete-rs.fish`

#### Scenario: Immediate activation

- **WHEN** integration is installed
- **THEN** completions work in current session without restart

### Requirement: Fish-Specific Features

The fish integration SHALL leverage fish's advanced completion features.

#### Scenario: Fuzzy matching

- **WHEN** fish's fuzzy matching is enabled
- **THEN** autocomplete-rs suggestions respect fish preferences

#### Scenario: Color support

- **WHEN** fish displays completions
- **THEN** suggestions use fish's color scheme

### Requirement: Error Handling

The fish integration SHALL handle errors gracefully.

#### Scenario: Daemon unavailable

- **GIVEN** daemon is not running
- **WHEN** completion is triggered
- **THEN** daemon is auto-started

#### Scenario: Timeout handling

- **GIVEN** daemon doesn't respond within 1 second
- **WHEN** waiting for completions
- **THEN** completion silently fails without blocking

#### Scenario: Fallback completions

- **GIVEN** autocomplete-rs fails to provide suggestions
- **WHEN** user requests completions
- **THEN** fish's built-in completions still work
