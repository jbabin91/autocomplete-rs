# Development Tooling

This guide covers all development tools used in autocomplete-rs - formatters,
linters, git hooks, and task running.

## Overview

| JavaScript/TypeScript | Rust Equivalent     | Installed?           |
| --------------------- | ------------------- | -------------------- |
| Prettier              | `prettier`          | Via mise (npm)       |
| ESLint                | `cargo clippy`      | Built-in             |
| markdownlint-cli2     | `markdownlint-cli2` | Via mise (npm)       |
| tsc --noEmit          | `cargo check`       | Built-in             |
| Husky                 | `hk`                | Via mise (pre-built) |
| lint-staged           | `hk`                | Via mise (pre-built) |
| npm test              | `cargo nextest run` | Via mise (cargo)     |
| Make / npm scripts    | `mise tasks`        | Via mise             |
| asdf / nvm / pyenv    | `mise`              | Install (see below)  |
| -                     | `taplo`             | Via mise (pre-built) |

**Formatting Approach:**

- **Rust files (`.rs`):** `cargo fmt --all` (via hk)
- **TOML files (`.toml`):** `taplo fmt` (via hk)
- **Markdown/JSON/YAML:** `prettier` (via hk)
- **Markdown linting:** `markdownlint-cli2` (via hk)

## Installation

### One-Command Setup

We use [mise](https://mise.jdx.dev) to manage all development tools:

```sh
# Install mise
cargo install mise

# Install all project tools
mise install

# Set up git hooks
hk install
```

That's it! mise will automatically install:

- taplo (TOML formatter - pre-built binary)
- hk (git hooks manager - pre-built binary)
- pkl (hk configuration language - pre-built binary)
- cocogitto (conventional commit validator - pre-built binary)
- cargo-nextest (fast test runner - via cargo)
- prettier (JSON/Markdown/YAML formatter - via npm)
- markdownlint-cli2 (markdown linter - via npm)

### Manual Installation (Alternative)

If you prefer not to use mise:

```sh
# Install pre-built binaries (see individual tool docs for methods)
# taplo: https://taplo.tamasfe.dev
# hk: https://github.com/jdx/hk
# pkl: https://pkl-lang.org

# Install via cargo
cargo install cargo-nextest

# Install via npm
npm install -g prettier
npm install -g markdownlint-cli2

# Set up git hooks
hk install
```

## Daily Commands

### Using mise (Recommended)

```sh
# Format all files
mise run fmt

# Check formatting without changing files
mise run fmt-check

# Type check
mise run check

# Lint with clippy
mise run lint

# Run tests
mise run test

# Run all CI checks
mise run ci

# Build release
mise run release
```

### Direct Commands (Alternative)

```sh
# Format Rust files
cargo fmt

# Format TOML files
taplo fmt

# Format other files (JSON, Markdown, YAML)
prettier --write '**/*.{json,md,yml,yaml}'

# Lint markdown
markdownlint-cli2 '**/*.md'

# Run clippy (like eslint) — matches CI flags
cargo clippy --all-targets --all-features -- -D warnings

# Fix auto-fixable issues
cargo clippy --fix --allow-dirty --allow-staged --all-targets --all-features

# Quick type check (like tsc --noEmit)
cargo check --all-targets --all-features

# Run all tests
cargo nextest run --all-features

# Run specific test
cargo nextest run -E 'test(test_name)'

# Run with output (stdout visible)
cargo nextest run --no-capture
```

## Pre-Commit Hooks

When you commit, hk automatically runs (flags match CI exactly):

1. ✅ `cargo_fmt` — `cargo fmt --all -- --check`
2. ✅ `cargo_clippy` — `cargo clippy --all-targets --all-features -- -D warnings`
3. ✅ `cargo_check` — `cargo check --all-targets --all-features`
4. ✅ `prettier` — `prettier --check` (hk builtin)
5. ✅ `taplo` — `taplo fmt --check`
6. ✅ `markdown_lint` — `markdownlint-cli2`

**Auto-fix enabled:** If issues are found, hk automatically runs fix commands.
Review fixes with `git diff` and re-stage with `git add .`

Before push, it runs:

1. ✅ `cargo nextest run --all-features --no-tests=warn`

## Configuration Files

| File                 | Purpose               | Like              |
| -------------------- | --------------------- | ----------------- |
| `mise.toml`          | Tools & tasks         | `package.json`    |
| `hk.pkl`             | Git hooks config      | `.husky/`         |
| `rustfmt.toml`       | Rust format rules     | prettier config   |
| `.markdownlint.json` | Markdown lint rules   | `.markdownlintrc` |
| `.prettierrc.toml`   | Prettier format rules | `.prettierrc`     |
| `taplo.toml`         | TOML format rules     | (TOML-specific)   |
| `clippy.toml`        | Lint rules            | `.eslintrc`       |

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

CI runs via GitHub Actions (`.github/workflows/ci.yml`) with seven jobs:

1. **Format** — `cargo fmt --check` + `taplo` + `prettier` + `markdownlint-cli2` (fail-fast gate)
2. **Lint** — `cargo clippy --locked --all-targets --all-features` (needs Format)
3. **Test** — `cargo nextest run --locked --all-features --no-tests=warn` (needs Format)
4. **MSRV** — `cargo check --locked` with minimum supported Rust version (needs Format)
5. **Deny** — `cargo-deny` license/advisory/ban/source checks (needs Format)
6. **PR Title** — Conventional commit format validation (PRs only)
7. **CI Status** — Gate job aggregating all results into a summary table

Reusable composite actions in `.github/actions/`:

- **`setup-rust`** — Installs Rust (rustfmt + clippy), cargo-nextest, and rust-cache
- **`setup-mise`** — Installs mise with taplo, prettier, and markdownlint-cli2
- **`static-analysis`** — Runs all formatting checks (assumes tools installed)
- **`run-tests`** — Runs `cargo nextest run --locked --all-features --no-tests=warn` (assumes tools installed)

See `.claude/rules/github-actions.md` for full CI/CD documentation.

## Additional Tools (Optional)

### cargo-watch

Auto-run checks on file change:

```sh
cargo install cargo-watch
cargo watch -x check -x test
```

### cargo-audit

Security vulnerability scanning:

```sh
cargo install cargo-audit
cargo audit
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

Reinstall hooks:

```sh
hk install
```

Verify installation:

```sh
ls -la .git/hooks/
```

### Test manually

Run pre-commit checks without committing:

```sh
hk run pre-commit
```

## Getting Help

- **mise:** `mise help` or `mise tasks`
- **Rustfmt:** `cargo fmt --help`
- **Clippy:** `cargo clippy --help`
- **prettier:** `prettier --help`
- **taplo:** `taplo --help`
- **markdownlint:** `markdownlint-cli2 --help`
- **hk:** `hk run pre-commit` (test manually)

---

**TL;DR:** Run `mise run ci` before committing. It's that simple!
