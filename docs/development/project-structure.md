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
├── .beads/                # Issue tracking (beads)
├── docs/                   # Documentation
├── Cargo.toml             # Rust project manifest
├── Cargo.lock             # Dependency lock file
├── build.rs               # Build script (spec parsing)
└── README.md              # Project overview
```

## Source Code (`src/`)

The crate is both a library (`src/lib.rs`) and a binary (`src/main.rs`). The
binary imports from the library, and integration tests in `tests/` use the
library directly.

### `src/lib.rs`

**Purpose:** Library crate root — re-exports all public modules

**Exports:** `daemon`, `engine`, `parser`, `protocol`

### `src/main.rs`

**Purpose:** CLI entry point and command routing

**Responsibilities:**

- Parse command-line arguments with Clap
- Route to appropriate subcommand (daemon, stop, status, complete, install)
- Handle top-level errors
- Set up logging/tracing
- Send shutdown message to daemon for `stop` command

**Key Types:**

```rust
enum Commands {
    Daemon { socket: String },   // Start the daemon
    Stop { socket: String },     // Send shutdown message
    Status { socket: String },   // Check if daemon is running
    Complete { buffer, cursor, socket },  // Get completions
    Install { shell: String },   // Print shell integration instructions
}
```

All socket args support `AUTOCOMPLETE_RS_SOCKET` env var override.

### `src/protocol.rs`

**Purpose:** Shared IPC protocol types and validation

**Key Types:**

- `CompletionRequest` — buffer + cursor + version
- `CompletionResponse` — list of `Suggestion`s
- `DaemonMessage` — tagged enum (`Complete` | `Shutdown`) for the envelope
  protocol. Bare `CompletionRequest` (no `"type"` field) is also accepted for
  backward compatibility.
- `ErrorResponse`, `ShutdownAck` — error and shutdown responses
- `ValidationError` — buffer too long, cursor out of bounds

**Constants:** `MAX_BUFFER_LEN` (10,000), `MAX_REQUEST_SIZE` (100KB),
`PROTOCOL_VERSION` (1)

### `src/engine.rs`

**Purpose:** Completion backend abstraction

**Key Types:**

- `CompletionEngine` trait — `fn complete(&self, request) -> CompletionResponse`
- `StubEngine` — returns empty suggestions (used in tests and benchmarks)

The trait is `Send + Sync` so the daemon can share it via `Arc<dyn
CompletionEngine>`. The daemon uses `ParserEngine` (from `src/parser/`) as
the default engine.

### `src/daemon/`

**Purpose:** Background process that handles completion requests

**Responsibilities:**

- Listen on Unix domain socket with semaphore-based backpressure
- Accept concurrent connections (max 100)
- Parse and validate JSON requests (with 1s read timeout, 100KB size limit)
- Delegate to `CompletionEngine` for completions
- Handle graceful shutdown via `CancellationToken` (Ctrl+C or shutdown message)
- Enforce single-instance via PID file

**Components:**

```sh
daemon/
├── mod.rs           # Thin facade: start() and start_with_engine()
├── server.rs        # Accept loop, shutdown orchestration, socket permissions
├── handler.rs       # Per-connection request handling with timeouts/validation
├── state.rs         # DaemonState (engine, semaphore, cancel token, metrics)
└── pid.rs           # RAII PidFile for single-instance enforcement
```

**Key Functions:**

- `start(socket_path)` — Start daemon with default `ParserEngine`
- `start_with_engine(socket_path, engine)` — Start with custom engine
- `handler::handle_connection(reader, writer, state, conn_id)` — Per-connection
  logic (generic over `AsyncRead`/`AsyncWrite` for testability)

**Performance Requirements:**

- Response time: <10ms
- Startup time: <5ms
- Memory: <50MB with all specs loaded

### `src/parser/`

**Purpose:** Parse command buffer and classify completion context

**Responsibilities:**

- Tokenize shell command buffer (quotes, escaping, operators)
- Track cursor position within the token stream
- Classify completion context (command, subcommand, option, argument, filename)
- Implement `CompletionEngine` trait for daemon integration

**Key Components:**

```sh
parser/
├── mod.rs           # Public facade with re-exports
├── tokenizer.rs     # Single-pass FSM tokenizer (~540 lines)
├── context.rs       # CompletionContext enum + analyze_context()
└── engine.rs        # ParserEngine implementing CompletionEngine
```

**Current State:** Tokenizer and context analysis implemented. Returns empty
suggestions — spec-based suggestion generation is the next phase.

**Key Types:**

```rust
pub enum TokenKind { Word, Operator }

pub struct Token {
    kind: TokenKind,
    text: String,
    start: usize,
    end: usize,
    quote_open: bool,
}

pub struct TokenizeResult {
    tokens: Vec<Token>,
    cursor_token_index: Option<usize>,
    at_word_boundary: bool,
    prefix: String,
    cursor: usize,
}

pub enum CompletionContext {
    Command,
    Subcommand { command: String },
    Option { command: String },
    Argument { command: String, position: usize },
    Filename,
}

pub struct ParserEngine;  // Stateless, Send + Sync
```

**Parsing Pipeline:**

1. **Tokenize:** FSM scans `buffer.as_bytes()` with a manual byte index
   (decoding UTF-8 only for non-ASCII bytes) — handles whitespace,
   single/double quotes, backslash escaping, multi-char operators (`||`, `&&`,
   `>>`, `|&`), cursor tracking with char-boundary clamping
2. **Context:** Walk backward from cursor to find active pipeline segment,
   count `Word` tokens to classify context
3. **Suggest:** (not yet implemented — pending spec system)

**Performance Requirements:**

- Parsing time: <5ms
- Handle 100+ char buffers

**When to modify:**

- Implementing spec matching logic (next phase)
- Adding new completion context types
- Optimizing parse performance

### `src/tui/` (planned)

**Purpose:** Inline ANSI dropdown for completion display

**Current State:** Not yet implemented. The old Ratatui-based TUI has been
removed. Will use raw ANSI escape codes via crossterm to render an inline
dropdown below the cursor (see [ADR-0006](../adr/0006-inline-ansi-dropdown.md)).

**When to implement:** Phase 1 MVP

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
├── daemon_integration.rs   # Real socket IPC tests (7 tests)
├── parser_integration.rs   # Parser Send+Sync, panic safety, context checks
├── logging_integration.rs  # Logging integration tests
└── storage_integration.rs  # Storage integration tests
```

**Current tests:**

- `start_connect_complete` — Start daemon, send request, verify response
- `shutdown_message_clean_exit` — Send shutdown, verify clean exit + cleanup
- `socket_permissions` — Verify socket is `0600` (owner-only)
- `concurrent_connections` — 10 simultaneous connections
- `malformed_json_returns_error` — Error response for bad input
- `envelope_and_bare_request_both_work` — Both `DaemonMessage` and bare request
- `pid_file_path_derivation` — PID file path from socket path

**Pattern:** Tests use atomic counter for unique temp socket paths (not
timestamps) to avoid collisions under parallel execution.

### Unit Tests

Inline unit tests across modules:

- `protocol` (12): serde round-trips, validation, `DaemonMessage` variants
- `handler` (8): valid/invalid requests, shutdown, backward compat
- `pid` (8): path derivation, process detection, acquire/release, stale cleanup
- `state` (4): connection guard, metrics, semaphore permits
- `engine` (2): stub behavior, trait object behind `Arc`
- `parser/tokenizer` (~80): words, quotes, escaping, operators, cursor
  positions, unclosed quotes, UTF-8, empty buffer
- `parser/context` (~15): command/subcommand/option/argument/filename
  classification, pipeline segments, chain operators
- `parser/engine` (3): empty suggestions, Send+Sync, empty buffer

## Benchmarks (`benches/`)

### Performance Benchmarks

Criterion-based benchmarks with `harness = false`. Each file is a standalone
binary with `criterion_main!`. Run via `mise run bench`.

```sh
benches/
├── engine.rs     # StubEngine::complete() via Arc<dyn CompletionEngine>
├── protocol.rs   # JSON deserialization + validate_request()
├── privacy.rs    # redact_buffer() + redact_sensitive_patterns()
├── handler.rs    # Full handle_connection() async roundtrip (in-memory I/O)
└── parser.rs     # tokenize() + ParserEngine::complete() (simple/quoted/pipe/complex)
```

HTML reports are generated in `target/criterion/*/report/index.html`.

**When to modify:**

- Optimizing performance
- Adding new features (benchmark them!)
- Tracking performance regressions
- Wiring in real parser (update engine bench to compare StubEngine vs real)

## Issue Tracking (`.beads/`)

**Purpose:** Git-native issue tracking for AI-supervised development

Uses [Beads](https://github.com/steveyegge/beads) for tracking features, bugs, and tasks
with dependencies. Issues are tracked via `bd` CLI commands.

**Key commands:**

- `bd ready` - Show issues ready to work (no blockers)
- `bd list` - List all issues
- `bd show <id>` - View issue details
- `bd create --title="..." --type=feature` - Create new issue

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
│   ├── 0005-ratatui-for-tui.md        # Superseded
│   └── 0006-inline-ansi-dropdown.md
├── development/          # Developer guides
│   ├── getting-started.md
│   ├── project-structure.md (this file)
│   ├── testing.md
│   └── contributing.md
├── user-guide/           # User documentation
│   ├── installation.md
│   └── troubleshooting.md
└── design/              # Design specs (pre-implementation)
    ├── overview.md
    ├── daemon.md
    ├── parser.md
    ├── overlay.md        # Overlay dropdown
    └── configuration.md  # Configuration system (Phase 3)
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
Inline Dropdown (not yet implemented)
    - Renders dropdown below cursor
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
  │   └── dropdown (phase 1, not yet implemented)
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
- `rustfmt.toml` - Rust formatting rules (100 char width, 2024 edition)
- `clippy.toml` - Clippy linting rules (cognitive complexity threshold)
- `mise.toml` - Dev tools and task runner
- `hk.pkl` - Git hooks configuration (Pkl language)
- `taplo.toml` - TOML formatting rules
- `.prettierrc.toml` - Prettier formatting rules
- `.prettierignore` - Files excluded from prettier
- `.markdownlint.json` - Markdown linting rules
- `deny.toml` - cargo-deny dependency policy (licenses, advisories, bans)
- `dist-workspace.toml` - cargo-dist release configuration
- `release-plz.toml` - release-plz versioning configuration
- `.github/workflows/` - CI/CD workflows (ci, release-plz, release, audit, codeql, branch-cleanup)
- `.github/actions/` - Reusable composite actions (setup-rust, setup-mise, static-analysis, run-tests)
- `.github/renovate.json` - Dependency update automation

## Next Steps

- Read [Getting Started](getting-started.md) to set up development
- Read [Architecture Overview](../design/overview.md) for system design
- Check project issues (`bd ready`) for current priorities
- Read [Testing Guide](testing.md) for testing practices
