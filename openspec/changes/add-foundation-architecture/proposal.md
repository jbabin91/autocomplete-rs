# Add Foundation Architecture

**Priority:** 1A **Phase:** 1 (MVP) **Dependencies:** None **Blocks:**
`implement-mvp-parser`, all other features

## Why

We need to establish the core architectural components that enable
autocomplete-rs to function: a persistent daemon for fast response times, a
beautiful TUI for displaying suggestions, and shell integration for capturing
user input. This foundation will support all future features.

## What Changes

- Implement persistent daemon with Unix socket IPC
- Build Ratatui-based TUI with dropdown menu and keyboard navigation
- Create zsh integration via ZLE widgets
- Establish communication protocol between shell and daemon
- Define message format for requests and responses

## Impact

- Affected specs:
  - `daemon` (new capability)
  - `tui` (new capability)
  - `shell-integration-zsh` (new capability)
- Affected code:
  - `src/daemon/mod.rs` - Unix socket server, connection handling
  - `src/tui/mod.rs` - Ratatui dropdown UI, keyboard navigation
  - `shell-integration/zsh.zsh` - ZLE widget, keybindings
  - `src/main.rs` - daemon command implementation
- Dependencies: Already in Cargo.toml (tokio, ratatui, crossterm)
- Migration: N/A (net new functionality)

## Design Decisions

- **Unix sockets over HTTP**: Lower latency, simpler protocol, no network
  overhead
- **Persistent daemon**: Avoids startup cost on every completion request
- **Direct terminal control**: No Accessibility API hacks (fixes Amazon Q
  positioning bugs)
- **JSON protocol**: Simple, debuggable, extensible
