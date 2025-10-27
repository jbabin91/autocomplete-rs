# Add Multi-Shell Support (Phase 4)

**Priority:** 4 **Phase:** 4 (Universal) **Dependencies:** `add-theme-support`
**Blocks:** None (final phase)

## Why

To truly be universal, autocomplete-rs must work with all popular shells, not
just zsh. Adding bash and fish support expands the user base significantly.

## What Changes

- Implement bash integration using readline
- Implement fish integration using native fish completion system
- Create abstraction layer for shell-specific behavior
- Add shell detection and automatic integration installation
- Support multiple shells on same system

## Impact

- Affected specs:
  - `shell-integration-bash` (new capability)
  - `shell-integration-fish` (new capability)
  - `daemon` (modified - shell-aware responses if needed)
- Affected code:
  - `shell-integration/bash.sh` - bash readline integration
  - `shell-integration/fish.fish` - fish completion integration
  - `src/main.rs` - install command for bash/fish
  - `src/daemon/mod.rs` - potentially shell-specific handling
- Dependencies: None (shell-specific scripts only)
- Migration: Existing zsh users unaffected

## Design Decisions

- **Shell-agnostic daemon**: Daemon doesn't need to know shell type in most
  cases
- **Native integration**: Use each shell's native completion system (readline
  for bash, fish completions for fish)
- **Shared protocol**: All shells use same Unix socket protocol
- **Shell detection**: Auto-detect shell for installation
