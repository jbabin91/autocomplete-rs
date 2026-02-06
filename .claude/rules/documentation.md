---
paths:
  - 'docs/**/*.md'
  - 'README.md'
  - 'docs/CONTRIBUTING.md'
---

# Documentation Rules

## Reality Check

This project is pre-alpha. Much of the existing documentation describes planned features as if they exist. When editing docs:

- Clearly distinguish what IS implemented vs what is PLANNED
- Don't describe unimplemented features in present tense
- Use "planned", "future", or "(Phase N)" markers for upcoming work

## What Actually Exists Today

- Daemon: basic Unix socket server with hardcoded empty suggestions
- Parser: stub (returns empty vec)
- Inline dropdown: not yet implemented (old Ratatui TUI removed)
- Shell integration: zsh ZLE widget (functional but no real completions)

## Doc Structure

- `docs/adr/` — Architecture Decision Records (use ADR template)
- `docs/design/` — Design specs (overview, daemon, parser, tui, configuration)
- `docs/development/` — Developer guides (setup, structure, testing, contributing)
- `docs/research/` — Industry analysis and findings
- `docs/user-guide/` — End-user docs (install, config, troubleshooting)

## Conventions

- Use present tense for existing features ("The daemon listens...")
- Use future tense or markers for planned features ("Phase 2 will add...")
- Link to beads issues (`bd list`, `bd ready`) instead of removed ROADMAP.md
- Keep ADRs immutable once accepted (add new ADRs to supersede)
