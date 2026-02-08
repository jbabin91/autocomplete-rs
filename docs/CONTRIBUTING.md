# Contributing to autocomplete-rs

Thank you for your interest in contributing to autocomplete-rs! 🎉

## Quick Links

- **Getting Started:**
  [docs/development/getting-started.md](docs/development/getting-started.md)
- **Tooling Setup:** [docs/development/tooling.md](docs/development/tooling.md)
- **Full Contributing Guide:**
  [docs/development/contributing.md](docs/development/contributing.md)
- **Project Structure:**
  [docs/development/project-structure.md](docs/development/project-structure.md)
- **Testing Guide:** [docs/development/testing.md](docs/development/testing.md)

## Development Environment Setup

### Prerequisites

- **Rust:** MSRV specified in [`Cargo.toml`](../Cargo.toml) (Rust 2024 Edition)
- **OS:** macOS or Linux

### 1. Fork and Clone

```sh
git clone https://github.com/jbabin91/autocomplete-rs.git
cd autocomplete-rs
```

### 2. Install Development Tools

```sh
# Install mise (tool manager)
curl https://mise.run | sh

# Install all project tools automatically
mise install

# Set up git hooks
hk install
```

mise will automatically install all required tools:

- taplo (TOML formatter - pre-built binary)
- hk (git hooks manager - pre-built binary)
- pkl (hk configuration language - pre-built binary)
- cocogitto (conventional commit validator - pre-built binary)
- cargo-nextest (fast test runner - via cargo)
- prettier (JSON/Markdown/YAML formatter - via npm)
- markdownlint-cli2 (markdown linter - via npm)

### 3. Build and Test

```sh
# Build
cargo build

# Run tests
mise run test

# Run all checks (what CI runs)
mise run ci
```

## Development Workflow

### Daily Commands

```sh
mise run fmt         # Format all files
mise run lint        # Run clippy
mise run test        # Run tests
mise run ci          # Run all CI checks (fmt-check + check + lint + test)
```

### Pre-Commit Hooks

When you commit, `hk` automatically runs formatting, linting, and type
checking. Auto-fix is enabled — review fixes with `git diff` and re-stage.

On commit message, `cocogitto` validates conventional commit format.

### Making Changes

1. **Create a branch:** `git checkout -b feat/my-feature`
2. **Write code and tests**
3. **Ensure checks pass:** `mise run ci`
4. **Commit with conventional format:** `git commit -m "feat: add my feature"`
5. **Push and open PR:** `git push origin feat/my-feature`

Branch naming: `feat/`, `fix/`, `refactor/`, `chore/`

## Code of Conduct

Be respectful, constructive, and professional. We're all here to build something
great together.

## Getting Help

- **Bug Reports:**
  [GitHub Issues](https://github.com/jbabin91/autocomplete-rs/issues)
- **Documentation:** [docs/](docs/)

## Development Priorities

Check project issues (`bd ready`) for current phase and priorities:

**Phase 1 (MVP) - Current:**

- Parser implementation
- Basic spec matching
- Integration tests

**Good First Issues:**

Look for issues tagged
[`good-first-issue`](https://github.com/jbabin91/autocomplete-rs/labels/good-first-issue):

- Documentation improvements
- Simple bug fixes
- Unit test additions
- Code cleanup

## License

By contributing, you agree that your contributions will be licensed under the
MIT License.

---

For detailed information, please read the
[full contributing guide](docs/development/contributing.md).
