# AGENTS.md

## Project Context

autocomplete-rs is a Rust-based terminal autocomplete engine — a spiritual successor to Fig.io that aims to be fast, lightweight, and free of the bloat that plagued Fig and its successors (CodeWhisperer, Amazon Q, Kiro CLI).

## Architecture

Three-component design: a Tokio-based daemon (Unix socket server), a CLI client, and a ZLE widget for zsh. The crate is both a library (`src/lib.rs`) and binary (`src/main.rs`) — the binary imports from the library, and integration tests use the library directly.

- **Protocol** (`src/protocol.rs`) — shared types at crate root: `CompletionRequest`, `CompletionResponse`, `DaemonMessage` (tagged enum with `Complete` | `Shutdown`), validation, constants. Used by both daemon and CLI client.
- **Engine** (`src/engine.rs`) — `CompletionEngine` trait at crate root. The daemon consumes it via `Arc<dyn CompletionEngine>`. `StubEngine` returns empty suggestions until the parser is wired in. Designed so the daemon-vs-single-process decision can be deferred.
- **Daemon** (`src/daemon/`) — long-running process listening on a Unix socket:
  - `mod.rs` — thin facade with `start()` and `start_with_engine()`
  - `server.rs` — accept loop with `CancellationToken` + `JoinSet` + semaphore backpressure
  - `handler.rs` — per-connection request handling with timeouts, size limits, validation
  - `state.rs` — `DaemonState` (engine, semaphore, cancel token, atomic metrics)
  - `pid.rs` — RAII `PidFile` for single-instance enforcement via `kill(pid, 0)`
- **Inline Dropdown** — Not yet implemented. Will render completions inline below the cursor using raw ANSI escape codes via crossterm (no alternate screen, no Ratatui).
- **Parser** (`src/parser/`) — stub. Intended to tokenize the shell buffer and match against completion specs. Will implement `CompletionEngine` trait.
- **Shell integration** (`shell-integration/zsh.zsh`) — ZLE widget that captures the buffer/cursor, calls the client, and inserts the selected completion.
- **Socket path:** `/tmp/autocomplete-rs.sock` (override with `AUTOCOMPLETE_RS_SOCKET` env var)

## Development

- **Rust 2024 Edition** (1.85+)
- **Task runner:** mise (see mise.toml)
- **Git hooks:** hk (see hk.pkl) — runs fmt, clippy, check on pre-commit; commit-msg validation; tests on pre-push
- **Formatting:** rustfmt (100 char width), prettier for non-Rust files, taplo for TOML, markdownlint-cli2 for markdown
- **Linting:** clippy with `-D warnings` (zero warnings policy)
- **Commit messages:** Conventional Commits enforced by cocogitto (`cog verify`)

```sh
mise run build       # debug build
mise run release     # optimized build
mise run test        # cargo nextest run --all-features
mise run lint        # clippy
mise run fmt         # format all files
mise run ci          # fmt-check + check + lint + test
cargo nextest run -E 'test(name)'  # run a single test by name
```

## Code Style

- Rust 2024 Edition idioms
- 100 char max line width
- `anyhow` for application errors, `thiserror` for library errors
- Structured logging via `tracing`
- Clap derive for CLI argument parsing
- Serde + serde_json for IPC serialization

## Git Workflow

**Branching:** GitHub Flow — `main` is the only long-lived branch.

- **Maintainer** can push directly to main
- **Contributors** must use feature branches + PRs
- **Branch naming:** `feat/`, `fix/`, `refactor/`, `chore/` prefixes (match conventional commit types)
- **Merging:** Squash merge or rebase merge only — no merge commits
- **CI:** All PRs must pass the `CI Status` gate check before merging
- **PR body:** Use `.github/pull_request_template.md` — fill in Summary (what and why) and Resolves (bead or issue)

**Commit conventions:** [Conventional Commits](https://www.conventionalcommits.org/)

- Format: `type(scope): description` (scope optional)
- Types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`
- Breaking changes: `feat!:` or `BREAKING CHANGE:` footer
- Enforced locally by hk commit-msg hook (`cog verify`)
- Enforced in CI by PR title validation (`amannn/action-semantic-pull-request`)

**Workflow formulas:** Use `bd mol pour <formula>` for structured work.

- `feature` — Design → Implement → Test → Document
- `bug` — Investigate → Fix → Verify
- `spike` — Scope → Research → Document → Follow-up
- `refactor` — Baseline → Implement → Verify

For simple tasks/chores, use `bd create` directly.

**Releases:** release-plz (versioning + changelog + crates.io via OIDC) + cargo-dist (binaries + installers + Homebrew tap). See `.claude/rules/github-actions.md` for full CI/CD documentation.

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:

   ```sh
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
