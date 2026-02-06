---
paths:
  - 'hk.pkl'
  - 'PklProject'
  - 'PklProject.deps.json'
  - 'mise.toml'
  - 'rustfmt.toml'
  - 'clippy.toml'
  - 'build.rs'
  - 'Cargo.toml'
  - '.prettierrc.toml'
  - '.markdownlint.json'
  - 'taplo.toml'
---

# Tooling & Config Rules

## hk.pkl (Git Hooks)

- Written in Pkl (Apple's configuration language) — NOT TOML, NOT YAML
- Amends the hk package from `package://github.com/jdx/hk/...`
- Uses `//` for comments, NOT `#`
- hk owns ALL git hooks — other tools (beads/bd) run as steps within hk
- Beads hooks use `TERM=dumb` prefix to suppress terminal escape sequences
- Pre-commit has `fix = true` and `stash = "git"` (auto-fix and stash untracked)

## Rust Config

- `rustfmt.toml`: 100 char max width, Rust 2024 edition, Unix newlines
- `clippy.toml`: cognitive-complexity-threshold = 30
- `Cargo.toml`: Rust 2024 edition, uses tokio (full), tokio-util (CancellationToken), clap (derive + env), serde, anyhow, thiserror, tracing, libc
- `build.rs`: stub for Phase 2 (will parse Fig TypeScript specs with deno_ast)

## Testing

- **Always use nextest** — `cargo test` runs integration tests as parallel threads in one process; nextest runs each test as a separate process (matters for tests that create temp files/sockets)
- Integration tests that create temp socket paths must use atomic counters (not timestamps) for uniqueness
- The crate is both a library (`lib.rs`) and binary (`main.rs`) — this enables integration tests in `tests/` to `use autocomplete_rs::*`

## CI / hk / mise Consistency

Commands live in three executable configs — do NOT duplicate exact flags in docs:

- **CI** (`.github/workflows/ci.yml` + `.github/actions/`) — source of truth for what blocks merges
- **mise** (`mise.toml`) — developer-facing task runner (`mise run ci`)
- **hk** (`hk.pkl`) — git hooks (pre-commit, pre-push)

**Key principle:** hk hooks must use the **same flags** as CI. Do NOT use hk builtins — they use minimal flags that differ from CI. All Rust/TOML steps in hk.pkl are custom overrides.

## Adding New Tools

- Install via mise (add to `[tools]` section in mise.toml)
- If it needs a pre-commit check, add as a step in hk.pkl
- **Prefer custom steps over builtins** — verify builtin commands match CI before using them
- When adding a new check, update all three: CI workflow/action, mise task, and hk step
