---
paths:
  - '**/*.md'
---

# Documentation Rules

## Reality Check

This project is pre-alpha. Much of the existing documentation describes planned features as if they exist. When editing docs:

- Clearly distinguish what IS implemented vs what is PLANNED
- Don't describe unimplemented features in present tense
- Use "planned", "future", or "(Phase N)" markers for upcoming work

## What Actually Exists Today

- Daemon: Unix socket server with hardcoded empty suggestions
- Storage: local turso (SQLite-compatible) database for session lifecycle, diagnostics, and metrics
- CLI: `daemon`, `stop`, `status`, `complete`, `install`, `diagnose` subcommands
- Parser: FSM tokenizer + context analysis (classifies completion context, returns empty suggestions pending spec system)
- Inline dropdown: not yet implemented (old Ratatui TUI removed)
- Shell integration: zsh ZLE widget (functional but no real completions)

## Doc Structure

- `docs/adr/` — Architecture Decision Records (use ADR template)
- `docs/design/` — Design specs (overview, daemon, parser, tui, configuration)
- `docs/development/` — Developer guides (setup, structure, testing, contributing)
- `docs/research/` — Industry analysis and findings
- `docs/user-guide/` — End-user docs (install, config, troubleshooting)

## Command References

- Don't duplicate exact command flags in prose — a raw `cargo clippy --all-targets
  --all-features` in a doc drifts the moment `.mise.toml` changes
- Use `mise run <task>`, or point at the source config (`.mise.toml`, `lefthook.yml`,
  `ci.yml`). `.claude/rules/tooling.md` carries the same rule for config files, but
  its path scope never loads it while you are editing markdown
- AGENTS.md's concise command block is the deliberate exception

## Snippet Fidelity

- Code snippets in design docs must match the real signatures, types, and serde
  attributes in `src/` — a snippet that drifts is read as current and believed
- Watch the details that drift silently: type widths (`AtomicU32` where the code
  uses `AtomicU64`), `rename_all` values, and `Option<T>` on a required field
- The suite runs `cargo nextest run`; a bare `cargo test` in docs is wrong.
  `cargo test --doc` is correct and expected, since nextest does not run doctests
- Socket path references mention the `AUTOCOMPLETE_RS_SOCKET` override

## Conventions

- Use present tense for existing features ("The daemon listens...")
- Use future tense or markers for planned features ("Phase 2 will add...")
- Contributor-facing docs (README, CONTRIBUTING, docs/development/) link to
  GitHub Issues, never to `bd` commands. Beads is backed by a Dolt remote
  rather than this repository, so a contributor who clones cannot run them.
- Maintainer-facing files (AGENTS.md, .claude/rules/) may reference `bd`
  freely
- Keep ADRs immutable once accepted (add new ADRs to supersede)
- Qualify spike measurements as observations, not guarantees — say
  "observed sub-ms on development hardware" not "sub-ms latency". Spike
  data varies by machine and load; stating it as fact misleads readers
  into treating it as a guaranteed property

## Single Source of Truth

- **MSRV / Rust version**: `Cargo.toml` `rust-version` field is the canonical
  source. Never hardcode version numbers (e.g. "1.88+") in prose — reference
  `Cargo.toml` instead. CI already reads it dynamically. The README badge uses a
  dynamic shields.io query against `Cargo.toml` on `main`.
- **Edition**: `Cargo.toml` `edition` field. `rustfmt.toml` mirrors it but the
  canonical source is Cargo.
- When a value is defined in a config file, docs should say "see X in Y" rather
  than repeating the value — repeated values drift out of sync on every bump.
