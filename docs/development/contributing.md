# Contributing Guide

Thank you for your interest in contributing to autocomplete-rs! This guide will
help you make your first contribution.

## Code of Conduct

Be respectful, constructive, and professional. We're all here to build something
great together.

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork:**

   ```sh
   git clone https://github.com/jbabin91/autocomplete-rs.git
   cd autocomplete-rs
   ```

3. **Set up development environment:** Follow
   [Getting Started Guide](getting-started.md)
4. **Create a branch:**

   ```sh
   git checkout -b feat/my-contribution
   ```

## Finding Something to Work On

### Good First Issues

Look for issues labeled `good-first-issue` on GitHub. These are specifically
chosen for newcomers and include:

- Documentation improvements
- Simple bug fixes
- Unit test additions
- Code cleanup tasks

### Current Priorities

Check [GitHub Issues](https://github.com/jbabin91/autocomplete-rs/issues) for
current phase priorities:

**Phase 1 (MVP):** Foundation work

- Daemon implementation
- Parser basics
- Inline dropdown rendering
- ZLE integration

**Phase 2 (Scale):** Spec parsing

- TypeScript parser with deno_ast
- MessagePack compilation
- Spec loading and caching

**Phase 3 (Polish):** Themes and UX

- Catppuccin theme support
- Configuration system
- Documentation

**Phase 4 (Universal):** Multi-shell

- Bash integration
- Fish integration
- Cross-platform testing

### Proposing New Features

For significant new features:

1. **Check existing issues** to avoid duplicates
2. **Open an issue** on GitHub
3. **Get feedback** before implementing

Maintainers track accepted work in beads, which is backed by a Dolt remote rather
than this repository — `bd` is not needed to contribute.

## Development Workflow

### 1. Make Your Changes

**Code Standards:**

- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Run `mise run ci` to check all formatting and linting
- Fix all clippy warnings (zero warnings policy)
- Add tests for new functionality
- Update documentation

**Commit Guidelines:**

- Use conventional commits format
- Write clear commit messages
- Keep commits focused and atomic

### 2. Test Your Changes

```sh
# Run everything CI runs
mise run ci

# Or run individual checks
mise run fmt       # Format all files
mise run check     # Format check, clippy, markdown, workflows
mise run test      # Run tests via nextest

# Run benchmarks (if performance-critical)
mise run bench

# Check docs
cargo doc --no-deps --open
```

All checks must pass before submitting PR.

### 3. Update Documentation

If your changes affect:

- **User-facing features** → Update user guide
- **API** → Update rustdoc comments
- **Architecture** → Update architecture docs or create ADR
- **Development process** → Update developer guides

### 4. Submit Pull Request

```sh
# Push to your fork
git push origin feat/my-contribution

# Create PR on GitHub
```

The repository includes a PR template at
[`.github/pull_request_template.md`](../../.github/pull_request_template.md) that
GitHub fills in automatically when you open a PR. Fill out each section — at
minimum, provide a summary and link the related bead or GitHub issue.

## Code Style

### Rust Style

Follow `rustfmt` defaults:

```rust
// Good: Clear, idiomatic Rust
pub fn parse_buffer(buffer: &str, cursor: usize) -> Result<Vec<Suggestion>> {
    let tokens = tokenize(buffer);
    let context = analyze_context(&tokens, cursor);
    generate_suggestions(context)
}

// Bad: Not idiomatic
pub fn parse_buffer(buffer: &str,cursor: usize)->Result<Vec<Suggestion>>{
  let tokens=tokenize(buffer);
  return generate_suggestions(analyze_context(&tokens,cursor));
}
```

### Naming Conventions

```rust
// Types: PascalCase
struct CompletionSpec { }
enum SuggestionType { }

// Functions: snake_case
fn parse_buffer() { }
fn generate_suggestions() { }

// Constants: SCREAMING_SNAKE_CASE
const MAX_SUGGESTIONS: usize = 100;
const DEFAULT_SOCKET_PATH: &str = "~/.autocomplete-rs/daemon.sock";

// Modules: snake_case
mod daemon;
mod spec_loader;
```

### Documentation Comments

Use rustdoc format:

````rust
/// Parses a command buffer and returns completion suggestions.
///
/// This function tokenizes the buffer, determines the completion context,
/// and queries the appropriate spec for suggestions.
///
/// # Arguments
///
/// * `buffer` - The complete command line buffer
/// * `cursor` - The cursor position (0-indexed)
///
/// # Returns
///
/// A vector of suggestions, or an error if parsing fails.
///
/// # Examples
///
/// ```
/// use autocomplete_rs::Parser;
///
/// let parser = Parser::new();
/// let suggestions = parser.parse("git checkout ", 13)?;
/// assert!(!suggestions.is_empty());
/// ```
///
/// # Errors
///
/// Returns `Error::InvalidBuffer` if the buffer is malformed.
pub fn parse(&self, buffer: &str, cursor: usize) -> Result<Vec<Suggestion>> {
    // Implementation
}
````

### Error Handling

Use `thiserror` for custom errors:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Invalid buffer: {0}")]
    InvalidBuffer(String),

    #[error("Cursor position {cursor} out of bounds (buffer length: {buffer_len})")]
    CursorOutOfBounds { cursor: usize, buffer_len: usize },

    #[error("Spec not found: {0}")]
    SpecNotFound(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

// Usage
return Err(ParseError::InvalidBuffer("Empty buffer".to_string()));
```

Use `anyhow` for application-level errors:

```rust
use anyhow::{Context, Result};

pub fn load_config() -> Result<Config> {
    let path = config_path()?;
    let contents = std::fs::read_to_string(&path)
        .context("Failed to read config file")?;

    let config = toml::from_str(&contents)
        .context("Failed to parse config")?;

    Ok(config)
}
```

## Testing Standards

### Test Coverage

- New code should have >85% test coverage
- Critical paths (parser, daemon) need >90%
- Include unit tests and integration tests

### Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_happy_path() {
        // Arrange
        let input = "test input";

        // Act
        let result = function(input);

        // Assert
        assert_eq!(result, expected);
    }

    #[test]
    fn test_function_error_case() {
        let result = function("");
        assert!(result.is_err());
    }
}
```

See [Testing Guide](testing.md) for comprehensive testing practices.

## Performance Guidelines

We have strict performance requirements:

- **Total latency:** <20ms
- **Daemon startup:** <5ms
- **Parser:** <5ms
- **Inline dropdown render:** <10ms

### Before Optimizing

1. **Profile first:**

   ```sh
   cargo install flamegraph
   cargo flamegraph --bin autocomplete-rs -- daemon /tmp/test.sock
   ```

2. **Benchmark:**

   ```sh
   mise run bench
   ```

3. **Optimize hot paths only**

4. **Verify improvement:**

   ```sh
   cargo bench --all-features -- --baseline before
   ```

### Optimization Techniques

**Good:**

- Use `&str` instead of `String` when possible
- Preallocate vectors with capacity
- Avoid unnecessary clones
- Use `Cow` for conditionally owned data
- Lazy initialization with `OnceCell`

**Bad:**

- Premature optimization
- Sacrificing readability for minor gains
- Micro-optimizations without profiling

## Commit Message Format

Use [Conventional Commits](https://www.conventionalcommits.org/):

```text
<type>(<scope>): <subject>

<body>

<footer>
```

**Types:**

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting)
- `refactor`: Code refactoring
- `perf`: Performance improvement
- `test`: Adding tests
- `chore`: Maintenance tasks

**Examples:**

```text
feat(parser): add support for git subcommands

Implements parsing for git checkout, commit, and push commands.
Includes spec matching and suggestion generation.

Closes #42
```

```text
fix(daemon): prevent socket cleanup race condition

Use atomic flag to coordinate shutdown between listener
and connection handlers.

Fixes #123
```

```text
docs(adr): add decision record for Ratatui choice

Documents comparison of TUI frameworks and rationale
for choosing Ratatui over alternatives.
```

```text
perf(parser): optimize tokenization for long buffers

Reduce allocations by using string slices instead of
creating new strings for each token.

Before: 8.2ms avg for 200-char buffer
After: 2.1ms avg for 200-char buffer
```

## Review Process

### What Reviewers Look For

**Code Quality:**

- Follows Rust idioms
- Clear and readable
- Well-documented
- Properly tested

**Functionality:**

- Solves stated problem
- Handles edge cases
- No regressions
- Meets performance requirements

**Design:**

- Fits architecture
- Follows existing patterns
- Minimal complexity
- Future-proof

### Addressing Feedback

- **Be responsive:** Reply within 48 hours
- **Be open:** Consider all feedback seriously
- **Ask questions:** If unclear, ask for clarification
- **Update PR:** Push changes to same branch
- **Mark resolved:** When addressed, mark conversations resolved

### Approval Process

1. **Automated checks** must pass (CI)
2. **One approving review** required
3. **No unresolved conversations**
4. **Maintainer merges** (don't merge your own PR)

## Release Process

(For maintainers)

Releases are automated via [release-plz](https://release-plz.ieni.dev/) +
[cargo-dist](https://opensource.axo.dev/cargo-dist/):

1. Push to main triggers release-plz, which detects conventional commits since last release
2. release-plz opens a release PR with version bump + CHANGELOG update
3. Merging the release PR publishes to crates.io (via OIDC) and creates a git tag
4. The tag triggers cargo-dist, which builds cross-platform binaries and creates a GitHub Release

## Getting Help

### Documentation

- [Getting Started](getting-started.md) - Development setup
- [Project Structure](project-structure.md) - Codebase layout
- [Testing Guide](testing.md) - Testing practices
- [Design Specs](../design/overview.md) - System design
- [ADRs](../adr/) - Technical decisions

### Communication

- **Questions:** GitHub Issues
- **Bugs:** GitHub Issues
- **Features:** GitHub Issues
- **Security:** Email maintainers privately

### Response Times

- **Issues/PRs:** Usually within 2-3 days
- **Security:** Within 24 hours

## Recognition

Contributors are recognized in:

- `CONTRIBUTORS.md`
- Release notes
- Project README

Significant contributions may result in:

- Commit access
- Maintainer role
- Project governance participation

## License

By contributing, you agree that your contributions will be licensed under the
MIT License.

## Thank You!

Every contribution makes autocomplete-rs better. Whether it's code,
documentation, bug reports, or feedback—thank you for being part of this
project! 🚀
