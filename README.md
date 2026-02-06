# autocomplete-rs

> Fast, universal terminal autocomplete that works everywhere — without the
> positioning bugs.

[![CI](https://github.com/jbabin91/autocomplete-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/jbabin91/autocomplete-rs/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Status: Pre-Alpha](https://img.shields.io/badge/status-pre--alpha-red.svg)](https://github.com/jbabin91/autocomplete-rs)

**Project Status: Early Development (Pre-Release)**

autocomplete-rs is currently in active development. Core features are being
implemented. Not yet ready for production use.

## Why autocomplete-rs?

Frustrated with [Kiro CLI](https://kiro.dev/cli/) (formerly Fig, then Amazon Q)
and its persistent positioning bugs, I decided to build a better terminal
autocomplete system from scratch.

**The Problem with Kiro CLI (and its predecessors):**

- Dropdown appears in the wrong location
- Breaks with multi-monitor setups
- Incorrect positioning in terminal multiplexers
- Issues with custom fonts and scaling
- Heavy resource usage (~100MB+ memory)

**The autocomplete-rs Solution:**

- **Zero positioning bugs** — Direct terminal control, no Accessibility API
- **Blazing fast** — <20ms total latency target
- **Lightweight** — <50MB memory, ~8-15MB binary
- **Universal** — Works on all terminals (iTerm2, Alacritty, Kitty, Ghostty,
  etc.)
- **Built with Rust** — Reliable, safe, and performant

## Roadmap

### Phase 1 — MVP (current)

- [ ] Daemon with Unix socket IPC
- [ ] Command buffer parser with tokenizer
- [ ] Inline ANSI completion dropdown
- [ ] Hardcoded git completion spec
- [ ] Zsh ZLE integration

### Phase 2 — Scale

- [ ] Full Fig spec parsing (600+ CLI tools)
- [ ] MessagePack spec embedding
- [ ] LRU spec caching

### Phase 3 — Polish

- [ ] Catppuccin theme support
- [ ] Configuration file system
- [ ] Theme customization

### Phase 4 — Universal

- [ ] Bash support
- [ ] Fish support
- [ ] WSL support

See project issues (`bd list`) for detailed development plan.

## Quick Start

### From Source (current method)

```bash
git clone https://github.com/jbabin91/autocomplete-rs.git
cd autocomplete-rs
cargo build --release
cp target/release/autocomplete-rs ~/.cargo/bin/
```

### Requirements

- **Rust:** 1.85+ (Rust 2024 Edition)
- **OS:** macOS or Linux
- **Shell:** Zsh 5.8+ (Bash and Fish planned for Phase 4)
- **Terminal:** Any modern terminal emulator

## How It Works

```text
User types "git checkout " + Alt+Space
         |
+-------------------------+
|  ZLE Widget (zsh)       |  Captures buffer & cursor
+-------------------------+
         | Unix Socket (JSON)
+-------------------------+
|  Persistent Daemon      |  <10ms response time
|  +- Parser              |  Analyzes command context
|  +- Spec Matcher        |  Finds relevant completions
|  +- Response            |  Returns suggestions
+-------------------------+
         | JSON Response
+-------------------------+
|  Inline Dropdown        |  Renders below cursor
|  +- ANSI escape codes   |  No alternate screen
|  +- Keyboard navigation |  Arrow keys, Enter, Esc
+-------------------------+
```

**Key Technical Decisions:**

- [Daemon Architecture](docs/adr/0002-daemon-architecture.md) — Persistent
  process for zero startup cost
- [Direct Terminal Control](docs/adr/0004-direct-terminal-control.md) — No
  Accessibility API, no positioning bugs
- [Build-time Spec Parsing](docs/adr/0003-build-time-spec-parsing.md) — Embed
  specs for instant availability

## Development

```bash
# Install tools
mise install

# Common tasks
mise run build       # debug build
mise run release     # optimized build
mise run test        # cargo test --all-features
mise run lint        # clippy
mise run fmt         # format all files
mise run ci          # fmt-check + check + lint + test
```

**Git workflow:** GitHub Flow with conventional commits. Squash or rebase merges
only.

**CI:** GitHub Actions runs lint, test, and conventional commit checks on every
push and PR.

See [AGENTS.md](AGENTS.md) for full development guidelines.

## Project Structure

```sh
autocomplete-rs/
+-- src/
|   +-- main.rs          # CLI entry point (Clap)
|   +-- daemon/          # Tokio Unix socket server
|   +-- parser/          # Command buffer parsing (stub)
|   +-- (dropdown/)      # Inline ANSI dropdown (not yet implemented)
+-- shell-integration/
|   +-- zsh.zsh          # ZLE widget
+-- docs/                # Architecture docs, ADRs, guides
+-- .beads/              # Issue tracking
+-- .github/             # CI workflows, templates, actions
```

## Contributing

Contributions welcome! This project is in early development and there's lots to
do.

1. Read [Contributing Guide](docs/CONTRIBUTING.md)
2. Check project issues (`bd ready`) for current priorities
3. Fork, branch (`feat/`, `fix/`, `refactor/`, `chore/`), and submit a PR
4. Use [conventional commits](https://www.conventionalcommits.org/)

## Inspiration & Related Projects

- [Kiro CLI](https://kiro.dev/cli/) (formerly Fig/Amazon Q) — Original
  inspiration (and frustration)
- [Fig Autocomplete Specs](https://github.com/withfig/autocomplete) — 600+
  completion specs we'll reuse
- [Inshellisense](https://github.com/microsoft/inshellisense) — Microsoft's
  Node.js autocomplete
- [Carapace](https://github.com/rsteube/carapace) — Go-based completion engine

## Tech Stack

- **Language:** [Rust](https://www.rust-lang.org/) 2024 Edition
- **Dev Tools:** [mise](https://mise.jdx.dev/) (tool & task manager)
- **Git Hooks:** [hk](https://hk.jdx.dev/) (with Rust builtins)
- **Async Runtime:** [Tokio](https://tokio.rs/)
- **Terminal I/O:** [Crossterm](https://github.com/crossterm-rs/crossterm) (planned — inline ANSI dropdown)
- **CLI:** [Clap](https://github.com/clap-rs/clap) (derive)

## License

MIT License — see [LICENSE](LICENSE) file for details.
