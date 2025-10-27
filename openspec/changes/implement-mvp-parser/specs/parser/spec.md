# Parser Specification

## ADDED Requirements

### Requirement: Command Buffer Tokenization

The parser SHALL tokenize shell command buffers into discrete tokens while
respecting quotes and escape sequences.

#### Scenario: Simple command tokenization

- **GIVEN** command buffer `git checkout main`
- **WHEN** tokenizer processes the buffer
- **THEN** tokens are `["git", "checkout", "main"]`

#### Scenario: Quoted arguments

- **GIVEN** command buffer `git commit -m "fix: bug"`
- **WHEN** tokenizer processes the buffer
- **THEN** tokens are `["git", "commit", "-m", "fix: bug"]`

#### Scenario: Partial command

- **GIVEN** command buffer `git che` with cursor at position 7
- **WHEN** tokenizer processes the buffer
- **THEN** identifies incomplete token `"che"` for completion

### Requirement: Completion Context Detection

The parser SHALL determine what type of completion is needed based on cursor
position and token context.

#### Scenario: Subcommand completion

- **GIVEN** buffer `git` with cursor after space
- **WHEN** parser analyzes context
- **THEN** completion type is SUBCOMMAND

#### Scenario: Option completion

- **GIVEN** buffer `git commit -` with cursor after dash
- **WHEN** parser analyzes context
- **THEN** completion type is OPTION

#### Scenario: Argument completion

- **GIVEN** buffer `git checkout` with cursor after space
- **WHEN** parser analyzes context
- **THEN** completion type is ARGUMENT for checkout subcommand

### Requirement: Spec-Based Suggestion Generation

The parser SHALL match the completion context against loaded specs and generate
ranked suggestions.

#### Scenario: Matching suggestions

- **GIVEN** incomplete token `che` in subcommand context for `git`
- **WHEN** parser queries git spec
- **THEN** suggestions include `checkout`, `cherry-pick`

#### Scenario: Exact match returns subcommand options

- **GIVEN** complete token `checkout` in subcommand context
- **WHEN** parser queries git spec
- **THEN** suggestions are options for checkout (`-b`, `--track`, etc.)

#### Scenario: No matches

- **GIVEN** incomplete token `xyz` in subcommand context for `git`
- **WHEN** parser queries git spec
- **THEN** suggestions list is empty

### Requirement: Performance

The parser SHALL generate suggestions within 5 milliseconds for typical command
buffers.

#### Scenario: Fast parsing

- **GIVEN** buffer `git checkout -b` (typical length)
- **WHEN** parser processes and generates suggestions
- **THEN** total time is less than 5ms
