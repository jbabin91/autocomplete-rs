# Shell Integration (Bash) Specification

## ADDED Requirements

### Requirement: Readline Integration

The bash integration SHALL use readline's programmable completion system.

#### Scenario: Completion function registration

- **WHEN** bash sources integration script
- **THEN** completion function `_autocomplete_rs_complete` is registered

#### Scenario: Capture COMP_LINE

- **GIVEN** user is typing `git checkout main`
- **WHEN** completion is triggered
- **THEN** COMP_LINE contains full command line

#### Scenario: Capture COMP_POINT

- **GIVEN** cursor is at position 12
- **WHEN** completion is triggered
- **THEN** COMP_POINT is 12

### Requirement: Completion Generation

The bash integration SHALL populate COMPREPLY with suggestions from daemon.

#### Scenario: Receive suggestions

- **GIVEN** daemon returns suggestions `["checkout", "cherry-pick"]`
- **WHEN** completion function processes response
- **THEN** COMPREPLY contains both suggestions

#### Scenario: No suggestions

- **GIVEN** daemon returns empty suggestions
- **WHEN** completion function processes response
- **THEN** COMPREPLY is empty (no completion)

### Requirement: Daemon Communication

The bash integration SHALL communicate with daemon via Unix socket.

#### Scenario: Send completion request

- **GIVEN** COMP_LINE is `git che` and COMP_POINT is 7
- **WHEN** completion function runs
- **THEN** JSON request is sent to daemon with buffer and cursor

#### Scenario: Handle daemon not running

- **GIVEN** daemon is not running
- **WHEN** completion is triggered
- **THEN** daemon is auto-started

### Requirement: Keybinding

The bash integration SHALL support configurable key binding.

#### Scenario: Default Tab binding

- **WHEN** integration is loaded
- **THEN** Tab key triggers autocomplete-rs completion

#### Scenario: Custom binding

- **GIVEN** user sets custom binding in bashrc
- **WHEN** integration is loaded
- **THEN** custom key triggers completion

### Requirement: Installation

The bash integration SHALL provide easy installation.

#### Scenario: Auto-install

- **WHEN** user runs `autocomplete-rs install bash`
- **THEN** integration is added to ~/.bashrc

#### Scenario: Manual installation

- **WHEN** user sources shell-integration/bash.sh
- **THEN** completion is activated

### Requirement: Error Handling

The bash integration SHALL handle errors without breaking the shell.

#### Scenario: Socket timeout

- **GIVEN** daemon doesn't respond within 1 second
- **WHEN** completion is triggered
- **THEN** completion silently fails without blocking shell

#### Scenario: Malformed response

- **GIVEN** daemon sends invalid JSON
- **WHEN** parsing response
- **THEN** error is logged and no completions shown
