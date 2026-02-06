# Inline Dropdown Architecture

This document details the planned design for the inline completion dropdown.

**Status:** Not yet implemented. The old Ratatui-based TUI has been removed
(see [ADR-0006](../adr/0006-inline-ansi-dropdown.md)).

## Overview

The inline dropdown renders completions directly below the cursor using raw ANSI
escape codes. Unlike the previous Ratatui implementation (which used alternate
screen / full-screen takeover), this approach preserves the user's terminal
context — matching Fig's original UX.

## Design Principles

1. **Inline:** Renders below cursor, no alternate screen
2. **Fast:** <2ms render time target
3. **Clean:** Box-drawing characters, ANSI colors
4. **Adaptive:** Work across terminals (16/256/truecolor)
5. **Non-disruptive:** Terminal state fully restored on dismiss

## Rendering Approach

```text
1. Save cursor position (CSI s / CSI 7)
2. Move cursor below current line
3. Write dropdown box using box-drawing chars + ANSI colors
4. Handle keyboard input in raw mode
5. On dismiss: restore cursor, erase dropdown lines (CSI 2K)
6. Use synchronized output (DEC mode 2026) to prevent flicker
```

### Visual Layout

```text
$ git checkout |
┌─ Completions ──────────────────────────────┐
│   checkout       Switch branches or restore │
│   cherry         Apply changes from commits │
│ → cherry-pick    Apply changes from commits │  ← Selected
│   clean          Remove untracked files     │
│   clone          Clone a repository         │
└─────────────────────────────────────────────┘
```

## Key Bindings

- `Esc` → cancel (return None)
- `Enter` → select (return chosen suggestion)
- `Down` / `Ctrl+J` → next item (wraps)
- `Up` / `Ctrl+K` → previous item (wraps)
- Typing → filter suggestions (future)

## Constraints

- Max ~5-8 visible suggestions (scrollable)
- Smart positioning: render above cursor if near bottom of terminal
- Must work in all major terminals (iTerm2, Alacritty, Kitty, Ghostty, WezTerm)
- Render time: <2ms target

## Dependencies

- **crossterm** — raw mode, cursor control, keyboard input, ANSI output
- No Ratatui dependency

## Related Documents

- [Architecture Overview](overview.md) - System architecture
- [ADR-0006: Inline ANSI Dropdown](../adr/0006-inline-ansi-dropdown.md) - Design decision
- [ADR-0005: Ratatui for TUI](../adr/0005-ratatui-for-tui.md) - Superseded decision
