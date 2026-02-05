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

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:

   ```bash
   git pull --rebase
   bd sync
   git push
   git status  # MUST show "up to date with origin"
   ```

5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**

- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds
