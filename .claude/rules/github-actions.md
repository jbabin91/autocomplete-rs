---
paths:
  - '.github/workflows/**'
  - '.github/actions/**'
  - '.github/renovate.json'
  - 'dist-workspace.toml'
  - 'deny.toml'
---

# GitHub Actions & CI/CD

## Workflows

| Workflow       | File                 | Triggers                    | Purpose                                                       |
| -------------- | -------------------- | --------------------------- | ------------------------------------------------------------- |
| CI             | `ci.yml`             | push/PR to main             | Format, lint, test, MSRV, deny, PR title with gate job        |
| Release PLZ    | `release-plz.yml`    | push to main                | Creates release PRs, publishes to crates.io via OIDC          |
| Release        | `release.yml`        | version tags (`v*.*.*`)     | cargo-dist: builds binaries, GitHub Release, Homebrew formula |
| Security Audit | `audit.yml`          | weekly + Cargo file changes | `rustsec/audit-check` for dependency vulnerabilities          |
| CodeQL         | `codeql.yml`         | push to main + weekly       | Static analysis for Rust                                      |
| Branch Cleanup | `branch-cleanup.yml` | PR closed + daily           | Deletes unmerged PR branches and stale branches (30+ days)    |

## CI jobs

| Job       | Purpose                                                            |
| --------- | ------------------------------------------------------------------ |
| Lint      | `cargo clippy --locked --all-targets --all-features`               |
| Test      | `cargo nextest run --locked --all-features`                        |
| MSRV      | `cargo check --locked` with Rust version from `Cargo.toml`         |
| Deny      | `cargo-deny` — license compliance, advisory, bans, source policies |
| Format    | `cargo fmt` + `taplo` + `prettier` + `markdownlint-cli2`           |
| PR Title  | Conventional commit format validation (PRs only)                   |
| CI Status | Gate job aggregating all results into a summary table              |

`ci-status` is the only required check in the GitHub ruleset. Skipped jobs (e.g., on release-plz branches) pass the gate.

## Workflow hardening

- **Permissions**: `permissions: {}` at workflow level, grant per-job (least privilege)
- **Checkout**: Always use `persist-credentials: false` unless git push is needed
- **Cargo flags**: Use `--locked` on all cargo commands in CI (ensures Cargo.lock match)
- **Concurrency**: `cancel-in-progress` is PR-only (don't cancel main branch runs)
- **Release-plz skip**: All CI jobs skip on `release-plz-*` branches (only change Cargo.toml/lock/CHANGELOG)
- **Fail-fast gate**: Format job runs first (~15-30s, no compile). Lint, test, MSRV, and deny all `needs: [format]` — if formatting fails, expensive Rust compile jobs are skipped

## Composite actions

Reusable actions in `.github/actions/`:

| Action            | Purpose                                                              |
| ----------------- | -------------------------------------------------------------------- |
| `setup-rust`      | Installs stable Rust (rustfmt + clippy), cargo-nextest, rust-cache   |
| `setup-mise`      | Installs mise with taplo, prettier, markdownlint-cli2                |
| `static-analysis` | Runs `cargo fmt --check`, taplo, prettier, markdownlint (no install) |
| `run-tests`       | Runs `cargo nextest run --locked --all-features` (no install)        |

**Design pattern**: Setup actions install tools. Run actions execute checks and assume tools are already available. Jobs compose them: `checkout` → `setup-*` → `run-*`.

## cargo-deny

Config: `deny.toml` — enforces dependency policies:

- **Licenses**: Allow-list (MIT, Apache-2.0, BSD, ISC, Unicode)
- **Advisories**: Deny known vulnerabilities, warn on unmaintained
- **Bans**: Warn on duplicate dependency versions
- **Sources**: Only allow crates.io (deny unknown registries/git deps)

## Release pipeline

1. Push to main → **Release PLZ** creates/updates a release PR (version bump + changelog)
2. Merge release PR → **Release PLZ** publishes to crates.io (OIDC) and tags the version
3. Version tag → **Release** (cargo-dist) builds binaries, creates GitHub Release, publishes Homebrew formula

## CI optimizations

- **Release PRs** (`release-plz-*` branches): lint, test, and pr-title are skipped — these PRs only change `Cargo.toml`, `Cargo.lock`, and `CHANGELOG.md`
- **Security audit**: runs weekly (Mondays 6am UTC) + on Cargo file changes, not on every PR
- **CodeQL**: runs on push to main + weekly, not on PRs
- **Distribute**: only triggers on version tags, not PRs (`pr-run-mode = "skip"`)

## Secrets

| Secret                                | Used by              | Purpose                                                           |
| ------------------------------------- | -------------------- | ----------------------------------------------------------------- |
| `AUTOMATED_ACTIONS_AGENT_APP_ID`      | release-plz          | GitHub App for bot commits/PRs                                    |
| `AUTOMATED_ACTIONS_AGENT_PRIVATE_KEY` | release-plz          | GitHub App private key                                            |
| `HOMEBREW_TAP_TOKEN`                  | release (cargo-dist) | Fine-grained PAT with `contents:write` on `jbabin91/homebrew-tap` |
| `GITHUB_TOKEN`                        | all workflows        | Implicit, standard GitHub authentication                          |

## Action pinning

Actions are SHA-pinned for supply chain security. Two systems manage pins:

- **`release.yml`**: Pins defined in `dist-workspace.toml` under `[dist.github-action-commits]`, generated by `dist generate-ci`. Renovate auto-updates the YAML directly (enabled by `allow-dirty = ["ci"]`).
- **All other workflows**: Pins managed manually with `# vX` comments. Renovate auto-updates via `helpers:pinGitHubActionDigests`.

## cargo-dist management

`release.yml` is auto-generated by cargo-dist. Configuration lives in `dist-workspace.toml`. `.prettierignore` excludes `release.yml` so prettier doesn't conflict with cargo-dist's formatting.

**Regenerating** (needed for cargo-dist upgrades or config changes):

1. Remove `allow-dirty = ["ci"]` from `dist-workspace.toml`
2. Update SHAs in `[dist.github-action-commits]` to latest
3. Run `dist generate-ci`
4. Add `allow-dirty = ["ci"]` back
5. Commit all changes

**Do NOT manually edit `release.yml`** — changes will be overwritten on regeneration. Edit `dist-workspace.toml` instead.

## Renovate

Config: `.github/renovate.json`

- Non-major updates are grouped and auto-merged
- Major updates require manual review
- Uses `helpers:pinGitHubActionDigests` to keep action SHAs current
- Semantic commit messages on dependency PRs
