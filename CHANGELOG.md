# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1](https://github.com/jbabin91/autocomplete-rs/compare/v0.1.0...v0.1.1) - 2026-02-06

### Other

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
