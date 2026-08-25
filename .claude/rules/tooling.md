---
paths:
  - 'lefthook.yml'
  - '.mise.toml'
  - 'dprint.json'
  - '.rumdl.toml'
  - '.gitleaks.toml'
  - '.editorconfig'
  - 'rustfmt.toml'
  - 'clippy.toml'
  - 'deny.toml'
  - 'build.rs'
  - 'Cargo.toml'
---

# Tooling & Config Rules

## Pinning

Every tool in `.mise.toml` is pinned to an exact version, the Rust toolchain
included. Never use `"latest"` or `"stable"`. Clippy runs under `-D warnings`, so
a floating version reds CI on code nobody touched — this repo lost two months of
CI to a `latest` prettier that changed how it formatted an untouched file.
Renovate proposes bumps as reviewable PRs; the `rust` pin is excluded from
automerge.

The `.mise.toml` rust pin is the toolchain CI builds with. `Cargo.toml`'s
`rust-version` is the lower MSRV the crate promises. They are different numbers
on purpose, and `mise run msrv` verifies the second.

## One writer per file type

- rustfmt owns `.rs`
- rumdl owns `.md`
- dprint owns TOML, JSON, YAML

dprint configures no markdown plugin, which is what actually enforces this; the
`**/*.md` exclude in `dprint.json` keeps it true if one is ever added. Two
formatters on one file oscillate, each undoing the other. Never add a second
writer for a file type.

`dprint` applies cargo-sort-style key ordering to `Cargo.toml`. This is stable
after the first pass — do not fight it by hand.

## Generated files

Some files are owned by a generator and must be excluded from formatters and
linters rather than fixed in place:

- `.github/workflows/release.yml` — cargo-dist (`dist generate-ci`); excluded in
  `dprint.json` and `.github/actionlint.yaml`
- `CHANGELOG.md` — release-plz via git-cliff; excluded in `.rumdl.toml`

## lefthook (Git Hooks)

- `core.hooksPath` must stay unset. Tools that set it (beads `bd init` among
  them) make git dispatch there and bypass lefthook silently — formatting,
  `cog verify`, and the gitleaks scan all stop running with no visible symptom.
  `mise run check` asserts this.
- Beads runs through the tracked `.beads/hooks/*` shims, which beads generates
  and which already handle a missing `bd`, the BEADS_HOOK_TIMEOUT watchdog, and
  exit 124/142/3. Do not reimplement that logic in `lefthook.yml`.
- Never run `bd hooks install --beads`: it sets `core.hooksPath`, which bypasses
  lefthook. Copy or regenerate the shim files directly instead.
- Beads hooks use `TERM=dumb` to suppress terminal escape sequences.
- `stage_fixed: true` on formatters re-stages what they rewrite.
- A linter runs chained behind its own formatter (`fmt && check`), never as a
  sibling in the same `parallel` group: rustfmt and dprint truncate in place, so
  a concurrent reader can see a zero-byte file, and a check racing its own
  formatter rejects commits the formatter was about to fix.
- `fmt-rust` uses `rustfmt --config=skip_children=true` rather than `cargo fmt`,
  so it touches only staged files. Without `skip_children`, rustfmt follows `mod`
  declarations and rewrites unstaged siblings, which `stage_fixed` then leaves
  dirty in the working tree.

## Rust Config

- `rustfmt.toml`: 100 char max width, Rust 2024 edition, Unix newlines
- `clippy.toml`: cognitive-complexity-threshold = 30
- `Cargo.toml`: Rust 2024 edition, uses tokio (full), tokio-util (CancellationToken), clap (derive + env), serde, anyhow, thiserror, tracing, libc
- `build.rs`: stub for Phase 2 (will parse Fig TypeScript specs with deno_ast)

## Dependency policy (`deny.toml`)

- `unused-ignored-advisory` and `unused-allowed-license` are both `deny`. An
  entry that stops applying fails the build, which forces removal instead of
  leaving a stale suppression behind.
- Advisory ignores are scoped by ID, never by crate, so a real vulnerability in
  the same dependency still fails.
- There are currently no ignores, and adding one needs a reachability argument
  that survives being run. Two suppressions were written here and both were
  wrong: one claimed a path the macOS build never compiled, when two Linux
  targets ship it; the other claimed no upgrade existed, when bumping the parent
  crate pulled the patched version. Check whether the dependency can simply be
  removed or updated before suppressing it.

## Testing

- **Always use nextest** — `cargo test` runs integration tests as parallel threads in one process; nextest runs each test as a separate process (matters for tests that create temp files/sockets)
- nextest does not run doctests, so `mise run test` runs `cargo test --doc` as a second command
- Integration tests that create temp socket paths must use atomic counters (not timestamps) for uniqueness
- The crate is both a library (`lib.rs`) and binary (`main.rs`) — this enables integration tests in `tests/` to `use autocomplete_rs::*`

## CI / mise Consistency

`.mise.toml` is the single definition of what each check runs. CI calls
`mise run <task>` rather than restating the commands, so the two cannot drift.
Do not duplicate exact flags into a workflow or into docs.

- **`.mise.toml`** — source of truth for check commands
- **`.github/workflows/ci.yml`** — decides which tasks gate a merge, not what they do
- **`lefthook.yml`** — git hooks; these legitimately differ, because hooks operate
  on staged files while CI operates on the whole tree

**`--locked` scope:** mise tasks use `--locked` because CI invokes them directly.
It fails when `Cargo.lock` is behind `Cargo.toml` instead of silently rewriting
it. Run `cargo update` deliberately when a bump is intended.

## CI job independence

Jobs in `ci.yml` must not `needs` one another. Chaining them meant one formatting
failure downgraded every other job to `skipped`, and the gate read `skipped` as
passing — so Lint, Test, MSRV, and Deny silently did not run for two months.

The gate no longer takes that on trust: it accepts `skipped` only for the jobs
named in its `SKIPPABLE` allowlist (currently `pr-title` alone), so a `needs:`
edge now fails the gate rather than slipping past it. Nor may a job be gated on
a branch name — `head_ref` is chosen by whoever opens the pull request.

## Benchmarking

- Run via `mise run bench` (see `tasks.bench` in `.mise.toml` for exact flags)
- Criterion benchmarks live in `benches/` with `harness = false`
- Current suites: `engine`, `protocol`, `privacy`, `handler`, `parser`
- HTML reports generated in `target/criterion/**/report/index.html`
- Not in CI (noisy on shared runners) — run locally for regression detection
- Deliberately absent from the pre-push hook, which runs actionlint and
  `mise run test` — benchmarks are too slow to gate a push

## Adding New Tools

- Install via mise, pinned exact (add to `[tools]` in `.mise.toml`)
- Add the check to the relevant `.mise.toml` task, so CI picks it up for free
- If it needs a pre-commit check, add a step in `lefthook.yml` scoped by `glob`,
  chained behind whichever formatter writes the files it reads
- Prefer a single binary (GitHub releases, crates.io, or aqua) over anything
  needing a language runtime
