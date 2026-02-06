# ADR-0006: Inline ANSI Dropdown

**Status:** Accepted **Date:** 2025-12-01 **Decision Makers:** Project Team
**Technical Story:** Replace Ratatui TUI with inline ANSI rendering for completion dropdown
**Supersedes:** [ADR-0005](0005-ratatui-for-tui.md) (Ratatui for TUI)

## Context

ADR-0005 chose Ratatui for rendering the completion dropdown. After prototyping, we found that Ratatui's alternate screen approach (full-screen takeover) doesn't match the target UX: an inline dropdown that appears below the cursor without disrupting the terminal context (like Fig's UX).

The key problems with Ratatui for this use case:

- `EnterAlternateScreen` takes over the entire terminal — loses the user's command context
- Alternate screen mode is designed for full TUI apps (htop, vim), not small inline widgets
- The dropdown should render _in place_, below the prompt, like native shell completions
- Ratatui's widget system and layout engine are overkill for a simple dropdown box

## Decision

Use **raw ANSI escape codes via crossterm** to render the completion dropdown inline below the cursor. No Ratatui dependency.

### Rendering Approach

1. Save cursor position (`CSI s` or `CSI 7`)
2. Move cursor below the current line
3. Write dropdown using box-drawing characters (`┌─┐│└─┘`) and ANSI colors
4. Handle keyboard input in raw mode (arrows, Enter, Esc)
5. On dismiss: restore cursor, erase dropdown lines (`CSI 2K` per line)
6. Use synchronized output (DEC mode 2026) to prevent flicker

## Consequences

### Positive

- **Correct UX** — dropdown appears inline, preserving terminal context
- **Smaller binary** — removes ~2MB from Ratatui + crossterm widget overhead
- **Simpler dependency tree** — fewer crates to maintain and update
- **Full control** — no framework abstractions between code and terminal
- **Faster rendering** — direct ANSI writes, no buffer diffing layer

### Negative

- **More manual work** — must handle cursor math, line clearing, box drawing manually
- **No widget system** — layout calculations done by hand
- **Terminal quirks** — must handle edge cases across terminals ourselves

## Alternatives Considered

Keeping Ratatui but avoiding alternate screen was considered, but Ratatui's rendering model (immediate-mode full-frame redraw with buffer diffing) adds unnecessary complexity for a small inline widget.
