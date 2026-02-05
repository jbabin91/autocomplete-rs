<!-- OPENSPEC:START -->

## OpenSpec Instructions

Always open `@/openspec/AGENTS.md` when the request:

- Mentions planning or proposals (words like proposal, spec, change, plan)
- Introduces new capabilities, breaking changes, architecture shifts, or big
  performance/security work
- Sounds ambiguous and you need the authoritative spec before coding

Use `@/openspec/AGENTS.md` to learn:

- How to create and apply change proposals
- Spec format and conventions
- Project structure and guidelines

Keep this managed block so 'openspec update' can refresh the instructions.

<!-- OPENSPEC:END -->

# AGENTS.md

## Project Context

autocomplete-rs is a Rust-based terminal autocomplete engine — a spiritual successor to Fig.io that aims to be fast, lightweight, and free of the bloat that plagued Fig and its successors (CodeWhisperer, Amazon Q, Kiro CLI).

## Architecture

Three-component design: a Tokio-based daemon (Unix socket server), a CLI client, and a ZLE widget for zsh.

- **Daemon** (`src/daemon/`) — long-running process listening on a Unix socket. Receives `CompletionRequest` JSON (buffer + cursor position), returns `CompletionResponse` with suggestions.
- **TUI** (`src/tui/`) — Ratatui-based dropdown shown by the client after receiving suggestions. Currently uses alternate screen (full-screen) — should be replaced with inline ANSI rendering.
- **Parser** (`src/parser/`) — stub. Intended to tokenize the shell buffer and match against completion specs.
- **Shell integration** (`shell-integration/zsh.zsh`) — ZLE widget that captures the buffer/cursor, calls the client, and inserts the selected completion.
- **Socket path:** `/tmp/autocomplete-rs.sock` (override with `AUTOCOMPLETE_RS_SOCKET` env var)

## Development

- **Rust 2024 Edition** (1.85+)
- **Task runner:** mise (see mise.toml)
- **Git hooks:** hk (see hk.pkl) — runs fmt, clippy, check on pre-commit; tests on pre-push
- **Formatting:** rustfmt (100 char width), prettier for non-Rust files, taplo for TOML
- **Linting:** clippy with `-D warnings` (zero warnings policy)

```bash
mise run build       # debug build
mise run release     # optimized build
mise run test        # cargo test --all-features
mise run lint        # clippy
mise run fmt         # format all files
mise run ci          # fmt-check + check + lint + test
cargo test <name>    # run a single test by name
```

## Code Style

- Rust 2024 Edition idioms
- 100 char max line width
- `anyhow` for application errors, `thiserror` for library errors
- Structured logging via `tracing`
- Clap derive for CLI argument parsing
- Serde + serde_json for IPC serialization
