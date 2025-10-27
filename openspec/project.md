# Project Context

## Purpose

autocomplete-rs is a fast, universal terminal autocomplete system that provides
IDE-style dropdown completions for command-line tools. It aims to replace
Fig/Amazon Q with a native Rust implementation that:

- Works across all terminals (iTerm2, Alacritty, Kitty, Wezterm, Ghostty, VSCode
  terminal, etc.)
- Supports multiple shells (zsh, bash, fish)
- Uses direct ZLE integration instead of fragile Accessibility API hacks
- Provides sub-millisecond response times with zero runtime overhead
- Reuses Fig's 600+ open-source completion specs

## Tech Stack

- **Language**: Rust 2024 Edition
- **TUI Framework**: Ratatui 0.29 + Crossterm 0.29
- **Async Runtime**: Tokio 1.48
- **Serialization**: Serde 1.0, MessagePack (rmp-serde)
- **CLI Framework**: Clap 4.5
- **Logging**: Tracing + Tracing-subscriber
- **Error Handling**: Anyhow + Thiserror 2.0
- **Build Tools**: Cargo, deno_ast (for parsing TypeScript specs)

## Project Conventions

### Code Style

- Use `rustfmt` defaults (Rust 2024 edition)
- Prefer explicit error types with `thiserror` for library code
- Use `anyhow` for application-level errors
- Follow Rust naming conventions:
  - `snake_case` for functions, variables, modules
  - `PascalCase` for types, traits, enums
  - `SCREAMING_SNAKE_CASE` for constants
- Prefix unused variables with underscore (`_stream`)
- Maximum line length: 100 characters
- Use meaningful variable names - avoid single letters except for iterators

### Architecture Patterns

- **Modular structure**: Separate concerns into daemon, parser, TUI, specs
  modules
- **Client-Server**: Daemon runs persistently, shell widgets communicate via
  Unix sockets
- **Async-first**: Use Tokio for all I/O operations
- **Message passing**: JSON over Unix domain sockets for shell communication
- **Build-time parsing**: Parse Fig TypeScript specs at compile time, embed as
  MessagePack
- **Direct terminal control**: Use Ratatui for TUI rendering, no web
  technologies

### Testing Strategy

- Unit tests for parser logic
- Integration tests for daemon communication
- Manual testing with real shell environments
- Test coverage for all public APIs
- Use `cargo test` for running tests

### Git Workflow

- **Main branch**: `main` - always deployable
- **Feature branches**: `feature/description` or use change-id from OpenSpec
- **Commit format**: Conventional commits
  - `feat:` new features
  - `fix:` bug fixes
  - `refactor:` code refactoring
  - `docs:` documentation
  - `test:` tests
  - Include `🤖 Generated with Claude Code` footer when applicable

## Domain Context

### Terminal Autocomplete Concepts

- **Completion Spec**: Declarative schema defining commands, subcommands,
  options, and arguments
- **ZLE (Zsh Line Editor)**: Zsh's built-in line editing system with widget
  hooks
- **Unix Socket**: IPC mechanism for daemon-shell communication
- **TUI (Terminal User Interface)**: Text-based UI rendered in terminal
- **Cursor Position**: Critical for accurate dropdown placement
- **Command Buffer**: Current shell input being completed

### Architecture Approach

- **Phase 1 (MVP)**: Basic daemon, parser, TUI with hardcoded git spec
- **Phase 2**: Build-time TypeScript parser for Fig specs
- **Phase 3**: Polish, theming (Catppuccin support), performance optimization
- **Phase 4**: Multi-shell support (bash, fish, PowerShell)

## Important Constraints

### Technical Constraints

- **Must be fast**: Sub-10ms latency for completion generation
- **Single binary**: All dependencies statically linked
- **No runtime**: Pure Rust, no Node.js/JavaScript at runtime
- **Memory efficient**: <50MB resident memory for daemon
- **Cross-platform**: Works on macOS, Linux (Windows future)
- **No Accessibility API**: Direct shell integration only

### Performance Requirements

- Daemon startup: <5ms
- Completion generation: <5ms
- TUI rendering: <10ms
- Total response time: <20ms (imperceptible to humans)

### Compatibility

- Rust 1.85+ (Edition 2024 features)
- Must work with all major terminal emulators
- Shell-agnostic architecture (start with zsh, expand to others)

## External Dependencies

### Fig Autocomplete Specs

- Repository: https://github.com/withfig/autocomplete
- 600+ completion specs in TypeScript
- Open source, community maintained
- Parse at build time using `deno_ast`

### Terminal Standards

- VT100/ANSI escape codes
- Unix domain sockets (IPC)
- PTY (pseudo-terminal) behavior

### Shell Integration Points

- **zsh**: ZLE widgets, hooks
- **bash**: readline (future)
- **fish**: native completion system (future)
