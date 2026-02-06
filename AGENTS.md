# AGENTS.md

## Project Context

autocomplete-rs is a Rust-based terminal autocomplete engine — a spiritual successor to Fig.io that aims to be fast, lightweight, and free of the bloat that plagued Fig and its successors (CodeWhisperer, Amazon Q, Kiro CLI).

## Architecture

Three-component design: a Tokio-based daemon (Unix socket server), a CLI client, and a ZLE widget for zsh.

- **Daemon** (`src/daemon/`) — long-running process listening on a Unix socket. Receives `CompletionRequest` JSON (buffer + cursor position), returns `CompletionResponse` with suggestions.
- **Inline Dropdown** — Not yet implemented. Will render completions inline below the cursor using raw ANSI escape codes via crossterm (no alternate screen, no Ratatui).
- **Parser** (`src/parser/`) — stub. Intended to tokenize the shell buffer and match against completion specs.
- **Shell integration** (`shell-integration/zsh.zsh`) — ZLE widget that captures the buffer/cursor, calls the client, and inserts the selected completion.
- **Socket path:** `/tmp/autocomplete-rs.sock` (override with `AUTOCOMPLETE_RS_SOCKET` env var)

## Development

- **Rust 2024 Edition** (1.85+)
- **Task runner:** mise (see mise.toml)
- **Git hooks:** hk (see hk.pkl) — runs fmt, clippy, check on pre-commit; commit-msg validation; tests on pre-push
- **Formatting:** rustfmt (100 char width), prettier for non-Rust files, taplo for TOML
- **Linting:** clippy with `-D warnings` (zero warnings policy)
- **Commit messages:** Conventional Commits enforced by cocogitto (`cog verify`)

```sh
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

## Git Workflow

**Branching:** GitHub Flow — `main` is the only long-lived branch.

- **Maintainer** can push directly to main
- **Contributors** must use feature branches + PRs
- **Branch naming:** `feat/`, `fix/`, `refactor/`, `chore/` prefixes (match conventional commit types)
- **Merging:** Squash merge or rebase merge only — no merge commits
- **CI:** All PRs must pass the `CI Status` gate check before merging

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

**Releases:** release-plz (versioning + changelog + crates.io via OIDC) + cargo-dist (binaries + installers + Homebrew tap).

## CI/CD

### Workflows

| Workflow       | File                 | Triggers                    | Purpose                                                       |
| -------------- | -------------------- | --------------------------- | ------------------------------------------------------------- |
| CI             | `ci.yml`             | push/PR to main             | Lint, test, PR title validation with gate job                 |
| Release PLZ    | `release-plz.yml`    | push to main                | Creates release PRs, publishes to crates.io via OIDC          |
| Release        | `release.yml`        | version tags (`v*.*.*`)     | cargo-dist: builds binaries, GitHub Release, Homebrew formula |
| Security Audit | `audit.yml`          | weekly + Cargo file changes | `rustsec/audit-check` for dependency vulnerabilities          |
| CodeQL         | `codeql.yml`         | push to main + weekly       | Static analysis for Rust                                      |
| Branch Cleanup | `branch-cleanup.yml` | PR closed + daily           | Deletes unmerged PR branches and stale branches (30+ days)    |

### CI gate job

`ci-status` is the only required check in the GitHub ruleset. It aggregates lint, test, and pr-title results into a summary table. Skipped jobs (e.g., on release-plz branches) pass the gate.

### Release pipeline

1. Push to main → **Release PLZ** creates/updates a release PR (version bump + changelog)
2. Merge release PR → **Release PLZ** publishes to crates.io (OIDC) and tags the version
3. Version tag → **Release** (cargo-dist) builds binaries, creates GitHub Release, publishes Homebrew formula

### CI optimizations

- **Release PRs** (`release-plz-*` branches): lint, test, and pr-title are skipped — these PRs only change `Cargo.toml`, `Cargo.lock`, and `CHANGELOG.md`
- **Security audit**: runs weekly (Mondays 6am UTC) + on Cargo file changes, not on every PR
- **CodeQL**: runs on push to main + weekly, not on PRs
- **Distribute**: only triggers on version tags, not PRs (`pr-run-mode = "skip"`)

### Secrets

| Secret                                | Used by              | Purpose                                                           |
| ------------------------------------- | -------------------- | ----------------------------------------------------------------- |
| `AUTOMATED_ACTIONS_AGENT_APP_ID`      | release-plz          | GitHub App for bot commits/PRs                                    |
| `AUTOMATED_ACTIONS_AGENT_PRIVATE_KEY` | release-plz          | GitHub App private key                                            |
| `HOMEBREW_TAP_TOKEN`                  | release (cargo-dist) | Fine-grained PAT with `contents:write` on `jbabin91/homebrew-tap` |
| `GITHUB_TOKEN`                        | all workflows        | Implicit, standard GitHub authentication                          |

### Action pinning

Actions are SHA-pinned for supply chain security. Two systems manage pins:

- **`release.yml`**: Pins defined in `dist-workspace.toml` under `[dist.github-action-commits]`, generated by `dist generate-ci`. Renovate auto-updates the YAML directly (enabled by `allow-dirty = ["ci"]`).
- **All other workflows**: Pins managed manually with `# vX` comments. Renovate auto-updates via `helpers:pinGitHubActionDigests`.

### cargo-dist management

`release.yml` is auto-generated by cargo-dist. Configuration lives in `dist-workspace.toml`. `.prettierignore` excludes `release.yml` so prettier doesn't conflict with cargo-dist's formatting.

**Regenerating** (needed for cargo-dist upgrades or config changes):

1. Remove `allow-dirty = ["ci"]` from `dist-workspace.toml`
2. Update SHAs in `[dist.github-action-commits]` to latest
3. Run `dist generate-ci`
4. Add `allow-dirty = ["ci"]` back
5. Commit all changes

**Do NOT manually edit `release.yml`** — changes will be overwritten on regeneration. Edit `dist-workspace.toml` instead.

### Renovate

Config: `.github/renovate.json`

- Non-major updates are grouped and auto-merged
- Major updates require manual review
- Uses `helpers:pinGitHubActionDigests` to keep action SHAs current
- Semantic commit messages on dependency PRs

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
