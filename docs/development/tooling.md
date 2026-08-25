# Development Tooling

This guide covers development tools used in autocomplete-rs — installation,
daily workflow, and editor setup.

For exact command flags, see the source-of-truth configs: `.mise.toml` (tools and
tasks), `lefthook.yml` (git hooks), `.github/workflows/ci.yml` (CI).

## Overview

| JavaScript/TypeScript | Rust Equivalent     | Installed?           |
| --------------------- | ------------------- | -------------------- |
| Prettier              | `dprint` / `rumdl`  | Via mise (pre-built) |
| ESLint                | `cargo clippy`      | Built-in             |
| markdownlint-cli2     | `rumdl`             | Via mise (pre-built) |
| tsc --noEmit          | `cargo check`       | Built-in             |
| Husky                 | `lefthook`          | Via mise (pre-built) |
| lint-staged           | `lefthook`          | Via mise (pre-built) |
| npm test              | `cargo nextest run` | Via mise (cargo)     |
| Make / npm scripts    | `mise tasks`        | Via mise             |
| asdf / nvm / pyenv    | `mise`              | Install (see below)  |

Every tool is a single binary from GitHub releases, crates.io, or aqua — no Node
runtime.

### One formatter per file type

rustfmt owns `.rs`, rumdl owns `.md`, and dprint owns TOML, JSON, and YAML.
dprint configures no markdown plugin, so it never writes a `.md` file; the
`**/*.md` exclude in `dprint.json` keeps that true if one is ever added. Two
formatters writing one file oscillate, each undoing the other.

### Everything is pinned exact

`.mise.toml` pins every tool to an exact version, including the Rust toolchain.
A floating version turns CI red on code nobody touched: clippy runs under
`-D warnings`, so a toolchain release that adds a lint breaks the build with no
commit to review. Renovate proposes bumps as reviewable pull requests instead.

The pin in `.mise.toml` is the toolchain CI builds with. It is deliberately not
`Cargo.toml`'s `rust-version`, which is the lower MSRV the crate promises and is
verified separately by `mise run msrv`.

## Installation

### One-Command Setup

We use [mise](https://mise.jdx.dev) to manage all development tools:

```sh
# Install mise
cargo install mise

# Install all project tools
mise install

# Set up git hooks
mise run setup
```

mise installs the Rust toolchain (clippy, rustfmt, rust-src, rust-analyzer)
plus:

- dprint (TOML/JSON/YAML formatter)
- rumdl (markdown formatter and linter)
- lefthook (git hooks manager)
- cocogitto (conventional commit validator)
- gitleaks (secret scanner)
- actionlint + shellcheck (workflow linting; actionlint shells out to
  shellcheck for inline `run:` blocks and silently skips them without it)
- cargo-deny (dependency advisories, licenses, sources)
- cargo-nextest (fast test runner)

`core.hooksPath` must stay unset. Some tools set it, which makes git dispatch
there and bypass lefthook with no visible symptom: formatting, `cog verify`, and
the gitleaks scan all stop running. `mise run check` asserts that git still
resolves hooks to `.git/hooks`.

## Daily Commands

All commands go through mise. CI invokes these same tasks rather than restating
their commands, so local and CI cannot drift.

```sh
mise run check        # fmt + clippy + dprint + rumdl + actionlint  (CI: Static Analysis)
mise run test         # nextest + doctests                          (CI: Tests)
mise run msrv         # build against Cargo.toml's rust-version     (CI: MSRV)
mise run audit        # cargo-deny advisories/licenses/sources      (CI: Audit)
mise run scan-secrets # gitleaks over full history                  (CI: Secret Scan)
mise run ci           # all of the above
mise run fmt          # format everything
mise run build        # debug build
mise run release      # optimized build
mise run bench        # Criterion benchmarks
```

For running a specific test:

```sh
mise run test                              # Run full suite (see .mise.toml for flags)
cargo nextest run -E 'test(test_name)'     # Run one test by name (mise doesn't support filters)
```

## Git Hooks

lefthook runs automatically on commit and push. See `lefthook.yml` for exact
commands.

**Pre-commit** (auto-fixes and re-stages via `stage_fixed`):

- Rust formatting (staged files only), then clippy
- Markdown formatting, then linting
- TOML/JSON/YAML formatting
- gitleaks on staged content
- Beads (`bd hooks run pre-commit`)

Each linter is chained behind its own formatter rather than run beside it, so it
cannot read a file the formatter is mid-rewrite.

**Pre-push** (`piped`, so a fast failure skips the slow one):

- actionlint on changed workflows — here rather than pre-commit, because dprint
  rewrites those same files during pre-commit
- `mise run test` — the same nextest run and doctests CI runs

**Commit-msg:**

- Conventional commit format validation (via cocogitto)

Hooks and CI deliberately differ: hooks operate on staged files, CI on the whole
tree, so the flags differ too. Run `mise run ci` for the CI-equivalent pass.

The gitleaks pre-commit hook is the layer that prevents a leak. CI's scan is a
backstop: by the time it runs, the branch is already pushed and any secret in it
already exposed.

## Configuration Files

| File             | Purpose                       | Like              |
| ---------------- | ----------------------------- | ----------------- |
| `.mise.toml`     | Tools & tasks                 | `package.json`    |
| `lefthook.yml`   | Git hooks config              | `.husky/`         |
| `rustfmt.toml`   | Rust format rules             | prettier config   |
| `.rumdl.toml`    | Markdown format + lint rules  | `.markdownlintrc` |
| `dprint.json`    | TOML/JSON/YAML format rules   | `.prettierrc`     |
| `clippy.toml`    | Lint rules                    | `.eslintrc`       |
| `deny.toml`      | Dependency policy             | `npm audit` cfg   |
| `.gitleaks.toml` | Secret-scan allowlist         | -                 |
| `.editorconfig`  | Editor defaults before a save | `.editorconfig`   |

## Editor Integration

### VS Code

Install extensions:

- **rust-analyzer** (rust-lang.rust-analyzer)
- **CodeLLDB** (vadimcn.vscode-lldb)

Add to `.vscode/settings.json`:

```json
{
  "rust-analyzer.check.command": "clippy",
  "editor.formatOnSave": true,
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  }
}
```

### Other Editors

- **IntelliJ/RustRover**: Built-in support
- **Vim/Neovim**: Install rust-analyzer via coc.nvim or LSP
- **Emacs**: Install rustic-mode

## Continuous Integration

CI runs via GitHub Actions (`.github/workflows/ci.yml`). Six jobs: Static
Analysis, Tests, MSRV, Secret Scan, PR Title, and Audit, gated by CI Status.

The jobs are deliberately independent — none of them `needs` another. A job that
skips is one an explicit `if` excluded, never one an upstream failure stranded.
Chaining them meant a single formatting failure downgraded every other job to
`skipped`, which the gate then read as passing. The gate now accepts `skipped` only for the jobs in its `SKIPPABLE` allowlist.

`.github/actions/setup` installs mise and the toolchain. A job needing one tool
passes its name via the `tools` input, and must also set `MISE_AUTO_INSTALL=0` on
the step that runs the task — `mise run` otherwise installs the rest of
`.mise.toml` on demand and the scoping saves nothing.

See `.claude/rules/github-actions.md` for full CI/CD documentation.

## Additional Tools (Optional)

### cargo-watch

Auto-run checks on file change:

```sh
cargo install cargo-watch
cargo watch -x check -x test
```

### cargo-outdated

Check for outdated dependencies:

```sh
cargo install cargo-outdated
cargo outdated
```

### cargo-tree

Visualize dependency tree:

```sh
cargo tree
```

## Troubleshooting

### Pre-commit hook not running

Check that git is not dispatching elsewhere, then reinstall:

```sh
git rev-parse --git-path hooks    # must print .git/hooks
lefthook install
ls -la .git/hooks/
```

An empty `core.hooksPath` is not the same as an unset one — git resolves it to
the worktree root, which is why the check reads the resolved path rather than
the config value.

### Test manually

Run pre-commit checks without committing:

```sh
lefthook run pre-commit
```

## Getting Help

- **mise:** `mise help` or `mise tasks`
- **lefthook:** `lefthook run pre-commit` (test manually)

---

**TL;DR:** Run `mise run ci` before committing. It's that simple!
