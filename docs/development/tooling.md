# Development Tooling

This guide covers all development tools used in autocomplete-rs - formatters,
linters, git hooks, and task running.

## Overview

| JavaScript/TypeScript | Rust Equivalent     | Installed?          |
| --------------------- | ------------------- | ------------------- |
| Prettier              | `prettier`          | Via mise (npm)      |
| ESLint                | `cargo clippy`      | ✅ Built-in         |
| markdownlint-cli2     | `markdownlint-cli2` | Via mise (npm)      |
| tsc --noEmit          | `cargo check`       | ✅ Built-in         |
| Husky                 | `hk`                | Via mise            |
| lint-staged           | `hk`                | Via mise            |
| npm test              | `cargo test`        | ✅ Built-in         |
| Make / npm scripts    | `mise tasks`        | Via mise            |
| asdf / nvm / pyenv    | `mise`              | Install (see below) |
| -                     | `taplo`             | Via mise (cargo)    |

**Formatting Approach:**

- **Rust files (`.rs`):** `cargo fmt` (via hk builtin)
- **TOML files (`.toml`):** `taplo` (via hk builtin)
- **Markdown/JSON/YAML:** `prettier` (via hk builtin)
- **Markdown linting:** `markdownlint` (via hk builtin)

## Installation

### One-Command Setup

We use [mise](https://mise.jdx.dev) to manage all development tools:

```bash
# Install mise
cargo install mise

# Install all project tools
mise install

# Set up git hooks
hk install
```

That's it! mise will automatically install:

- taplo (TOML formatter - via cargo)
- prettier (JSON/Markdown/YAML formatter - via npm)
- markdownlint-cli2 (markdown linter - via npm)
- hk (git hooks manager - via cargo)
- pkl (hk configuration language - via homebrew)

### Manual Installation (Alternative)

If you prefer not to use mise:

```bash
# Install Rust tools
cargo install taplo-cli
cargo install hk

# Install via npm
npm install -g prettier
npm install -g markdownlint-cli2

# Install via Homebrew
brew install pkl

# Set up git hooks
hk install
```

## Daily Commands

### Using mise (Recommended)

```bash
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

```bash
# Format Rust files
cargo fmt

# Format TOML files
taplo fmt

# Format other files (JSON, Markdown, YAML)
prettier --write '**/*.{json,md,yml,yaml}'

# Lint markdown
markdownlint-cli2 '**/*.md'

# Run clippy (like eslint)
cargo clippy

# With strict warnings as errors
cargo clippy -- -D warnings

# Fix auto-fixable issues
cargo clippy --fix

# Quick type check (like tsc --noEmit)
cargo check

# Check all targets
cargo check --all-targets --all-features

# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture
```

## Pre-Commit Hooks

When you commit, hk automatically runs (using builtins):

1. ✅ `cargo_fmt` - Rust formatting (hk builtin)
2. ✅ `cargo_clippy` - Rust linting (hk builtin)
3. ✅ `cargo_check` - Type checking (hk builtin)
4. ✅ `prettier` - JSON/Markdown/YAML formatting (hk builtin)
5. ✅ `taplo` - TOML formatting (hk builtin)
6. ✅ `markdown_lint` - Markdown linting (hk builtin)

**Auto-fix enabled:** If issues are found, hk automatically runs fix commands.
Review fixes with `git diff` and re-stage with `git add .`

Before push, it runs:

1. ✅ `cargo test` - Full test suite

## Configuration Files

| File                 | Purpose                   | Like              |
| -------------------- | ------------------------- | ----------------- |
| `mise.toml`          | Tools & tasks             | `package.json`    |
| `hk.pkl`             | Git hooks (all builtins!) | `.husky/`         |
| `rustfmt.toml`       | Rust format rules         | prettier config   |
| `.markdownlint.json` | Markdown lint rules       | `.markdownlintrc` |
| `.prettierrc.json`   | Prettier format rules     | `.prettierrc`     |
| `taplo.toml`         | TOML format rules         | (TOML-specific)   |
| `clippy.toml`        | Lint rules                | `.eslintrc`       |

## Editor Integration

### VS Code

Install extensions:

- **rust-analyzer** (rust-lang.rust-analyzer)
- **CodeLLDB** (vadimcn.vscode-lldb)
- **dprint** (dprint.dprint) - optional, for format-on-save

Add to `.vscode/settings.json`:

```json
{
  "rust-analyzer.check.command": "clippy",
  "editor.formatOnSave": true,
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  },
  "[toml]": {
    "editor.defaultFormatter": "dprint.dprint"
  },
  "[markdown]": {
    "editor.defaultFormatter": "dprint.dprint"
  },
  "[json]": {
    "editor.defaultFormatter": "dprint.dprint"
  }
}
```

### Other Editors

- **IntelliJ/RustRover**: Built-in support
- **Vim/Neovim**: Install rust-analyzer via coc.nvim or LSP
- **Emacs**: Install rustic-mode

## Continuous Integration

Example GitHub Actions:

```yaml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rust-lang/setup-rust-toolchain@v1

      # Install dprint
      - run: cargo install dprint

      # Format check
      - run: dprint check

      # Lint and test
      - run: cargo clippy --all-targets -- -D warnings
      - run: cargo test --all-features
```

## Additional Tools (Optional)

### cargo-watch

Auto-run checks on file change:

```bash
cargo install cargo-watch
cargo watch -x check -x test
```

### cargo-audit

Security vulnerability scanning:

```bash
cargo install cargo-audit
cargo audit
```

### cargo-outdated

Check for outdated dependencies:

```bash
cargo install cargo-outdated
cargo outdated
```

### cargo-tree

Visualize dependency tree:

```bash
cargo tree
```

## Troubleshooting

### "No files found to format" (dprint)

Check which files dprint finds:

```bash
dprint output-file-paths
```

### Pre-commit hook not running

Reinstall hooks:

```bash
hk install
```

Verify installation:

```bash
ls -la .git/hooks/
```

### Test manually

Run pre-commit checks without committing:

```bash
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
