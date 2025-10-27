# Shell Integration (Zsh) Specification

## ADDED Requirements

### Requirement: ZLE Widget

The integration SHALL provide a ZLE widget that captures command buffer and
triggers completion.

#### Scenario: Widget registration

- **WHEN** zsh sources integration script
- **THEN** widget `_autocomplete_rs_widget` is registered with ZLE

#### Scenario: Capture buffer

- **GIVEN** user is typing `git che`
- **WHEN** widget is triggered
- **THEN** widget captures current buffer value `"git che"`

#### Scenario: Capture cursor position

- **GIVEN** cursor is at position 7 in buffer
- **WHEN** widget is triggered
- **THEN** widget captures cursor position `7`

### Requirement: Daemon Communication

The widget SHALL communicate with the daemon via Unix socket.

#### Scenario: Send request

- **GIVEN** buffer `"git che"` and cursor `7`
- **WHEN** widget sends request to daemon
- **THEN** request JSON contains `{"buffer": "git che", "cursor": 7}`

#### Scenario: Receive response

- **WHEN** daemon sends response
- **THEN** widget receives and parses JSON response

#### Scenario: Daemon not running

- **GIVEN** daemon is not running
- **WHEN** widget attempts to connect
- **THEN** widget starts daemon automatically

### Requirement: Keybinding

The integration SHALL bind the widget to a configurable key combination.

#### Scenario: Default binding

- **WHEN** integration is loaded
- **THEN** widget is bound to Alt+Space by default

#### Scenario: Custom binding

- **GIVEN** user sets custom keybinding in config
- **WHEN** integration is loaded
- **THEN** widget is bound to custom key

### Requirement: Completion Insertion

The widget SHALL insert the selected completion into the command buffer.

#### Scenario: Insert full completion

- **GIVEN** buffer `"git che"` and user selects `"checkout"`
- **WHEN** completion is confirmed
- **THEN** buffer becomes `"git checkout "`

#### Scenario: Replace partial token

- **GIVEN** buffer `"git comm"` and user selects `"commit"`
- **WHEN** completion is confirmed
- **THEN** buffer becomes `"git commit "`

#### Scenario: Cancel completion

- **GIVEN** user presses Esc in TUI
- **WHEN** widget receives no selection
- **THEN** buffer remains unchanged

### Requirement: Daemon Auto-Start

The integration SHALL automatically start the daemon if it's not running.

#### Scenario: First invocation

- **GIVEN** daemon is not running
- **WHEN** widget is first triggered
- **THEN** daemon is started in background

#### Scenario: Daemon already running

- **GIVEN** daemon is running
- **WHEN** widget is triggered
- **THEN** existing daemon is used (no duplicate)

#### Scenario: Shell startup

- **WHEN** user opens new shell
- **THEN** integration checks for running daemon and starts if needed

### Requirement: Error Handling

The widget SHALL handle errors gracefully without disrupting the shell.

#### Scenario: Socket connection timeout

- **GIVEN** daemon is unresponsive
- **WHEN** widget attempts connection
- **THEN** widget times out after 1 second and shows error message

#### Scenario: Invalid response

- **GIVEN** daemon sends malformed response
- **WHEN** widget receives response
- **THEN** widget shows error and doesn't modify buffer

#### Scenario: Silent failure fallback

- **GIVEN** any error occurs
- **WHEN** widget encounters error
- **THEN** buffer remains unchanged (fail-safe behavior)

### Requirement: Installation

The integration SHALL provide easy installation for zsh users.

#### Scenario: Install command

- **WHEN** user runs `autocomplete-rs install zsh`
- **THEN** integration script is added to `.zshrc`

#### Scenario: Manual installation

- **GIVEN** user wants manual control
- **WHEN** user sources `shell-integration/zsh.zsh`
- **THEN** integration is activated for that session
