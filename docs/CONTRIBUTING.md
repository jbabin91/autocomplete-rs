# Contributing to autocomplete-rs

Thank you for your interest in contributing to autocomplete-rs! 🎉

## Quick Links

- **Getting Started:**
  [docs/development/getting-started.md](development/getting-started.md)
- **Tooling Setup:** [docs/development/tooling.md](development/tooling.md)
- **Full Contributing Guide:**
  [docs/development/contributing.md](development/contributing.md)
- **Project Structure:**
  [docs/development/project-structure.md](development/project-structure.md)
- **Testing Guide:** [docs/development/testing.md](development/testing.md)

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
mise run setup
```

mise installs the pinned Rust toolchain plus:

- dprint (TOML/JSON/YAML formatter)
- rumdl (markdown formatter and linter)
- lefthook (git hooks manager)
- cocogitto (conventional commit validator)
- gitleaks (secret scanner)
- actionlint + shellcheck (workflow linting)
- cargo-deny (dependency policy)
- cargo-nextest (fast test runner)

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
mise run check       # Format check, clippy, markdown, workflows
mise run test        # Run tests
mise run ci          # Everything CI runs
```

### Pre-Commit Hooks

When you commit, `lefthook` automatically formats and lints the staged files
and scans them for secrets. Formatters re-stage what they rewrite — review with
`git diff HEAD` before pushing.

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
- **Documentation:** [docs/](./)

## Development Priorities

Check [GitHub Issues](https://github.com/jbabin91/autocomplete-rs/issues) for
current phase and priorities:

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
[full contributing guide](development/contributing.md).
