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
- **Storage** (`src/storage/`) — local turso (SQLite-compatible) database for structured persistence:
  - `mod.rs` — public facade with `init()`, `open_readonly()`, `StorageHandle`
  - `events.rs` — `StorageEvent` enum, `Severity`, `DiagnosticCategory`
  - `schema.rs` — version-tracked migrations (v1: sessions, diagnostics, metrics)
  - `actor.rs` — background write actor with batched transactions (channel+actor pattern)
  - `queries.rs` — read queries for `diagnose` command (`DiagnoseReport`)
- **Shared utilities** (`src/paths.rs`) — `pub(crate)` helpers used across modules (e.g. `home_dir()` for `$HOME` resolution).
- **Overlay Dropdown** — Not yet implemented. Will render completions in a native overlay window positioned at the terminal cursor (like Fig.io). Uses platform-specific backends: NSPanel on macOS (via winit 0.31 `with_panel(true)`), override-redirect on X11, layer-shell on Wayland. See [ADR-0008](docs/adr/0008-native-overlay-dropdown.md) and the spike examples:
  - `examples/overlay_poc.rs` — raw objc2 NSPanel + Accessibility API positioning (macOS-only)
  - `examples/overlay_winit.rs` — winit 0.31 NSPanel via `with_panel(true)` + softbuffer rendering (cross-platform window creation, macOS NSPanel behavior)
  - `examples/overlay_tokio.rs` — winit + Tokio async runtime coexistence (winit on main thread, Tokio on background thread, cross-thread mpsc + `EventLoopProxy::wake_up()`). Validates single-process daemon+overlay architecture with sub-ms wake latency.
- **Parser** (`src/parser/`) — stub. Intended to tokenize the shell buffer and match against completion specs. Will implement `CompletionEngine` trait.
- **Shell integration** (`shell-integration/zsh.zsh`) — ZLE widget that captures the buffer/cursor, calls the client, and inserts the selected completion.
- **Socket path:** `/tmp/autocomplete-rs.sock` (override with `AUTOCOMPLETE_RS_SOCKET` env var)

## Development

- **Rust 2024 Edition** (MSRV in `Cargo.toml` `rust-version`)
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
mise run bench       # cargo bench --all-features (Criterion)
mise run ci          # fmt-check + check + lint + test
cargo nextest run -E 'test(name)'  # run a single test by name
cargo bench --bench engine         # run a single benchmark suite
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
- **Code review:** Copilot reviews all PRs (including drafts) automatically and
  re-reviews on every push. Review guidelines are in `.github/instructions/rust.instructions.md`.
  Address or reply to review comments, then resolve the threads.
- **PR body:** Use `.github/pull_request_template.md` — fill in Summary (what and why) and Resolves (bead or issue)

**Commit conventions:** [Conventional Commits](https://www.conventionalcommits.org/)

- Format: `type(scope): description` (scope optional)
- Types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`
- Breaking changes: `feat!:` or `BREAKING CHANGE:` footer
- Enforced locally by hk commit-msg hook (`cog verify`)
- Enforced in CI by PR title validation (`amannn/action-semantic-pull-request`)
- **Commit timing:** See the `commit-discipline` skill for rules. In short: don't commit
  during active back-and-forth; commit when working autonomously or when asked.

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
