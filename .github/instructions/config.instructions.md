---
applyTo: '**/*.toml,**/*.json,**/*.yml,.mise.toml,lefthook.yml,dprint.json'
---

# Configuration File Review Guidelines

For tooling conventions and config details, see `.claude/rules/tooling.md`.
For CI/CD workflow context, see `.claude/rules/github-actions.md`.

## Single Definition of Each Check

`.mise.toml` defines what every check runs; CI calls `mise run <task>` rather than
restating flags. Flag any workflow that inlines a check command instead of
invoking the task.

- **`.mise.toml`** — source of truth for check commands
- **`.github/workflows/ci.yml`** — decides which tasks gate a merge, not what they do
- **`lefthook.yml`** — git hooks; these legitimately differ, because hooks operate
  on staged files while CI operates on the whole tree

## Pinning

Every tool in `.mise.toml` must be an exact version — flag `latest`, `stable`, or
any floating specifier. A floating version reds CI on unchanged code.

## One Writer Per File Type

rustfmt owns `.rs`, rumdl owns `.md`, dprint owns TOML/JSON/YAML. Flag any change
that gives two formatters the same file type.

## Cargo.toml

- Rust edition must be `2024`
- New dependencies should justify their inclusion — flag large transitive dependency
  trees for small features
- Feature flags should be minimal — only enable what's needed
- `[dev-dependencies]` for test-only crates — flag test utilities in main dependencies

## lefthook.yml

- Every formatter command must set `stage_fixed: true`, or it rewrites files the
  commit does not pick up
- A formatter needs a matching check command — `rumdl fmt` exits 0 leaving
  non-auto-fixable violations behind. Chain it behind the formatter in the same
  command (`fmt && check`), never as a sibling in a `parallel` group, or the
  check can read the file before the formatter rewrote it
- `core.hooksPath` must stay unset, or git bypasses lefthook silently

## deny.toml

- Licenses: allow-list only (MIT, Apache-2.0, BSD, ISC, Unicode) — flag new license
  additions without justification
- Sources: only crates.io — flag git dependencies or unknown registries
- `unused-ignored-advisory` and `unused-allowed-license` must stay `deny`, so a
  suppression that stops applying fails instead of lingering
- Advisory ignores are scoped by ID, never by crate, and each carries a rationale
