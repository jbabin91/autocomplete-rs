---
applyTo: '**/*.md'
---

# Documentation Review Guidelines

For project architecture and code style, see `AGENTS.md`. For the command
deduplication principle, see `.claude/rules/tooling.md`.

## Command References

- Documentation must NOT duplicate exact command flags — flag raw `cargo clippy --all-targets
--all-features` or similar in markdown files
- Use `mise run <task>` abstractions or reference the source config file (mise.toml,
  hk.pkl, ci.yml)
- The only exception is AGENTS.md which has a concise command reference block

## Accuracy

- Flag references to `cargo test` — this project uses `cargo nextest run` exclusively
- Flag socket path references that don't mention the `AUTOCOMPLETE_RS_SOCKET` env var
  override
- Flag outdated architecture descriptions that don't reflect implemented components

## Style

- Markdown must pass markdownlint-cli2 (config in `.markdownlint.json`)
- Prettier formats markdown files (config in `.prettierrc.toml`)
- No trailing whitespace, single trailing newline
