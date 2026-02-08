# ADR-0006: Inline ANSI Dropdown

**Status:** Superseded by [ADR-0008](0008-native-overlay-dropdown.md) **Date:** 2025-12-01 **Decision Makers:** Project Team
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

## Addendum (2026-02-08): Native GUI Overlay Research

Subsequent research revealed that Fig.io's dropdown was **not** in-band terminal
text — it was a native macOS overlay window (`NSPanel` with `WKWebView`) floating
above the terminal, positioned using the macOS Accessibility API and a custom
InputMethodKit plugin for cursor tracking. See
[dropdown-rendering-architecture.md](../research/dropdown-rendering-architecture.md)
for the full analysis.

The native overlay approach was evaluated and **intentionally rejected** for
autocomplete-rs based on:

1. **Portability** — Accessibility API + IME are macOS-only. Linux/Wayland has no
   equivalent for arbitrary window positioning. SSH/tmux/containers are unsupported.
2. **Operational complexity** — Fig's overlay suffered persistent cursor
   misalignment, memory leaks (160GB), permission friction, and per-terminal
   quirks that ultimately contributed to the project's end.
3. **Maintenance burden** — Cursor tracking requires platform-specific code for
   each OS and fallback strategies for each terminal emulator.
4. **Every active tool in this space** (inshellisense, Atuin, Reedline) uses
   in-band ANSI rendering, validating this approach.

**Decision reaffirmed:** In-band ANSI rendering via crossterm is the correct
choice. A native overlay renderer may be added as an optional progressive
enhancement in the future, but the ANSI renderer is the universal baseline.

The rendering interface should be trait-based to support this future extension:

```rust
trait CompletionRenderer {
    fn show(&mut self, suggestions: &[Suggestion], cursor_col: u16) -> Result<()>;
    fn hide(&mut self) -> Result<()>;
    fn navigate(&mut self, direction: Direction) -> Result<()>;
    fn selected(&self) -> Option<&Suggestion>;
}
```
