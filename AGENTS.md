# AGENTS.md

## Project Context

autocomplete-rs is a Rust-based terminal autocomplete engine — a spiritual successor to Fig.io that aims to be fast, lightweight, and free of the bloat that plagued Fig and its successors (CodeWhisperer, Amazon Q, Kiro CLI).

## Architecture

Three-component design: a Tokio-based daemon (Unix socket server), a CLI client, and a ZLE widget for zsh. The crate is both a library (`src/lib.rs`) and binary (`src/main.rs`) — the binary imports from the library, and integration tests use the library directly.

- **Protocol** (`src/protocol.rs`) — shared types at crate root: `CompletionRequest`, `CompletionResponse`, `DaemonMessage` (tagged enum with `Complete` | `Shutdown`), validation, constants. Used by both daemon and CLI client.
- **Engine** (`src/engine.rs`) — `CompletionEngine` trait at crate root. The daemon consumes it via `Arc<dyn CompletionEngine>`. `StubEngine` returns empty suggestions until the parser is wired in. Designed so the daemon-vs-single-process decision can be deferred.
- **Daemon** (`src/daemon/`) — long-running process listening on a Unix socket:
  - `mod.rs` — facade with `start()`, `start_with_engine()`, `start_with_overlay()`, and shared `run_daemon()`. `start_with_overlay()` runs winit on the main thread with Tokio on a background thread.
  - `server.rs` — accept loop with `CancellationToken` + `JoinSet` + semaphore backpressure
  - `handler.rs` — per-connection request handling with timeouts, size limits, validation. Forwards completions and shutdown signals to the overlay channel when present.
  - `state.rs` — `DaemonState` (engine, semaphore, cancel token, atomic metrics, `Option<OverlayChannel>`)
  - `pid.rs` — RAII `PidFile` for single-instance enforcement via `kill(pid, 0)`
- **Storage** (`src/storage/`) — local turso (SQLite-compatible) database for structured persistence:
  - `mod.rs` — public facade with `init()`, `open_readonly()`, `StorageHandle`
  - `events.rs` — `StorageEvent` enum, `Severity`, `DiagnosticCategory`
  - `schema.rs` — version-tracked migrations (v1: sessions, diagnostics, metrics)
  - `actor.rs` — background write actor with batched transactions (channel+actor pattern)
  - `queries.rs` — read queries for `diagnose` command (`DiagnoseReport`)
- **Shared utilities** (`src/paths.rs`) — path resolution and private-directory handling shared across modules: `home_dir()` (errors when `$HOME` is unset — never falls back to `/tmp`), `data_dir()`, `default_socket_path()`, and `ensure_private_dir()` with its `Repair::Yes`/`Repair::No` policy.
- **Overlay Dropdown** (`src/overlay/`) — native overlay window positioned at the terminal cursor (like Fig.io). macOS MVP implemented; Linux/Windows are follow-up. See [ADR-0008](docs/adr/0008-native-overlay-dropdown.md).
  - `mod.rs` — module facade, `OverlayMessage` enum (tagged for cross-thread IPC)
  - `app.rs` — winit `ApplicationHandler` (`OverlayApp`): window creation, message dispatch, keyboard navigation, rendering. Window starts hidden, shown when non-empty suggestions arrive.
  - `renderer.rs` — pixel-buffer rendering of the dropdown (ARGB for softbuffer). Constants: colors, sizing, max visible items.
  - `font.rs` — bitmap 5×7 glyph data scaled 3× for HiDPI, `draw_char`/`draw_text`
  - `positioning.rs` — pure coordinate math: cursor position from terminal geometry, overlay placement with edge detection + flip-above
  - `backend.rs` — `OverlayBackend` trait, `OverlayPosition`, `PositioningError`
  - `macos.rs` — macOS backend: Accessibility API window bounds + TIOCGWINSZ + coordinate conversion
  - Spike examples removed (superseded); see git history for `examples/overlay_*.rs`
- **Parser** (`src/parser/`) — FSM tokenizer and context analyzer implementing `CompletionEngine`:
  - `mod.rs` — public facade re-exporting `ParserEngine`, `CompletionContext`, `Token`, `TokenKind`, `TokenizeResult`
  - `tokenizer.rs` — single-pass FSM tokenizer: handles whitespace splitting, single/double quotes, backslash escaping, multi-char operators (`|`, `||`, `&&`, `;`, `&`, `|&`, `<`, `>`, `>>`), unclosed quotes, cursor tracking with char-boundary clamping
  - `context.rs` — `CompletionContext` enum (`Command`, `Subcommand`, `Option`, `Argument`, `Filename`) and `analyze_context()` which walks the tokenized buffer to classify what the user is completing
  - `engine.rs` — `ParserEngine` (stateless, `Send + Sync`) wired into the daemon. MVP returns empty suggestions; spec-based generation is the next phase
- **Shell integration** (`shell-integration/zsh.zsh`) — ZLE widget that captures the buffer/cursor, calls the client, and inserts the selected completion.
- **Socket path:** `~/.autocomplete-rs/daemon.sock` (override with `AUTOCOMPLETE_RS_SOCKET` env var)

## Development

- **Rust 2024 Edition.** `.mise.toml` pins the toolchain CI builds with; `Cargo.toml`'s
  `rust-version` is the lower MSRV the crate promises, verified by `mise run msrv`.
- **Task runner:** mise (see `.mise.toml`). Every tool is pinned exact — a floating
  version can red CI on code nobody touched.
- **Git hooks:** lefthook (see `lefthook.yml`). `mise run setup` installs them.
  `core.hooksPath` must stay unset, or git bypasses lefthook silently.
- **Formatting:** one writer per file type — rustfmt owns `.rs` (100 char width),
  rumdl owns `.md`, dprint owns TOML/JSON/YAML. Two formatters on one file oscillate.
- **Linting:** clippy with `-D warnings` (zero warnings policy); actionlint for workflows
- **Secrets:** gitleaks on pre-commit, with a full-history scan in CI as a backstop
- **Commit messages:** Conventional Commits enforced by cocogitto (`cog verify`)

CI runs these same tasks rather than restating their commands, so the two cannot drift.

```sh
mise run check       # fmt + clippy + dprint + rumdl + actionlint  (CI: Static Analysis)
mise run test        # nextest + doctests                          (CI: Tests)
mise run msrv        # build against Cargo.toml's rust-version     (CI: MSRV)
mise run audit       # cargo-deny advisories/licenses/sources      (CI: Audit)
mise run scan-secrets # gitleaks over full history                 (CI: Secret Scan)
mise run ci          # all of the above
mise run fmt         # format everything
mise run build       # debug build
mise run release     # optimized build
mise run bench       # cargo bench --all-features (Criterion)
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

## Conventions

Detailed conventions live in `docs/conventions/`. Nothing loads them automatically —
read the one covering the files you are about to touch, **before** writing, not after.

| File | Read before touching |
| ---- | -------------------- |
| [`rust.md`](docs/conventions/rust.md) | any `.rs` file — error handling, `unwrap()`/`expect()` scope, async, RAII cleanup, filesystem, logging |
| [`daemon.md`](docs/conventions/daemon.md) | `src/daemon/`, `src/protocol.rs`, `src/engine.rs` — socket lifecycle, backpressure, PID file, shutdown |
| [`storage.md`](docs/conventions/storage.md) | `src/storage/` — turso, the write actor, migrations |
| [`dropdown.md`](docs/conventions/dropdown.md) | `src/overlay/`, `docs/design/overlay.md` — winit, rendering, panic safety in `ApplicationHandler` |
| [`shell-integration.md`](docs/conventions/shell-integration.md) | `shell-integration/` — ZLE widget contract |
| [`tooling-and-formatting.md`](docs/conventions/tooling-and-formatting.md) | `Cargo.toml`, `build.rs`, `.mise.toml`, `lefthook.yml`, `dprint.json`, `deny.toml`, `rustfmt.toml`, `clippy.toml`, `.rumdl.toml`, `.gitleaks.toml`, `.editorconfig` |
| [`github-actions.md`](docs/conventions/github-actions.md) | `.github/workflows/`, `.github/actions/`, `.github/renovate.json`, `dist-workspace.toml`, `deny.toml` — hardening, pinning, the CI gate |
| [`documentation.md`](docs/conventions/documentation.md) | any `.md` — command references, snippet fidelity, tense |

This column is the complete trigger list, transcribed from the `paths:` frontmatter
these files carried when the harness matched them automatically. Keep it that way: a
path missing here is a path with no guidance.

The two that bite hardest and apply almost everywhere: `unwrap()`/`expect()` belong
only in tests and known-safe const contexts, and sockets and temp directories are
cleaned up via RAII, never by a call on the happy path.

## Git Workflow

**Branching:** GitHub Flow — `main` is the only long-lived branch.

- **Maintainer** can push directly to main
- **Contributors** must use feature branches + PRs
- **Branch naming:** `feat/`, `fix/`, `refactor/`, `chore/` prefixes (match conventional commit types)
- **Merging:** Squash merge or rebase merge only — no merge commits
- **CI:** All PRs must pass the `CI Status` gate check before merging
- **Code review:** no reviewer bot comments on PRs — nothing catches a design or
  correctness mistake after you push. CI still gates the mechanical checks (clippy
  `-D warnings`, cargo-deny, gitleaks) and CodeQL runs post-merge on `main`.
  Substantive review happens locally before the commit, via the `review-cycle` skill,
  against the conventions in `docs/conventions/` (see below). Address or reply to any
  human review comments, then resolve the threads.
- **PR body:** Use `.github/pull_request_template.md` — fill in Summary (what and why) and Resolves (bead or issue)

**Commit conventions:** [Conventional Commits](https://www.conventionalcommits.org/)

- Format: `type(scope): description` (scope optional)
- Types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`
- Breaking changes: `feat!:` or `BREAKING CHANGE:` footer
- Enforced locally by the lefthook commit-msg hook (`cog verify`)
- Enforced in CI by PR title validation (`amannn/action-semantic-pull-request`)
- **Commit timing:** See the `commit-discipline` skill for rules. In short: don't commit
  during active back-and-forth; commit when working autonomously or when asked.

**Workflow formulas:** Use `bd mol pour <formula>` for structured work.

- `feature` — Design → Implement → Test → Document
- `bug` — Investigate → Fix → Verify
- `spike` — Scope → Research → Document → Follow-up
- `refactor` — Baseline → Implement → Verify

For simple tasks/chores, use `bd create` directly.

**Releases:** release-plz (versioning + changelog + crates.io via OIDC) + cargo-dist (binaries + installers + Homebrew tap). See `docs/conventions/github-actions.md` for full CI/CD documentation.

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:

   ```sh
   git pull --rebase
   bd dolt push   # issues live in a Dolt remote, not in this repo
   git push
   git status  # MUST show "up to date with origin"
   ```

   `bd sync` no longer exists (removed in bd 1.2.2). The issue store is a Dolt
   remote configured per-machine in `.beads/.env`; only the audit trail in
   `.beads/interactions.jsonl` travels through git.
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**

- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds
