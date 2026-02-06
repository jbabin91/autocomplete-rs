---
paths:
  - 'src/tui/**'
---

# Inline Completion Dropdown Rules

## Target Architecture

The completion dropdown renders **inline** in the shell using raw ANSI escape codes via crossterm — NOT alternate screen, NOT Ratatui. It appears below the cursor without disrupting terminal context (like Fig's UX, not fzf's).

Implementation approach:

1. Save cursor position
2. Move cursor below current line
3. Write dropdown box using box-drawing characters and ANSI colors
4. Handle keyboard input (arrows, enter, esc, typing to filter)
5. On dismiss: restore cursor, erase dropdown lines
6. Use synchronized output (DEC mode 2026) to prevent flicker

## Current State

The inline dropdown is **not yet implemented**. The old Ratatui-based TUI has been removed. Currently `complete_command` outputs raw JSON to stdout.

## Key Bindings

- `Esc` → cancel (return None)
- `Enter` → select (return Some(Suggestion))
- `Down` → next item (wraps)
- `Up` → previous item (wraps)

## Constraints

- Max ~5-8 visible suggestions (scrollable)
- Smart positioning: render above cursor if near bottom of terminal
- Must work in all major terminals (iTerm2, Alacritty, Kitty, Ghostty, WezTerm, VS Code terminal)
- Render time: <2ms target
