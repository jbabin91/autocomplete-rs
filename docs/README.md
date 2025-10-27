# autocomplete-rs Documentation

Welcome to the autocomplete-rs documentation!

## Table of Contents

### For Users

- [Installation Guide](user-guide/installation.md) - How to install and set up
  autocomplete-rs
- [Configuration Guide](user-guide/configuration.md) - Customizing themes and
  behavior
- [Troubleshooting](user-guide/troubleshooting.md) - Common issues and solutions

### For Developers

- [Getting Started](development/getting-started.md) - Set up development
  environment
- [Project Structure](development/project-structure.md) - Understanding the
  codebase
- [Testing](development/testing.md) - How to run and write tests
- [Contributing](development/contributing.md) - How to contribute to the project

### Architecture

- [Architecture Overview](architecture/overview.md) - High-level system design
- [Daemon](architecture/daemon.md) - Unix socket server and request handling
- [Parser](architecture/parser.md) - Command buffer parsing and spec matching
- [TUI](architecture/tui.md) - Terminal UI rendering with Ratatui

### Architecture Decision Records (ADRs)

Decision records document the "why" behind our technical choices:

- [ADR-0001: Use Rust](adr/0001-use-rust.md) - Why Rust over Bun/TypeScript
- [ADR-0002: Daemon Architecture](adr/0002-daemon-architecture.md) - Why
  persistent daemon with Unix sockets
- [ADR-0003: Build-time Spec Parsing](adr/0003-build-time-spec-parsing.md) - Why
  parse Fig specs at build time
- [ADR-0004: Direct Terminal Control](adr/0004-direct-terminal-control.md) - Why
  ZLE integration over Accessibility API
- [ADR-0005: Ratatui for TUI](adr/0005-ratatui-for-tui.md) - Why Ratatui over
  other TUI frameworks

## Quick Links

- [OpenSpec Changes](../openspec/changes/) - Planned features and changes
- [Roadmap](../openspec/ROADMAP.md) - Development phases and priorities
- [Project Context](../openspec/project.md) - Tech stack and conventions

## Documentation Standards

When writing documentation:

- Use clear, concise language
- Include code examples where appropriate
- Keep docs up-to-date with code changes
- Link to related documents
- Use present tense ("The daemon listens..." not "The daemon will listen...")
