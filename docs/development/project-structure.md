# Project Structure

This document explains the organization of the autocomplete-rs codebase and the
purpose of each component.

## Directory Layout

```sh
autocomplete-rs/
├── src/                    # Rust source code
├── shell-integration/      # Shell-specific integration scripts
├── tests/                  # Integration tests
├── benches/                # Performance benchmarks
├── specs/                  # Compiled completion specs (generated)
├── vendor/                 # Third-party code (Fig specs)
├── openspec/              # OpenSpec proposals and specs
├── docs/                   # Documentation
├── Cargo.toml             # Rust project manifest
├── Cargo.lock             # Dependency lock file
├── build.rs               # Build script (spec parsing)
└── README.md              # Project overview
```

## Source Code (`src/`)

### `src/main.rs`

**Purpose:** CLI entry point and command routing

**Responsibilities:**

- Parse command-line arguments with Clap
- Route to appropriate subcommand (daemon, complete, install)
- Handle top-level errors
- Set up logging/tracing

**Key Types:**

```rust
struct Cli {
    command: Commands,
}

enum Commands {
    Daemon { socket: String },
    Complete { buffer: String, cursor: usize },
    Install { shell: String },
}
```

**When to modify:**

- Adding new CLI commands
- Changing command-line interface
- Adding global flags

### `src/daemon/`

**Purpose:** Background process that handles completion requests

**Responsibilities:**

- Listen on Unix domain socket
- Accept concurrent connections
- Parse JSON requests
- Coordinate parser and TUI
- Send JSON responses

**Key Components:**

```sh
daemon/
├── mod.rs           # Main daemon logic
├── server.rs        # Unix socket server (future)
├── handler.rs       # Request handling (future)
└── protocol.rs      # JSON protocol types (future)
```

**Current State:** Basic structure in `mod.rs`

**Key Functions:**

- `start(socket_path: &str)` - Start daemon, listen on socket
- `handle_connection(stream)` - Process single request
- `parse_request(json)` - Deserialize request
- `send_response(stream, response)` - Send JSON response

**Performance Requirements:**

- Response time: <10ms
- Startup time: <5ms
- Memory: <50MB with all specs loaded

**When to modify:**

- Changing request/response format
- Adding new daemon features
- Optimizing connection handling

### `src/parser/`

**Purpose:** Parse command buffer and generate completions

**Responsibilities:**

- Tokenize shell command buffer
- Identify command, subcommands, options, arguments
- Match against completion specs
- Generate context-aware suggestions

**Key Components:**

```sh
parser/
├── mod.rs           # Parser coordination
├── tokenizer.rs     # Split buffer into tokens (future)
├── context.rs       # Determine completion context (future)
└── matcher.rs       # Match specs to context (future)
```

**Current State:** Stub implementation

**Key Types:**

```rust
pub struct Parser {
    specs: SpecLoader,
}

pub struct ParseContext {
    command: String,
    subcommands: Vec<String>,
    current_token: String,
    cursor_position: usize,
}

pub struct Suggestion {
    text: String,
    description: Option<String>,
    suggestion_type: SuggestionType,
}
```

**Parsing Pipeline:**

1. **Tokenize:** `"git checkout -b "` → `["git", "checkout", "-b", ""]`
2. **Context:** Identify we're at argument position after `-b` flag
3. **Match:** Find `git checkout -b <branch-name>` spec
4. **Generate:** Suggest branch names or templates

**Performance Requirements:**

- Parsing time: <5ms
- Support 600+ specs
- Handle 100+ char buffers

**When to modify:**

- Implementing spec matching logic
- Adding new completion types
- Optimizing parse performance

### `src/tui/`

**Purpose:** Render completion UI with Ratatui

**Responsibilities:**

- Display dropdown with suggestions
- Handle keyboard navigation (arrows, Enter, Esc)
- Apply themes (Catppuccin)
- Render within terminal constraints

**Key Components:**

```sh
tui/
├── mod.rs           # Main UI logic
├── widgets.rs       # Custom widgets (future)
├── theme.rs         # Color themes (future)
└── layout.rs        # Layout calculations (future)
```

**Current State:** Basic Ratatui setup

**Key Types:**

```rust
pub struct CompletionUI {
    suggestions: Vec<String>,
    selected: usize,
}

pub struct Theme {
    border: Color,
    selected_bg: Color,
    selected_fg: Color,
    text: Color,
}
```

**UI Layout:**

```text
┌─ Completions ────────────────┐
│ > git checkout               │
│   main           Default     │
│ → feature/new    Create new  │  ← Selected
│   develop        Development │
└──────────────────────────────┘
```

**Performance Requirements:**

- Render time: <10ms
- Handle 100+ suggestions with scrolling
- Smooth keyboard navigation

**When to modify:**

- Changing UI appearance
- Adding new themes
- Improving keyboard handling

### `src/specs/`

**Purpose:** Completion spec data structures (future)

**Responsibilities:**

- Define spec types (commands, options, arguments)
- Load compiled MessagePack specs
- Provide spec query API
- Cache frequently used specs

**Key Components:**

```sh
specs/
├── mod.rs           # Spec types and loader
├── loader.rs        # MessagePack loading (future)
├── types.rs         # Spec data structures (future)
└── cache.rs         # LRU cache (future)
```

**Current State:** Not yet implemented (Phase 2)

**Key Types:**

```rust
pub struct CompletionSpec {
    name: String,
    description: Option<String>,
    subcommands: Vec<Subcommand>,
    options: Vec<Option>,
    args: Vec<Argument>,
}

pub struct SpecLoader {
    cache: LruCache<String, CompletionSpec>,
}
```

**When to modify:**

- Implementing spec loading (Phase 2)
- Adding new spec features
- Optimizing spec lookups

## Shell Integration (`shell-integration/`)

### `zsh.zsh`

**Purpose:** ZLE widget for zsh integration

**Responsibilities:**

- Bind to keyboard shortcut (Alt+Space)
- Capture buffer and cursor from ZLE
- Send request to daemon via Unix socket
- Display completions
- Update buffer with selection

**Current State:** Basic widget structure

**Key Functions:**

```zsh
_autocomplete_rs_widget() {
    # Get state
    local buffer="$BUFFER"
    local cursor="$CURSOR"

    # Call daemon
    # Display UI
    # Update buffer
}

zle -N _autocomplete_rs_widget
bindkey '^[ ' _autocomplete_rs_widget  # Alt+Space
```

**When to modify:**

- Changing key binding
- Improving UI rendering in zsh
- Handling edge cases

### Future Shell Integrations

- `bash.sh` - Readline-based (Phase 4)
- `fish.fish` - Native fish completions (Phase 4)

## Build System

### `build.rs`

**Purpose:** Build-time spec parsing (future - Phase 2)

**Responsibilities:**

- Parse TypeScript specs from `vendor/autocomplete/`
- Convert to Rust data structures
- Serialize to MessagePack
- Embed in binary with `include_bytes!`

**Current State:** Stub (deno_ast disabled)

**Build Process:**

1. Read `.ts` files from `vendor/autocomplete/src/`
2. Parse with `deno_ast`
3. Extract completion data
4. Serialize to MessagePack
5. Write to `specs/*.msgpack`
6. Include in compiled binary

**When to modify:**

- Implementing spec parsing (Phase 2)
- Adding new spec sources
- Optimizing build time

### `Cargo.toml`

**Purpose:** Rust project configuration

**Key sections:**

- `[package]` - Project metadata
- `[dependencies]` - Runtime dependencies
- `[build-dependencies]` - Build-time dependencies (deno_ast)
- `[dev-dependencies]` - Test dependencies

**When to modify:**

- Adding new dependencies
- Updating versions
- Configuring features

## Testing (`tests/`)

### Integration Tests

```sh
tests/
├── integration.rs    # End-to-end tests (future)
├── daemon_test.rs    # Daemon tests (future)
├── parser_test.rs    # Parser tests (future)
└── fixtures/         # Test data
    └── specs/        # Sample specs for testing
```

**When to modify:**

- Adding new features (add tests!)
- Fixing bugs (add regression tests)
- Improving test coverage

## Benchmarks (`benches/`)

### Performance Benchmarks

```sh
benches/
├── daemon_bench.rs   # Daemon startup and IPC (future)
├── parser_bench.rs   # Parser performance (future)
└── tui_bench.rs      # TUI render time (future)
```

**When to modify:**

- Optimizing performance
- Adding new features (benchmark them!)
- Tracking performance regressions

## OpenSpec (`openspec/`)

### Project Specifications

```sh
openspec/
├── project.md        # Project context for AI
├── ROADMAP.md        # Development phases
├── AGENTS.md         # OpenSpec workflow
└── changes/          # Change proposals
    ├── add-foundation-architecture/
    ├── implement-mvp-parser/
    ├── add-fig-spec-parser/
    ├── add-theme-support/
    └── add-multi-shell-support/
```

**Purpose:** Specification-driven development

**When to use:**

- Planning new features
- Documenting architecture
- Coordinating with AI assistants

See [OpenSpec Workflow](../../openspec/AGENTS.md) for details.

## Documentation (`docs/`)

### Documentation Structure

```sh
docs/
├── README.md             # Documentation hub
├── adr/                  # Architecture Decision Records
│   ├── 0001-use-rust.md
│   ├── 0002-daemon-architecture.md
│   ├── 0003-build-time-spec-parsing.md
│   ├── 0004-direct-terminal-control.md
│   └── 0005-ratatui-for-tui.md
├── development/          # Developer guides
│   ├── getting-started.md
│   ├── project-structure.md (this file)
│   ├── testing.md
│   └── contributing.md
├── user-guide/           # User documentation
│   ├── installation.md
│   ├── configuration.md
│   └── troubleshooting.md
└── architecture/         # System design
    ├── overview.md
    ├── daemon.md
    ├── parser.md
    └── tui.md
```

**When to modify:**

- Making architectural changes (update ADRs)
- Adding user-facing features (update user guide)
- Changing development process (update dev guides)

## Data Flow

### Completion Request Flow

```text
User types "git che" + Alt+Space
         ↓
ZLE widget (zsh.zsh)
    - Captures: buffer="git che", cursor=7
         ↓
Unix Socket → Daemon (src/daemon/mod.rs)
    - Receives JSON: {"buffer":"git che","cursor":7}
         ↓
Parser (src/parser/mod.rs)
    - Tokenizes: ["git", "che"]
    - Identifies: command=git, partial=che
    - Queries specs for "git" starting with "che"
         ↓
Spec Loader (src/specs/mod.rs)
    - Loads git.msgpack
    - Finds: checkout, cherry, cherry-pick
         ↓
Daemon sends response
    - JSON: {"suggestions":[...]}
         ↓
ZLE widget receives response
         ↓
TUI (src/tui/mod.rs)
    - Renders dropdown
    - User selects "checkout"
         ↓
ZLE widget updates buffer
    - BUFFER="git checkout"
```

## Module Dependencies

```sh
main.rs
  ├── daemon (phase 1)
  │   ├── parser (phase 1-2)
  │   │   └── specs (phase 2)
  │   └── tui (phase 1)
  │       └── theme (phase 3)
  └── installer (phase 1)
      └── shell-integration/*.{zsh,sh,fish}
```

**Dependency Rules:**

- No circular dependencies
- Lower layers don't depend on higher layers
- Specs layer has no dependencies (pure data)

## File Naming Conventions

- **Modules:** `snake_case` (e.g., `parser/mod.rs`)
- **Types:** `PascalCase` (e.g., `CompletionSpec`)
- **Functions:** `snake_case` (e.g., `parse_buffer`)
- **Constants:** `SCREAMING_SNAKE_CASE` (e.g., `MAX_SUGGESTIONS`)
- **Test files:** `*_test.rs` or in `tests/`
- **Benchmark files:** `*_bench.rs` in `benches/`

## Configuration Files

- `.gitignore` - Git ignored files
- `.rustfmt.toml` - Code formatting rules (future)
- `clippy.toml` - Linter configuration (future)
- `.github/workflows/` - CI/CD (future)

## Next Steps

- Read [Getting Started](getting-started.md) to set up development
- Read [Architecture Overview](../architecture/overview.md) for system design
- Check [ROADMAP.md](../../openspec/ROADMAP.md) for current phase
- Read [Testing Guide](testing.md) for testing practices
