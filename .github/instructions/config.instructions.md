---
applyTo: '**/*.toml,**/*.pkl,**/*.json,mise.toml,hk.pkl'
---

# Configuration File Review Guidelines

For tooling conventions and config details, see `.claude/rules/tooling.md`.
For CI/CD workflow context, see `.claude/rules/github-actions.md`.

## Three-System Consistency

Any change to a check command must be reflected in all three systems:

- **CI** (`.github/workflows/ci.yml` + `.github/actions/`) — source of truth
- **mise** (`mise.toml`) — developer task runner
- **hk** (`hk.pkl`) — git hooks

Flag changes to check commands in one system without corresponding updates to the
other two.

## Cargo.toml

- Rust edition must be `2024`
- New dependencies should justify their inclusion — flag large transitive dependency
  trees for small features
- Feature flags should be minimal — only enable what's needed
- `[dev-dependencies]` for test-only crates — flag test utilities in main dependencies

## hk.pkl

- Written in Pkl (Apple's configuration language) — uses `//` comments, NOT `#`
- All Rust and TOML steps must be custom overrides, not builtins — builtins use
  minimal flags that differ from CI
- Pre-commit must have `fix = true` and `stash = "git"`
- Check commands must match CI flags exactly

## deny.toml

- Licenses: allow-list only (MIT, Apache-2.0, BSD, ISC, Unicode) — flag new license
  additions without justification
- Sources: only crates.io — flag git dependencies or unknown registries
