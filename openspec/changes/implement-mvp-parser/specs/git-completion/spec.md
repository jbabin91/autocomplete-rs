# Git Completion Specification

## ADDED Requirements

### Requirement: Git Subcommand Completion

The git completion spec SHALL provide suggestions for common git subcommands.

#### Scenario: Checkout subcommand

- **GIVEN** incomplete token `che`
- **WHEN** matching against git spec
- **THEN** `checkout` is in suggestions

#### Scenario: Commit subcommand

- **GIVEN** incomplete token `com`
- **WHEN** matching against git spec
- **THEN** `commit` is in suggestions

#### Scenario: List all subcommands

- **GIVEN** empty token (cursor after `git`)
- **WHEN** matching against git spec
- **THEN** suggestions include `checkout`, `commit`, `add`, `push`, `pull`,
  `status`, `branch`, `merge`, `rebase`, `log`

### Requirement: Git Option Completion

The git completion spec SHALL provide option suggestions for subcommands.

#### Scenario: Commit options

- **GIVEN** context is `git commit -`
- **WHEN** matching against git spec
- **THEN** suggestions include `-m`, `--amend`, `--no-verify`, `-a`

#### Scenario: Checkout options

- **GIVEN** context is `git checkout -`
- **WHEN** matching against git spec
- **THEN** suggestions include `-b`, `--track`, `-B`, `--detach`

#### Scenario: Global git options

- **GIVEN** context is `git -`
- **WHEN** matching against git spec
- **THEN** suggestions include `--version`, `--help`, `-C`

### Requirement: Option Descriptions

The git completion spec SHALL include brief descriptions for each option and
subcommand.

#### Scenario: Commit message option

- **GIVEN** suggestion for `-m` option
- **WHEN** displaying suggestion
- **THEN** description is "Commit message"

#### Scenario: Checkout branch option

- **GIVEN** suggestion for `-b` option
- **WHEN** displaying suggestion
- **THEN** description is "Create and checkout new branch"

### Requirement: Hardcoded Implementation

The MVP git spec SHALL be hardcoded in Rust (not parsed from external files).

#### Scenario: Spec availability

- **GIVEN** autocomplete-rs binary is built
- **WHEN** daemon starts
- **THEN** git spec is immediately available without loading external files

#### Scenario: Minimal set

- **GIVEN** hardcoded git spec
- **WHEN** listing available subcommands
- **THEN** spec includes at minimum: `checkout`, `commit`, `add`, `push`,
  `pull`, `status`
