# autocomplete-rs

> Fast, universal terminal autocomplete that works everywhere—without the
> positioning bugs.

[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Status: Pre-Alpha](https://img.shields.io/badge/status-pre--alpha-red.svg)](https://github.com/jacebabin/autocomplete-rs)

**⚠️ Project Status: Early Development (Pre-Release)**

autocomplete-rs is currently in active development. Core features are being
implemented. Not yet ready for production use.

## Why autocomplete-rs?

Frustrated with [Amazon Q](https://aws.amazon.com/q/) (formerly Fig) and its
persistent positioning bugs, I decided to build a better terminal autocomplete
system from scratch.

**The Problem with Amazon Q:**

- Dropdown appears in the wrong location
- Breaks with multi-monitor setups
- Incorrect positioning in terminal multiplexers
- Issues with custom fonts and scaling
- Heavy resource usage (~100MB+ memory)

**The autocomplete-rs Solution:**

- ✅ **Zero positioning bugs** - Direct terminal control, no Accessibility API
- ⚡ **Blazing fast** - <20ms total latency, <5ms daemon startup
- 🪶 **Lightweight** - <50MB memory, ~8-15MB binary
- 🌍 **Universal** - Works on all terminals (iTerm2, Alacritty, Kitty, Ghostty,
  etc.)
- 🦀 **Built with Rust** - Reliable, safe, and performant

## Features

### Current (Phase 1 - MVP)

- [x] Persistent daemon with Unix socket IPC
- [x] ZLE integration for zsh
- [x] Terminal UI with Ratatui
- [ ] Basic parser (in progress)
- [ ] Partial spec support (in progress)

### Planned

**Phase 2 (Scale):**

- [ ] Full Fig spec parsing (600+ CLI tools)
- [ ] MessagePack spec embedding
- [ ] LRU spec caching

**Phase 3 (Polish):**

- [ ] Catppuccin theme support (Mocha, Macchiato, Frappe, Latte)
- [ ] Configuration file support
- [ ] Theme customization

**Phase 4 (Universal):**

- [ ] Bash support
- [ ] Fish support
- [ ] Windows WSL support

See project issues (`bd list`) for detailed development plan.

## Quick Start

### Installation

**From source (current method):**

```bash
# Clone repository
git clone https://github.com/jacebabin/autocomplete-rs.git
cd autocomplete-rs

# Build release binary
cargo build --release

# Install binary
cp target/release/autocomplete-rs ~/.cargo/bin/

# Install shell integration (zsh only for now)
autocomplete-rs install zsh

# Restart shell
exec zsh
```

**Usage:**

Type a command and press **Alt+Space** to trigger completions.

```bash
git checkout <Alt+Space>
# → Shows branch suggestions

npm run <Alt+Space>
# → Shows package.json scripts (Phase 2)
```

### Requirements

- **Rust:** 1.85+ (for Rust 2024 Edition)
- **OS:** macOS or Linux (Windows WSL coming in Phase 4)
- **Shell:** Zsh 5.8+ (Bash and Fish coming in Phase 4)
- **Terminal:** Any modern terminal (iTerm2, Alacritty, Kitty, WezTerm, Ghostty,
  etc.)

## How It Works

```text
User types "git checkout " + Alt+Space
         ↓
┌─────────────────────────┐
│  ZLE Widget (zsh)       │  Captures buffer & cursor
└─────────────────────────┘
         ↓ Unix Socket (JSON)
┌─────────────────────────┐
│  Persistent Daemon      │  <10ms response time
│  ├─ Parser              │  Analyzes command context
│  ├─ Spec Matcher        │  Finds relevant completions
│  └─ Response            │  Returns suggestions
└─────────────────────────┘
         ↓ JSON Response
┌─────────────────────────┐
│  Terminal UI (Ratatui)  │  Renders dropdown
│  ├─ Dropdown below cmd  │  Native terminal rendering
│  ├─ Keyboard navigation │  Arrow keys, Enter, Esc
│  └─ Theme support       │  Catppuccin colors (Phase 3)
└─────────────────────────┘
```

**Key Technical Decisions:**

- **Daemon Architecture** ([ADR-0002](docs/adr/0002-daemon-architecture.md)) -
  Persistent process for zero startup cost
- **Direct Terminal Control**
  ([ADR-0004](docs/adr/0004-direct-terminal-control.md)) - No Accessibility API,
  no positioning bugs
- **Build-time Spec Parsing**
  ([ADR-0003](docs/adr/0003-build-time-spec-parsing.md)) - Embed specs for
  instant availability
- **Ratatui for TUI** ([ADR-0005](docs/adr/0005-ratatui-for-tui.md)) - Rich
  terminal UI framework

## Documentation

### For Users

- [Installation Guide](docs/user-guide/installation.md) - How to install and set
  up
- [Configuration Guide](docs/user-guide/configuration.md) - Customize themes and
  behavior
- [Troubleshooting](docs/user-guide/troubleshooting.md) - Common issues and
  solutions

### For Developers

- [Getting Started](docs/development/getting-started.md) - Development
  environment setup
- [Project Structure](docs/development/project-structure.md) - Codebase
  organization
- [Testing Guide](docs/development/testing.md) - Testing practices
- [Contributing Guide](docs/development/contributing.md) - How to contribute

### Architecture

- [Architecture Overview](docs/architecture/overview.md) - High-level system
  design
- [Daemon Architecture](docs/architecture/daemon.md) - Unix socket server design
- [Parser Architecture](docs/architecture/parser.md) - Command parsing
  algorithms
- [TUI Architecture](docs/architecture/tui.md) - Terminal UI rendering

### Architecture Decision Records (ADRs)

- [ADR-0001: Use Rust](docs/adr/0001-use-rust.md) - Why Rust over
  TypeScript/Bun/Go
- [ADR-0002: Daemon Architecture](docs/adr/0002-daemon-architecture.md) - Why
  persistent daemon
- [ADR-0003: Build-time Spec Parsing](docs/adr/0003-build-time-spec-parsing.md) -
  Why parse at build time
- [ADR-0004: Direct Terminal Control](docs/adr/0004-direct-terminal-control.md) -
  Why ZLE over Accessibility API
- [ADR-0005: Ratatui for TUI](docs/adr/0005-ratatui-for-tui.md) - Why Ratatui
  for rendering

## Performance

**Design Goals:**

- **Total latency:** <20ms (trigger to display)
- **Daemon startup:** <5ms
- **IPC round-trip:** <1ms
- **Parser:** <5ms
- **TUI render:** <10ms
- **Memory:** <50MB with all specs loaded

**Benchmarking:**

```bash
# Run benchmarks
cargo bench

# Profile with flamegraph
cargo install flamegraph
cargo flamegraph --bin autocomplete-rs -- daemon /tmp/test.sock
```

## Project Structure

```sh
autocomplete-rs/
├── src/
│   ├── main.rs          # CLI entry point
│   ├── daemon/          # Unix socket server
│   ├── parser/          # Command buffer parsing
│   ├── tui/             # Ratatui UI rendering
│   └── specs/           # Completion specs (Phase 2)
├── shell-integration/
│   └── zsh.zsh          # ZLE widget
├── docs/                # Comprehensive documentation
├── .beads/              # Issue tracking (beads)
└── tests/               # Integration tests
```

## Contributing

Contributions welcome! This project is in early development and there's lots to
do.

**Getting Started:**

1. Read [Getting Started Guide](docs/development/getting-started.md)
2. Check project issues (`bd ready`) for current priorities
3. Look for issues tagged
   [`good-first-issue`](https://github.com/jacebabin/autocomplete-rs/labels/good-first-issue)
4. Read [Contributing Guide](docs/development/contributing.md)
5. Submit your PR!

**Development Priorities (Phase 1):**

- [ ] Complete parser implementation
- [ ] Basic spec matching
- [ ] Integration tests
- [ ] Performance benchmarks
- [ ] Documentation improvements

## Inspiration & Related Projects

- **[Fig/Amazon Q](https://aws.amazon.com/q/)** - Original inspiration (and
  frustration)
- **[Fig Autocomplete Specs](https://github.com/withfig/autocomplete)** - 600+
  completion specs we'll reuse
- **[Inshellisense](https://github.com/microsoft/inshellisense)** - Microsoft's
  Node.js autocomplete
- **[zsh-autosuggestions](https://github.com/zsh-users/zsh-autosuggestions)** -
  Simple inline suggestions
- **[Carapace](https://github.com/rsteube/carapace)** - Go-based completion
  engine

## Tech Stack

- **Language:** [Rust](https://www.rust-lang.org/) 2024 Edition
- **Dev Tools:** [mise](https://mise.jdx.dev/) (tool & task manager)
- **Git Hooks:** [hk](https://hk.jdx.dev/) (with Rust builtins)
- **Async Runtime:** [Tokio](https://tokio.rs/) 1.48
- **TUI:** [Ratatui](https://ratatui.rs/) 0.29
- **Terminal:** [Crossterm](https://github.com/crossterm-rs/crossterm) 0.29
- **CLI:** [Clap](https://github.com/clap-rs/clap) 4.5
- **Serialization:** [MessagePack](https://msgpack.org/) via rmp-serde

## License

MIT License - see [LICENSE](LICENSE) file for details.

Copyright © 2025 [Jace Babin](https://github.com/jacebabin)

## Support

- **Issues:**
  [GitHub Issues](https://github.com/jacebabin/autocomplete-rs/issues)
- **Discussions:**
  [GitHub Discussions](https://github.com/jacebabin/autocomplete-rs/discussions)
- **Documentation:** [docs/](docs/)

## Acknowledgments

- **Fig team** for the excellent
  [autocomplete specs](https://github.com/withfig/autocomplete)
- **Ratatui community** for the amazing TUI framework
- **Rust community** for the language and ecosystem

---

**Built with 🦀 Rust** | **Powered by ⚡ Performance** | **Designed for 🌍
Everyone**
