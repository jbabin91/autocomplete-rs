# Getting Started - Developer Guide

Welcome to autocomplete-rs development! This guide will help you set up your
development environment and make your first contribution.

## Prerequisites

### Required

- **Rust** 1.85+ (for Rust 2024 Edition support)
- **Git** 2.0+
- **A Unix-like OS** (macOS or Linux)
  - Windows support via WSL planned for future

### Recommended

- **Zsh** 5.8+ (for testing shell integration)
- **A modern terminal** (iTerm2, Alacritty, Kitty, WezTerm, or Ghostty)
- **Visual Studio Code** or **RustRover** (optional)

## Installation

### 1. Install Rust

If you don't have Rust installed:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Ensure you have Rust 1.85 or later:

```sh
rustc --version
# Should show: rustc 1.85.0 or higher
```

If you have an older version:

```sh
rustup update stable
```

### 2. Clone the Repository

```sh
cd ~/.code/github/rustProjects/  # or your preferred location
git clone https://github.com/jbabin91/autocomplete-rs.git
cd autocomplete-rs
```

### 3. Build the Project

```sh
cargo build
```

This will:

- Download dependencies (~200MB first time)
- Compile the project
- Create binary at `target/debug/autocomplete-rs`

Expected build time: ~2-3 minutes first time, ~30s incremental

### 4. Install Development Tools

We use [mise](https://mise.jdx.dev) to manage development tools (formatters,
linters, test runner, git hooks):

```sh
# Install mise (if not already installed)
curl https://mise.run | sh

# Install all project tools
mise install

# Set up git hooks
hk install
```

### 5. Run Tests

```sh
mise run test
```

All tests should pass. If any fail, check:

- You're on Rust 1.85+
- You're on a Unix-like system
- No autocomplete-rs daemon is already running

### 6. Install Development Build

To test your changes in your actual shell:

```sh
cargo build --release
./target/release/autocomplete-rs install zsh
```

This will:

- Create `~/.config/autocomplete-rs/` directory
- Add ZLE widget to `~/.zshrc`
- Set up shell integration

Restart your shell or run:

```sh
source ~/.zshrc
```

## Development Workflow

### Daily Development

**1. Create a Branch**

```sh
git checkout -b feature/my-awesome-feature
```

**2. Make Changes**

Edit code in `src/`:

- `src/main.rs` - CLI entry point
- `src/daemon/` - Unix socket server
- `src/parser/` - Command buffer parsing
- `src/specs/` - Completion specs

**3. Build and Test**

```sh
# Quick check (compile only)
cargo check

# Full build
cargo build

# Run tests
mise run test

# Run all CI checks (format + check + lint + test)
mise run ci
```

**4. Run Locally**

```sh
# Start daemon manually (for debugging)
./target/debug/autocomplete-rs daemon /tmp/autocomplete-rs.sock

# In another terminal, trigger completion
./target/debug/autocomplete-rs complete "git chec" 8
```

**5. Debug with Logs**

```sh
# Enable debug logging
RUST_LOG=debug ./target/debug/autocomplete-rs daemon /tmp/autocomplete-rs.sock

# Or trace level for verbose output
RUST_LOG=trace ./target/debug/autocomplete-rs daemon /tmp/autocomplete-rs.sock
```

### Hot Reloading During Development

Since the daemon runs persistently, you need to restart it to see changes:

```sh
# Kill existing daemon
pkill autocomplete-rs

# Start new version
./target/debug/autocomplete-rs daemon /tmp/autocomplete-rs.sock &

# Or use this helper script (create in project root)
./dev-reload.sh
```

Create `dev-reload.sh`:

```sh
#!/bin/bash
pkill autocomplete-rs
cargo build && ./target/debug/autocomplete-rs daemon /tmp/autocomplete-rs.sock &
```

### IDE Setup

#### Visual Studio Code

Install extensions:

- **rust-analyzer** (rust-lang.rust-analyzer)
- **CodeLLDB** (vadimcn.vscode-lldb) for debugging

Recommended `settings.json`:

```json
{
  "rust-analyzer.check.command": "clippy",
  "rust-analyzer.cargo.features": "all"
}
```

#### RustRover

RustRover has built-in Rust support. Just open the project directory.

## Project Structure

```sh
autocomplete-rs/
├── src/
│   ├── main.rs          # CLI entry point
│   ├── daemon/
│   │   └── mod.rs       # Unix socket server
│   ├── parser/
│   │   └── mod.rs       # Command parsing
│   └── specs/           # Completion specs (Phase 2 — not yet created)
├── shell-integration/
│   └── zsh.zsh          # ZLE widget
├── .beads/              # Issue tracking (beads)
├── .github/             # CI workflows, templates, actions
└── docs/
    ├── adr/             # Architecture decisions
    ├── development/     # This guide
    └── design/          # Design specs (pre-implementation)
```

See [Project Structure](project-structure.md) for detailed explanation of each
module.

## Common Tasks

### Adding a New Command

1. Add enum variant in `src/main.rs`:

```rust
#[derive(Subcommand)]
enum Commands {
    // ... existing commands
    MyCommand { arg: String },
}
```

1. Handle in main:

```rust
match cli.command {
    Commands::MyCommand { arg } => {
        // implementation
    }
}
```

### Adding a New Completion Spec

(This is Phase 2 work - spec parsing not yet implemented)

1. Add TypeScript spec to `vendor/autocomplete/src/`
2. Rebuild (build.rs will parse it)
3. Load in parser:

```rust
let spec = spec_loader.load("my-command")?;
```

### Running Benchmarks

```sh
cargo bench
```

This measures:

- Daemon startup time
- IPC latency
- Parser performance
- Inline dropdown render time

### Updating Dependencies

```sh
# Check for outdated dependencies
cargo outdated

# Update all dependencies
cargo update

# Update to latest compatible versions
cargo upgrade
```

Always test thoroughly after updating dependencies!

## Debugging

### Common Issues

**Issue: "Address already in use" error**

Solution: Kill existing daemon

```sh
pkill autocomplete-rs
# or
rm /tmp/autocomplete-rs.sock
```

**Issue: Completions not appearing**

Check:

1. Is daemon running? `ps aux | grep autocomplete-rs`
2. Socket exists? `ls -la /tmp/autocomplete-rs.sock`
3. ZLE widget bound? `bindkey | grep autocomplete`

Debug:

```sh
RUST_LOG=debug ./target/debug/autocomplete-rs daemon /tmp/autocomplete-rs.sock
```

**Issue: Build fails with deno_ast errors**

Currently expected - deno_ast is disabled until Phase 2. Comment it out in
Cargo.toml:

```toml
[build-dependencies]
# deno_ast = "0.40"  # TODO: Enable in Phase 2
```

### Using LLDB Debugger

```sh
# Build with debug symbols
cargo build

# Run under debugger
lldb ./target/debug/autocomplete-rs

# Set breakpoint
(lldb) b daemon::start
(lldb) run daemon /tmp/autocomplete-rs.sock
```

### Tracing Requests

Add tracing to see request flow:

```rust
use tracing::{debug, info, trace};

// In daemon
info!("Received completion request: buffer={}, cursor={}", buffer, cursor);

// In parser
debug!("Parsed tokens: {:?}", tokens);

// In dropdown
trace!("Rendering {} suggestions", suggestions.len());
```

## Testing

### Unit Tests

```sh
# Run all tests
mise run test

# Run specific module
cargo nextest run -E 'test(parser)'

# Run with output visible
cargo nextest run --no-capture
```

### Integration Tests

```sh
# Run integration tests
cargo nextest run --test integration
```

Integration tests in `tests/` verify:

- End-to-end completion flow
- Daemon startup/shutdown
- Socket communication
- Parser correctness

### Manual Testing

1. Start daemon:

```sh
./target/debug/autocomplete-rs daemon /tmp/autocomplete-rs.sock &
```

1. Test completion:

```sh
./target/debug/autocomplete-rs complete "git checkout " 13
```

Expected output (when specs implemented):

```json
{
  "suggestions": [
    { "text": "main", "description": "Switch to main branch" },
    { "text": "-b", "description": "Create new branch" }
  ]
}
```

## Contributing

Ready to contribute? Great!

1. Check project issues (`bd ready`) for current priorities
2. Look for issues tagged `good-first-issue`
3. Read [Contributing Guide](contributing.md)

### Code Standards

- Run `mise run ci` before committing (or let hk pre-commit hooks handle it)
- Fix clippy warnings (zero warnings policy)
- Add tests for new functionality
- Update documentation for user-facing changes
- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)

## Performance Guidelines

We have strict performance requirements:

- **Total latency:** <20ms (startup to completion display)
- **Daemon startup:** <5ms
- **IPC round-trip:** <1ms
- **Parser:** <5ms
- **Inline dropdown render:** <10ms

Before optimizing:

1. **Measure** with benchmarks
2. **Profile** with flamegraph
3. **Optimize** hot paths only
4. **Verify** with benchmarks again

```sh
# Install cargo-flamegraph
cargo install flamegraph

# Generate flamegraph
cargo flamegraph --bin autocomplete-rs -- daemon /tmp/test.sock
```

## Next Steps

- Read [Project Structure](project-structure.md) to understand the codebase
- Read [Architecture Overview](../design/overview.md) for system design
- Check project issues (`bd ready`) for what's being built
- Check GitHub Issues for current work

## Getting Help

- **Issues:** File on GitHub
- **Bugs:** File detailed issue with repro steps

Welcome aboard! 🚀
