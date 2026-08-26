# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.4](https://github.com/jbabin91/autocomplete-rs/compare/v0.1.3...v0.1.4) - 2026-08-26

### Added

- *(overlay)* integrate native dropdown into daemon ([#38](https://github.com/jbabin91/autocomplete-rs/pull/38))
- *(parser)* implement FSM tokenizer and context analysis ([#39](https://github.com/jbabin91/autocomplete-rs/pull/39))
- *(overlay)* spike winit + Tokio async runtime coexistence ([#35](https://github.com/jbabin91/autocomplete-rs/pull/35))
- *(overlay)* spike native overlay dropdown with NSPanel and winit ([#31](https://github.com/jbabin91/autocomplete-rs/pull/31))
- *(bench)* add Criterion benchmarking harness ([#28](https://github.com/jbabin91/autocomplete-rs/pull/28))
- *(logging)* wire request tracing into daemon handler ([#27](https://github.com/jbabin91/autocomplete-rs/pull/27))
- *(storage)* add libSQL storage layer and diagnose CLI ([#26](https://github.com/jbabin91/autocomplete-rs/pull/26))
- *(logging)* implement core logging module with privacy redaction
- *(daemon)* implement production-quality daemon with Unix socket IPC ([#23](https://github.com/jbabin91/autocomplete-rs/pull/23))

### Fixed

- *(daemon)* move the socket out of /tmp and harden private directories
- *(logging)* match the appender's real filenames when pruning logs
- *(tooling)* close CI and secret-scan gaps, migrate to dprint/rumdl/lefthook ([#170](https://github.com/jbabin91/autocomplete-rs/pull/170))
- *(deps)* update rust crate turso to 0.6 ([#104](https://github.com/jbabin91/autocomplete-rs/pull/104))
- *(deps)* update all non-major dependencies ([#61](https://github.com/jbabin91/autocomplete-rs/pull/61))

### Other

- move conventions out of .claude/rules into docs/conventions
- drop Copilot review scaffolding, port its unique rules
- *(deps)* update rust crate uuid to v1.23.5 ([#164](https://github.com/jbabin91/autocomplete-rs/pull/164))
- *(deps)* update rust crate anyhow to v1.0.103 ([#159](https://github.com/jbabin91/autocomplete-rs/pull/159))
- *(deps)* update all non-major dependencies ([#157](https://github.com/jbabin91/autocomplete-rs/pull/157))
- *(deps)* update all non-major dependencies to v1.23.3 ([#145](https://github.com/jbabin91/autocomplete-rs/pull/145))
- *(deps)* update rust crate uuid to v1.23.2 ([#129](https://github.com/jbabin91/autocomplete-rs/pull/129))
- *(deps)* update rust crate turso to v0.6.1 ([#116](https://github.com/jbabin91/autocomplete-rs/pull/116))
- *(deps)* update rust crate serde_json to v1.0.150 ([#113](https://github.com/jbabin91/autocomplete-rs/pull/113))
- *(deps)* update rust crate tokio to v1.52.3 ([#94](https://github.com/jbabin91/autocomplete-rs/pull/94))
- *(deps)* update rust crate tokio to v1.52.2 ([#86](https://github.com/jbabin91/autocomplete-rs/pull/86))
- *(deps)* update rust crate libc to v0.2.186 ([#74](https://github.com/jbabin91/autocomplete-rs/pull/74))
- *(deps)* bump rand to 0.9.4 for GHSA-cq8v-f236-94qc
- *(deps)* refresh yanked wasm-bindgen family in Cargo.lock
- update .gitignore and configuration for Dolt integration; add interactions.jsonl and metadata.json updates
- *(deps)* update rust crate anyhow to v1.0.102 ([#56](https://github.com/jbabin91/autocomplete-rs/pull/56))
- *(deps)* update rust crate clap to v4.5.60 ([#54](https://github.com/jbabin91/autocomplete-rs/pull/54))
- *(deps)* update rust crate clap to v4.5.59 ([#50](https://github.com/jbabin91/autocomplete-rs/pull/50))
- *(deps)* update rust crate uuid to v1.21.0 ([#47](https://github.com/jbabin91/autocomplete-rs/pull/47))
- *(deps)* update rust crate libc to v0.2.182 ([#42](https://github.com/jbabin91/autocomplete-rs/pull/42))
- *(deps)* update rust crate clap to v4.5.58 ([#41](https://github.com/jbabin91/autocomplete-rs/pull/41))
- update lock file
- update lock file
- *(deps)* update rust crate libc to v0.2.181 ([#37](https://github.com/jbabin91/autocomplete-rs/pull/37))
- *(deps)* update rust crate tempfile to v3.25.0 ([#36](https://github.com/jbabin91/autocomplete-rs/pull/36))
- *(test)* exercise real daemon entrypoint in integration tests ([#33](https://github.com/jbabin91/autocomplete-rs/pull/33))
- *(deps)* update rust crate core-graphics to 0.25 ([#32](https://github.com/jbabin91/autocomplete-rs/pull/32))
- *(deps)* update rust crate criterion to 0.8 ([#29](https://github.com/jbabin91/autocomplete-rs/pull/29))

## [0.1.3](https://github.com/jbabin91/autocomplete-rs/compare/v0.1.2...v0.1.3) - 2026-02-06

### Other

- audit and restructure documentation for accuracy
- add composite actions for mise, static analysis, and tests
- add cargo-nextest for faster test execution
- harden workflows and add MSRV + cargo-deny checks
- move CI/CD documentation to .claude/rules/github-actions.md
- add CI/CD section to AGENTS.md
- document cargo-dist regeneration workflow in AGENTS.md
- let cargo-dist fully manage release workflow

## [0.1.2](https://github.com/jbabin91/autocomplete-rs/compare/v0.1.1...v0.1.2) - 2026-02-06

### Other

- remove Windows target and PowerShell installer

## [0.1.1](https://github.com/jbabin91/autocomplete-rs/compare/v0.1.0...v0.1.1) - 2026-02-06

### Other

- add cargo audit job and update README badges/install methods
- release v0.1.0 ([#9](https://github.com/jbabin91/autocomplete-rs/pull/9))

## [0.1.0](https://github.com/jbabin91/autocomplete-rs/releases/tag/v0.1.0) - 2026-02-06

### Added

- add workflow formulas, CI pipeline, and dev tooling
- implement Phase 1A foundation architecture

### Fixed

- *(deps)* update ratatui to 0.30 and fix Backend trait bounds
- *(ci)* replace cocogitto commit check with PR title validation

### Other

- exclude non-essential files from crate package
- use consistent secret names for GitHub App token
- add release-plz + cargo-dist automated release pipeline
- *(deps)* update amannn/action-semantic-pull-request action to v6 ([#6](https://github.com/jbabin91/autocomplete-rs/pull/6))
- add agent skills and fix stale references across skill docs
- adapt skills to remove Gas Town and OpenSpec dependencies
- update stale references (Amazon Q → Kiro CLI, tui → dropdown)
- update dependencies
- set rust-version MSRV to 1.85 in Cargo.toml
- rename .claude/rules/tui.md to dropdown.md
- *(deps)* update actions/checkout action to v6 ([#5](https://github.com/jbabin91/autocomplete-rs/pull/5))
- *(deps)* update all non-major dependencies ([#4](https://github.com/jbabin91/autocomplete-rs/pull/4))
- remove Ratatui TUI in favor of planned inline ANSI dropdown
- *(deps)* pin dependencies ([#1](https://github.com/jbabin91/autocomplete-rs/pull/1))
- clean up README, fix docs, and add Renovate config
- optimize CI to 3 jobs with composite setup action
- update TUI rules for inline ANSI direction and clean up research docs
- add scoped Claude rules, taplo config, and fix prettier setup
- migrate from OpenSpec to beads issue tracker
- integrate beads hooks into hk hook manager
- integrate beads hooks into hk hook manager
- add beads session hooks to project Claude settings
- initialize beads issue tracker with sync branch
- add detailed terminal autocomplete landscape research
- reorganize project instructions and add Pkl IDE support
- add claude code settings
- Initial commit: Setup autocomplete-rs project foundation
