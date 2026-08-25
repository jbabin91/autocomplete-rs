---
applyTo: '**/*.md'
---

# Documentation Review Guidelines

For project architecture and code style, see `AGENTS.md`. For the command
deduplication principle, see `.claude/rules/tooling.md`.

## Command References

- Documentation must NOT duplicate exact command flags — flag raw `cargo clippy --all-targets
--all-features` or similar in markdown files
- Use `mise run <task>` abstractions or reference the source config file
  (`.mise.toml`, `lefthook.yml`, `ci.yml`)
- The only exception is AGENTS.md which has a concise command reference block

## Accuracy

- Flag bare `cargo test` for the suite — this project runs `cargo nextest run`.
  `cargo test --doc` is correct and expected, since nextest does not run doctests
- Flag socket path references that don't mention the `AUTOCOMPLETE_RS_SOCKET` env var
  override
- Flag outdated architecture descriptions that don't reflect implemented components
- Code snippets in design docs must match actual function signatures, types, and serde
  attributes — flag divergences between doc examples and `src/` implementations
- Flag incorrect type widths (e.g. `AtomicU32` when code uses `AtomicU64`), wrong
  `rename_all` values, or `Option<T>` when the field is actually required

## Style

- Markdown must pass `rumdl check` (config in `.rumdl.toml`). rumdl also formats,
  via `rumdl fmt`. dprint configures no markdown plugin, so rumdl is the only
  writer; the `**/*.md` exclude in `dprint.json` guards against one being added
- No trailing whitespace, single trailing newline
