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

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Ensure you have Rust 1.85 or later:

```bash
rustc --version
# Should show: rustc 1.85.0 or higher
```

If you have an older version:

```bash
rustup update stable
```

### 2. Clone the Repository

```bash
cd ~/.code/github/rustProjects/  # or your preferred location
git clone https://github.com/YOUR_USERNAME/autocomplete-rs.git
cd autocomplete-rs
```

### 3. Build the Project

```bash
cargo build
```

This will:

- Download dependencies (~200MB first time)
- Compile the project
- Create binary at `target/debug/autocomplete-rs`

Expected build time: ~2-3 minutes first time, ~30s incremental

### 4. Run Tests

```bash
cargo test
```

All tests should pass. If any fail, check:

- You're on Rust 1.85+
- You're on a Unix-like system
- No autocomplete-rs daemon is already running

### 5. Install Development Build

To test your changes in your actual shell:

```bash
cargo build --release
./target/release/autocomplete-rs install zsh
```

This will:

- Create `~/.config/autocomplete-rs/` directory
- Add ZLE widget to `~/.zshrc`
- Set up shell integration

Restart your shell or run:

```bash
source ~/.zshrc
```

## Development Workflow

### Daily Development

**1. Create a Branch**

```bash
git checkout -b feature/my-awesome-feature
```

**2. Make Changes**

Edit code in `src/`:

- `src/main.rs` - CLI entry point
- `src/daemon/` - Unix socket server
- `src/parser/` - Command buffer parsing
- `src/tui/` - Ratatui UI rendering
- `src/specs/` - Completion specs

**3. Build and Test**

```bash
# Quick check (compile only)
cargo check

# Full build
cargo build

# Run tests
cargo test

# Check formatting
cargo fmt --check

# Run linter
cargo clippy
```

**4. Run Locally**

```bash
# Start daemon manually (for debugging)
./target/debug/autocomplete-rs daemon /tmp/autocomplete-rs.sock

# In another terminal, trigger completion
./target/debug/autocomplete-rs complete "git chec" 8
```

**5. Debug with Logs**

```bash
# Enable debug logging
RUST_LOG=debug ./target/debug/autocomplete-rs daemon /tmp/autocomplete-rs.sock

# Or trace level for verbose output
RUST_LOG=trace ./target/debug/autocomplete-rs daemon /tmp/autocomplete-rs.sock
```

### Hot Reloading During Development

Since the daemon runs persistently, you need to restart it to see changes:

```bash
# Kill existing daemon
pkill autocomplete-rs

# Start new version
./target/debug/autocomplete-rs daemon /tmp/autocomplete-rs.sock &

# Or use this helper script (create in project root)
./dev-reload.sh
```

Create `dev-reload.sh`:

```bash
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
│   ├── tui/
│   │   └── mod.rs       # Ratatui UI
│   └── specs/           # Completion specs (future)
├── shell-integration/
│   └── zsh.zsh          # ZLE widget
├── tests/               # Integration tests
├── benches/             # Performance benchmarks
├── openspec/            # OpenSpec proposals
│   ├── project.md       # Project context
│   ├── ROADMAP.md       # Development phases
│   └── changes/         # Change proposals
└── docs/
    ├── adr/             # Architecture decisions
    ├── development/     # This guide
    └── architecture/    # System design docs
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

```bash
cargo bench
```

This measures:

- Daemon startup time
- IPC latency
- Parser performance
- TUI render time

### Updating Dependencies

```bash
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

```bash
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

```bash
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

```bash
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

// In TUI
trace!("Rendering {} suggestions", suggestions.len());
```

## Testing

### Unit Tests

```bash
# Run all tests
cargo test

# Run specific module
cargo test parser

# Run with output
cargo test -- --nocapture
```

### Integration Tests

```bash
# Run integration tests
cargo test --test integration
```

Integration tests in `tests/` verify:

- End-to-end completion flow
- Daemon startup/shutdown
- Socket communication
- Parser correctness

### Manual Testing

1. Start daemon:

```bash
./target/debug/autocomplete-rs daemon /tmp/autocomplete-rs.sock &
```

1. Test completion:

```bash
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

1. Check [ROADMAP.md](../../openspec/ROADMAP.md) for current priorities
2. Look for issues tagged `good-first-issue`
3. Read [Contributing Guide](contributing.md)
4. Follow [OpenSpec workflow](../../openspec/AGENTS.md) for larger changes

### Code Standards

- Run `cargo fmt` before committing
- Run `cargo clippy` and fix warnings
- Add tests for new functionality
- Update documentation for user-facing changes
- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)

## Performance Guidelines

We have strict performance requirements:

- **Total latency:** <20ms (startup to completion display)
- **Daemon startup:** <5ms
- **IPC round-trip:** <1ms
- **Parser:** <5ms
- **TUI render:** <10ms

Before optimizing:

1. **Measure** with benchmarks
2. **Profile** with flamegraph
3. **Optimize** hot paths only
4. **Verify** with benchmarks again

```bash
# Install cargo-flamegraph
cargo install flamegraph

# Generate flamegraph
cargo flamegraph --bin autocomplete-rs -- daemon /tmp/test.sock
```

## Next Steps

- Read [Project Structure](project-structure.md) to understand the codebase
- Read [Architecture Overview](../architecture/overview.md) for system design
- Check [ROADMAP.md](../../openspec/ROADMAP.md) for what's being built
- Join development discussions in GitHub Issues

## Getting Help

- **Issues:** File on GitHub
- **Questions:** Open a GitHub Discussion
- **Bugs:** File detailed issue with repro steps

Welcome aboard! 🚀
