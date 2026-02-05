---
paths:
  - 'src/tui/**'
---

# TUI Development Rules

## Architecture

- Ratatui immediate-mode rendering with Crossterm backend
- `CompletionUI` struct holds suggestions + selected index
- Event loop reads `crossterm::event::Event::Key` for navigation
- Navigation wraps around (last → first, first → last)

## Current Implementation (Alternate Screen)

- Uses `EnterAlternateScreen` / `LeaveAlternateScreen` (full-screen mode)
- Raw mode enabled during TUI, restored on exit (including on error)
- Terminal cleanup MUST happen in all exit paths (success, cancel, error)

## Future Direction

- Replace alternate screen with inline ANSI rendering (ADR-0004)
- Dropdown should render below the cursor line, pushing content down
- This is a significant architectural change — don't do it incrementally within the current alternate-screen approach

## Styling

- Selected item: Yellow + Bold
- Unselected text: White
- Description text: Gray (dimmed when unselected, Yellow when selected)
- Border: Cyan with "Completions" title
- Uses Ratatui `List`, `Block`, `ListItem` widgets

## Key Bindings

- `Esc` → cancel (return None)
- `Enter` → select (return Some(Suggestion))
- `Down` → next item (wraps)
- `Up` → previous item (wraps)

## Performance Budget

- TUI render: <10ms
- Must handle 100+ suggestions smoothly
