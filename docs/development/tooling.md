# Development Tooling

This guide covers development tools used in autocomplete-rs — installation,
daily workflow, and editor setup.

For exact command flags, see the source-of-truth configs: `mise.toml` (tasks),
`hk.pkl` (git hooks), `.github/workflows/ci.yml` (CI).

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

All commands go through mise. See `mise.toml` for exact flags.

```sh
mise run fmt          # Format all files (Rust, TOML, Markdown, JSON, YAML)
mise run fmt-check    # Check formatting without changing files
mise run check        # Type check (cargo check)
mise run lint         # Clippy + markdownlint
mise run test         # cargo nextest run
mise run ci           # All checks (fmt-check + check + lint + test)
mise run build        # Debug build
mise run release      # Optimized build
```

For running a specific test:

```sh
mise run test                              # Run full suite (see mise.toml for flags)
cargo nextest run -E 'test(test_name)'     # Run one test by name
```

## Git Hooks

hk runs automatically on commit and push. See `hk.pkl` for exact commands.

**Pre-commit** (`fix = true` — auto-fixes and re-stages):

- Rust formatting, linting, and type checking
- TOML, Markdown, JSON, YAML formatting
- Markdown linting

**Pre-push:**

- Full test suite via nextest

**Commit-msg:**

- Conventional commit format validation (via cocogitto)

Hooks use the same flags as CI — if it passes locally, it passes in CI.

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

CI runs via GitHub Actions (`.github/workflows/ci.yml`). Seven jobs:
Format (fail-fast gate), Lint, Test, MSRV, Deny, PR Title, CI Status.

Reusable composite actions live in `.github/actions/`.

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
- **hk:** `hk run pre-commit` (test manually)

---

**TL;DR:** Run `mise run ci` before committing. It's that simple!
