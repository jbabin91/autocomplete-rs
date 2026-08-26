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
| CI             | `ci.yml`             | push/PR to main, manual     | Static analysis, tests, MSRV, secret scan, audit, PR title, behind a gate job |
| Release PLZ    | `release-plz.yml`    | push to main                | Creates release PRs, publishes to crates.io via OIDC          |
| Release        | `release.yml`        | version tags (`v*.*.*`)     | cargo-dist: builds binaries, GitHub Release, Homebrew formula |
| Security Audit | `audit.yml`          | called by CI, weekly, manual | cargo-deny: advisories, licenses, bans, sources              |
| CodeQL         | `codeql.yml`         | push to main + weekly       | Static analysis for Rust                                      |
| Branch Cleanup | `branch-cleanup.yml` | PR closed + daily           | Deletes unmerged PR branches and stale branches (30+ days)    |

## CI jobs

| Job             | Purpose                                             | Source of truth                   |
| --------------- | --------------------------------------------------- | --------------------------------- |
| Static Analysis | rustfmt, clippy, dprint, rumdl, actionlint, dist pin + version parity, plus `cog check` on PRs | `.mise.toml` `check`; the cog step lives in `ci.yml` |
| Tests           | nextest plus doctests                               | `.mise.toml` `test`               |
| MSRV            | Build against `Cargo.toml`'s `rust-version`         | `.mise.toml` `msrv`               |
| Secret Scan     | gitleaks over full history                          | `.mise.toml` `scan-secrets`       |
| Audit           | License, advisory, bans, source policy              | `.mise.toml` `audit`, `deny.toml` |
| PR Title        | Conventional commit format validation (PRs only)    | `ci.yml` pr-title job             |
| CI Status       | Gate job aggregating all results into summary table | `ci.yml` ci-summary job           |

For exact command flags, read the CI files directly — do not duplicate them here.

`CI Status` is the only required check in the GitHub ruleset. It iterates `toJSON(needs)` rather than a hardcoded job list, so a job added to `needs` is gated without editing the step, and it fails if the row count does not match — a truncated read cannot pass silently. A skipped job fails the gate unless the step's `SKIPPABLE` allowlist names it, which today is `pr-title` alone; see the no-chaining rule below for why that allowance is safe.

## Workflow hardening

- **Permissions**: least privilege — the narrowest workflow-level scope the jobs need (`{}` where every job grants its own), widened per job
- **Checkout**: Always use `persist-credentials: false` unless git push is needed
- **Cargo flags**: Use `--locked` on all cargo commands in CI (ensures Cargo.lock match)
- **Concurrency**: `cancel-in-progress` is PR-only (don't cancel main branch runs)
- **No branch-name gating**: no job may skip on `head_ref`, which whoever opens the PR chooses — skipping on it hands anyone a green check
- **No job chaining**: jobs must not `needs` one another. When every Rust job had `needs: [format]`, one formatting failure downgraded them all to `skipped`, which the gate read as passing, and lint, test, MSRV, and deny silently did not run for two months. The gate no longer relies on that: it accepts `skipped` only for jobs named in its `SKIPPABLE` allowlist (currently `pr-title`), so a `needs:` edge now fails the gate instead of slipping past it.

## Composite actions

Reusable actions in `.github/actions/`:

| Action  | Purpose                                            |
| ------- | -------------------------------------------------- |
| `setup` | Installs mise and the toolchain declared in `.mise.toml` |

mise owns the Rust toolchain, so there is no separate toolchain action — a second
one would leave which `cargo` runs dependent on PATH ordering. Pass the `tools`
input to install a single tool, and set `MISE_AUTO_INSTALL=0` on the step that
runs the task: `mise run` otherwise installs the rest of `.mise.toml` on demand,
compiling cargo-nextest from source, and the scoping saves nothing.

The checks themselves are not restated here: CI runs `mise run <task>`, and
`.mise.toml` holds the exact flags.

**Design pattern**: the action installs tools; jobs run the checks. A job that runs a check composes `harden-runner` → `checkout` → `setup` → `mise run <task>`.

## cargo-deny

Config: `deny.toml` — enforces dependency policies:

- **Licenses**: Allow-list (MIT, Apache-2.0, BSD, ISC, Unicode)
- **Advisories**: Deny known vulnerabilities, warn on unmaintained
- **Bans**: Warn on duplicate dependency versions
- **Sources**: Only allow crates.io (deny unknown registries/git deps)

## Release pipeline

1. Push to main → **Release PLZ** creates/updates a release PR (version bump + changelog), but only if at least one commit matches `^(feat|fix|perf|refactor|revert)`. Commits like `docs:`, `ci:`, `chore:` still appear in the changelog but don't trigger a release on their own.
2. Merge release PR → **Release PLZ** publishes to crates.io (OIDC) and tags the version
3. Version tag → **Release** (cargo-dist) builds binaries, creates GitHub Release, publishes Homebrew formula

## CI optimizations

- **Security audit**: `audit.yml` is called by CI on every push and PR, and also runs a weekly cron (Mondays 6am UTC) — cargo-deny fetches the advisory database at run time, so a new advisory against an unchanged `Cargo.lock` needs a trigger no commit provides
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

- **`release.yml`**: `[dist.github-action-commits]` in `dist-workspace.toml` is the source of truth, and `dist generate-ci` writes those SHAs into the workflow. A `renovate.json` customManager updates the TOML, keyed on each pin's `# vX.Y.Z` comment — so every SHA there must be a release tag's own commit, because an untagged mid-stream commit gives Renovate no version to resolve and gives a reviewer nothing to read. `mise run check` fails when the TOML and `release.yml` disagree.
- **All other workflows**: Pins managed manually with `# vX` comments. Renovate auto-updates via `helpers:pinGitHubActionDigestsToSemver`, which keeps the version comment in step with the digest.

Renovate used to update `release.yml` directly while the TOML it is generated
from went untouched. The two drifted — `attest-build-provenance` sat a version
apart — and the next `dist generate-ci` would have reverted the workflow to the
TOML's older SHA: a downgrade arriving as regeneration noise, in the one
workflow holding `contents: write` and publishing binaries and attestations.
The parity check is what makes that unmergeable rather than merely discouraged.

## cargo-dist management

`release.yml` is auto-generated by cargo-dist. Configuration lives in `dist-workspace.toml`. `dprint.json` and `.github/actionlint.yaml` both exclude `release.yml` so neither conflicts with cargo-dist's generated output.

cargo-dist is pinned in `.mise.toml`, but that pin does not win on PATH by
itself: a Homebrew `cargo-dist` shadows it, because `/opt/homebrew/bin` precedes
mise's tool directories in a normal macOS shell — measured, a bare `dist` was
0.32.0 against a declared 0.30.3 even inside `mise run`. Reach the pinned one as
`mise exec -- dist`, or remove the Homebrew copy. CI is unaffected: no CI job
runs dist, and `release.yml` fetches it by curl. The version
appears three times (the mise pin, `cargo-dist-version`, and the installer URL
`dist` bakes into `release.yml`), and `mise run check` fails unless all three
agree; regenerating with the wrong binary rewrites that third copy, which is what
makes the mismatch catchable.

**Regenerating** (needed for cargo-dist upgrades or config changes):

1. Remove `allow-dirty = ["ci"]` from `dist-workspace.toml` — with it present
   `dist generate-ci` refuses outright (`'ci' is marked as allow-dirty in your
   dist config, refusing to run`), so nothing is silently overwritten
2. Update SHAs in `[dist.github-action-commits]` to latest
3. Run `dist generate-ci` — via mise, so it is the pinned version
4. Add `allow-dirty = ["ci"]` back
5. On a cargo-dist upgrade, bump the `.mise.toml` pin and `cargo-dist-version`
   together, or the version check fails
6. Commit all changes

**Never change a pin in `release.yml` alone** — `dist generate-ci` regenerates
the file from `dist-workspace.toml`, so an edit that is not mirrored there is
reverted at the next regeneration. Change the TOML, then propagate. The parity
check in `mise run check` enforces the pair, but it only compares SHAs: it
cannot tell you that some *other* hand-edit to the generated file will survive.

## Renovate

Config: `.github/renovate.json`

- Non-major updates are grouped and auto-merged, with three carve-outs that get
  a human: major bumps, a minor bump on a `<1.0.0` cargo dep (breaking under
  semver), and the mise `rust` pin (a toolchain that adds a clippy lint reds CI
  on unchanged code)
- Uses `helpers:pinGitHubActionDigestsToSemver` to keep action SHAs current
- A `customManagers` entry covers `[dist.github-action-commits]` in
  `dist-workspace.toml`, which no built-in manager reads — that gap is why
  those pins went stale while the generated `release.yml` kept being updated
- Semantic commit messages on dependency PRs
