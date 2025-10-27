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

- **Rust:** 1.85+ (for Rust 2024 Edition)
- **OS:** macOS or Linux

### 1. Fork and Clone

```bash
git clone https://github.com/YOUR_USERNAME/autocomplete-rs.git
cd autocomplete-rs
```

### 2. Install Development Tools

```bash
# Install mise (tool manager)
cargo install mise

# Install all project tools automatically
mise install

# Set up git hooks
hk install
```

mise will automatically install all required tools:

- taplo (TOML formatter - via cargo)
- prettier (JSON/Markdown/YAML formatter - via npm)
- markdownlint-cli2 (markdown linter - via npm)
- hk (git hooks manager - via cargo)
- pkl (hk configuration language - via homebrew)

### 3. Build and Test

```bash
# Build
cargo build

# Run tests
cargo test

# Run all checks (what CI runs)
mise run ci
```

## Development Workflow

### Daily Commands

```bash
# Format all files
mise run fmt

# Check formatting
mise run fmt-check

# Lint
mise run lint

# Type check
mise run check

# Run tests
mise run test

# Run all CI checks
mise run ci
```

### Pre-Commit Hooks

When you commit, `hk` automatically runs (using builtins):

1. ✅ `cargo_fmt` - Rust formatting (hk builtin)
2. ✅ `cargo_clippy` - Rust linting (hk builtin)
3. ✅ `cargo_check` - Type checking (hk builtin)
4. ✅ `prettier` - JSON/Markdown/YAML formatting (hk builtin)
5. ✅ `taplo` - TOML formatting (hk builtin)
6. ✅ `markdown_lint` - Markdown linting (hk builtin)

**Auto-fix enabled:** Issues are automatically fixed when possible. Review the
fixes with `git diff` and re-stage with `git add .`

### Configuration Files

| File                 | Purpose                   | Like              |
| -------------------- | ------------------------- | ----------------- |
| `mise.toml`          | Tools & tasks             | `package.json`    |
| `hk.pkl`             | Git hooks (all builtins!) | `.husky/`         |
| `rustfmt.toml`       | Rust format rules         | part of prettier  |
| `.markdownlint.json` | Markdown lint rules       | `.markdownlintrc` |
| `.prettierrc.json`   | Prettier format rules     | `.prettierrc`     |
| `taplo.toml`         | TOML format rules         | (TOML-specific)   |
| `clippy.toml`        | Lint rules                | `.eslintrc`       |

## Making Changes

1. **Create a branch:**

   ```bash
   git checkout -b feature/my-feature
   ```

2. **Write code and tests:**
   - Follow existing code style
   - Add tests for new features
   - Update documentation

3. **Ensure all checks pass:**

   ```bash
   mise run ci
   ```

4. **Commit:**

   ```bash
   git add .
   git commit -m "feat: add my feature"
   ```

   Pre-commit hooks will run automatically.

5. **Push and open PR:**

   ```bash
   git push origin feature/my-feature
   ```

   Then open a Pull Request on GitHub.

## Code of Conduct

Be respectful, constructive, and professional. We're all here to build something
great together.

## Getting Help

- **Questions:**
  [GitHub Discussions](https://github.com/jacebabin/autocomplete-rs/discussions)
- **Bug Reports:**
  [GitHub Issues](https://github.com/jacebabin/autocomplete-rs/issues)
- **Documentation:** [docs/](docs/)

## Development Priorities

Check [ROADMAP.md](openspec/ROADMAP.md) for current phase and priorities:

**Phase 1 (MVP) - Current:**

- Parser implementation
- Basic spec matching
- Integration tests

**Good First Issues:**

Look for issues tagged
[`good-first-issue`](https://github.com/jacebabin/autocomplete-rs/labels/good-first-issue):

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
