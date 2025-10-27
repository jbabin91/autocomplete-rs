# Add Theme Support (Phase 3)

**Priority:** 3 **Phase:** 3 (Polish) **Dependencies:** `add-fig-spec-parser`
**Blocks:** `add-multi-shell-support`

## Why

Users should be able to customize the appearance of the autocomplete dropdown to
match their terminal theme. Starting with Catppuccin (highly requested) provides
a great user experience and demonstrates extensibility.

## What Changes

- Create theming system with configurable colors
- Implement Catppuccin theme variants (Mocha, Macchiato, Frappe, Latte)
- Add theme configuration file support
- Allow runtime theme switching
- Provide default theme that works universally

## Impact

- Affected specs:
  - `theming` (new capability)
  - `tui` (modified - use theme colors)
- Affected code:
  - `src/theme/mod.rs` - theme definitions and loading
  - `src/theme/catppuccin.rs` - Catppuccin variants
  - `src/tui/mod.rs` - apply theme colors
  - `src/config/mod.rs` - theme configuration
- Dependencies: None (use existing Ratatui color system)
- Migration: Existing users get default theme automatically

## Design Decisions

- **Catppuccin first**: Popular theme with existing terminal support
- **Config file**: `~/.config/autocomplete-rs/theme.toml` for persistence
- **Runtime switching**: Allow theme changes without restart
- **Fallback**: Default theme works on all terminals (no deps on 256-color)
