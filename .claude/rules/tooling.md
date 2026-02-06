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

## mise.toml (Task Runner)

- Manages dev tools: taplo, hk, pkl, cocogitto (pre-built binaries); cargo-nextest (cargo); prettier, markdownlint-cli2 (npm)
- Key tasks: `fmt`, `lint`, `test`, `build`, `ci` (runs all checks)
- `ci` task depends on: fmt-check, check, lint, test
- Clippy runs with `-D warnings` (zero warnings policy)

## Rust Config

- `rustfmt.toml`: 100 char max width, Rust 2024 edition, Unix newlines
- `clippy.toml`: cognitive-complexity-threshold = 30
- `Cargo.toml`: Rust 2024 edition, uses tokio (full), tokio-util (CancellationToken), clap (derive + env), serde, anyhow, thiserror, tracing, libc
- `build.rs`: stub for Phase 2 (will parse Fig TypeScript specs with deno_ast)

## Testing

- **Always use nextest** — both `mise run test` and the pre-push hook use `cargo nextest run`
- `cargo test` runs integration tests as parallel threads in one process; nextest runs each test as a separate process — this matters for tests that create temp files/sockets
- Integration tests that create temp socket paths must use atomic counters (not timestamps) for uniqueness, even with nextest, to be defensive
- The crate is both a library (`lib.rs`) and binary (`main.rs`) — this enables integration tests in `tests/` to `use autocomplete_rs::*`

## Adding New Tools

- Install via mise (add to `[tools]` section in mise.toml)
- If it needs a pre-commit check, add as a step in hk.pkl
- Use hk builtins when available (`Builtins.cargo_fmt`, `Builtins.prettier`, etc.)
- Verify builtins exist before using: `pkl eval -f json "package://github.com/jdx/hk/...#/Builtins.pkl" | python3 -c "import json,sys; [print(k) for k in sorted(json.load(sys.stdin).keys())]"`
- `Builtins.taplo` handles BOTH validation (`taplo check`) and formatting (`taplo format` via `fix`) — there is no separate `taplo_format` builtin in v1.19.0
