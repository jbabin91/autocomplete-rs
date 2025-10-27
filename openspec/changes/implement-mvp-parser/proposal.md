# Implement MVP Parser with Git Completion

**Priority:** 1B **Phase:** 1 (MVP) **Dependencies:**
`add-foundation-architecture` **Blocks:** `add-fig-spec-parser`

## Why

We need a working end-to-end proof of concept to validate the architecture and
demonstrate that autocomplete-rs can successfully parse command buffers and
generate completions. This MVP will establish the foundation for all future
completion specs.

## What Changes

- Implement command buffer tokenizer and parser
- Create hardcoded git completion spec (proof of concept)
- Wire daemon to parser for completion generation
- Implement daemon-to-TUI communication
- Enable end-to-end testing with zsh integration

## Impact

- Affected specs:
  - `parser` (new capability)
  - `git-completion` (new capability)
  - `daemon` (modified - wire to parser)
- Affected code:
  - `src/parser/mod.rs` - implement tokenization and matching logic
  - `src/specs/git.rs` - hardcoded git spec
  - `src/daemon/mod.rs` - connect parser and respond with suggestions
  - `src/main.rs` - wire complete command to daemon
- Dependencies: None (using existing crates)
- Migration: N/A (net new functionality)
