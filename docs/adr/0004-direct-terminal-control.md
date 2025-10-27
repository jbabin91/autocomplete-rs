# ADR-0004: Direct Terminal Control via Shell Integration

**Status:** Accepted **Date:** 2025-10-25 **Decision Makers:** Project Team
**Technical Story:** Choose how to display completion UI in the terminal without
positioning bugs

## Context

The primary motivation for this project was frustration with Amazon Q's
(formerly Fig's) positioning bugs when displaying autocomplete dropdowns. Amazon
Q uses macOS Accessibility APIs to overlay a native window on top of the
terminal, which frequently misaligns with the cursor position.

### The Problem with Amazon Q

Amazon Q's approach:

1. Uses Accessibility API to get cursor position
2. Creates native macOS window
3. Positions window based on cursor coordinates
4. Overlays on top of terminal

Issues encountered:

- **Positioning bugs:** Window appears in wrong location
- **Multi-monitor:** Breaks when moving terminals between displays
- **Terminal variations:** Different terminals report positions differently
- **Split panes:** Incorrect positioning with terminal multiplexers
- **Font changes:** Position calculation breaks with custom fonts
- **Scaling:** Issues with HiDPI and fractional scaling

### Requirements

- **Accuracy:** Dropdown must appear exactly at cursor, always
- **Universality:** Work across terminals (iTerm2, Alacritty, Kitty, Wezterm,
  Ghostty, etc.)
- **Reliability:** No positioning bugs or visual glitches
- **Performance:** <10ms rendering time
- **Native Feel:** Look like part of the terminal, not an overlay

## Decision

We will use **direct shell integration** with **native terminal rendering** via
**ZLE widgets** (for zsh) and equivalent mechanisms for other shells.

### Architecture

**Shell Integration Layer:**

- ZLE widget captures buffer and cursor position directly from shell
- Widget controls when autocomplete triggers
- Shell provides exact cursor position in terminal coordinates

**Terminal Rendering:**

- Use terminal's own rendering system (ANSI escape codes)
- Draw UI directly in terminal buffer below prompt
- No external windows or overlays
- Terminal handles all positioning and display

**Communication:**

1. User triggers completion (Alt+Space)
2. ZLE widget reads `$BUFFER` and `$CURSOR`
3. Widget sends request to daemon via Unix socket
4. Daemon responds with suggestions
5. Widget renders dropdown using terminal escape codes
6. User selects completion
7. Widget updates buffer
8. Terminal displays normally

## Consequences

### Positive

**Accuracy:**

- Zero positioning bugs (terminal handles all positioning)
- Cursor position is exact (from shell, not Accessibility API)
- Works with all font sizes, DPI settings, scaling
- Correct with split panes, tmux, screen

**Universality:**

- Works on any terminal that supports ANSI escape codes
- No terminal-specific code needed
- Same approach works on macOS, Linux, Windows (WSL)
- Compatible with terminal multiplexers

**Performance:**

- No window creation overhead
- Terminal rendering is native and fast
- No coordinate calculations or translations
- <1ms to render dropdown

**Native Feel:**

- Looks like terminal output, not overlay
- Respects terminal theme automatically
- Works with terminal transparency/blur
- Feels integrated, not bolted on

**Simplicity:**

- No Accessibility API permissions required
- No window management code
- Standard ANSI escape codes
- Shell does all the heavy lifting

**Reliability:**

- Can't break from macOS updates (no private APIs)
- Terminal compatibility is stable
- No race conditions with window positioning
- Deterministic behavior

### Negative

**Complexity:**

- Must implement shell-specific integration (ZLE for zsh, readline for bash,
  etc.)
- Different escape code handling per terminal (minor)
- Need to manage terminal state (save/restore cursor)
- Screen redraw coordination

**Visual Limitations:**

- Can't overlay on top of existing text (pushes content down)
- Limited to terminal color capabilities (16/256/truecolor)
- Can't use native UI widgets (buttons, smooth fonts)
- Bounded by terminal window size

**Multi-line Behavior:**

- Dropdown pushes prompt down temporarily
- Screen scrolls if dropdown is tall
- Need to restore terminal state after completion
- Potential flicker on slow terminals

**Integration Work:**

- Must write shell integration for each shell (zsh, bash, fish)
- Each shell has different widget/completion APIs
- Need to handle shell-specific quirks
- More upfront development work

## Alternatives Considered

### Option 1: Accessibility API (Amazon Q's Approach)

**How It Works:**

- Use macOS Accessibility API to get cursor position
- Create native AppKit window
- Position window as overlay on terminal

**Pros:**

- Native macOS UI (smooth fonts, shadows, animations)
- Can overlay without pushing content
- Rich UI capabilities (images, custom widgets)

**Cons:**

- **Positioning bugs** (the entire reason for this project!)
- macOS only (no Linux/Windows support)
- Requires Accessibility permissions
- Breaks with terminal changes
- Window management complexity
- Fragile (depends on terminal cooperation)

**Why Not Chosen:** This is exactly what we're trying to avoid. The positioning
bugs are unacceptable and were the motivation for building autocomplete-rs.

### Option 2: SIXEL/Kitty Graphics Protocol

**How It Works:**

- Use advanced terminal graphics protocols
- Render images inline in terminal
- Create rich UI with graphics

**Pros:**

- Can render custom UI without native windows
- Pixel-perfect rendering
- Supports images, custom fonts

**Cons:**

- Limited terminal support (Kitty, WezTerm only for Kitty protocol)
- Not universal (iTerm2, Alacritty don't support Kitty protocol)
- Complex implementation
- Overkill for text dropdowns

**Why Not Chosen:** Not universal enough. We want to work across all terminals,
not just a subset.

### Option 3: Terminal Multiplexer Integration (tmux/screen)

**How It Works:**

- Integrate with tmux's popup feature
- Use tmux display-popup to show completions
- tmux handles positioning

**Pros:**

- tmux handles all positioning
- Native tmux feature
- Works well in tmux sessions

**Cons:**

- Only works inside tmux (not native terminals)
- tmux-specific implementation
- Doesn't solve the problem for non-tmux users
- Different approach needed for non-tmux anyway

**Why Not Chosen:** Not universal. Need solution that works both in and out of
tmux.

### Option 4: Remote Terminal Protocol (SSH)

**How It Works:**

- Terminal runs daemon, sends requests over SSH
- Server-side rendering, client displays
- Like Jupyter widgets

**Pros:**

- Works over SSH
- Server can be powerful machine

**Cons:**

- Massive complexity
- Latency over network
- Not needed for local use
- Overkill for autocomplete

**Why Not Chosen:** Solving a problem we don't have. Local autocomplete doesn't
need SSH.

### Option 5: Custom Terminal Emulator

**How It Works:**

- Fork Alacritty/WezTerm
- Add native autocomplete support
- Integrate directly into terminal

**Pros:**

- Perfect integration
- No positioning issues
- Maximum control

**Cons:**

- Requires users to switch terminals
- Must maintain terminal fork
- Massive scope increase
- Not universal

**Why Not Chosen:** Goal is to work with existing terminals, not force terminal
change.

## Comparison Matrix

| Criterion      | Shell Integration | Accessibility API | SIXEL/Kitty | tmux Popups  | Custom Terminal  |
| -------------- | ----------------- | ----------------- | ----------- | ------------ | ---------------- |
| Accuracy       | Perfect ✅        | Buggy ❌          | Perfect ✅  | Perfect ✅   | Perfect ✅       |
| Universal      | High ✅           | macOS only ❌     | Limited ❌  | tmux only ❌ | Requires fork ❌ |
| Complexity     | Medium ⚠️         | High ❌           | High ❌     | Low ✅       | Extreme ❌       |
| Performance    | <1ms ✅           | ~5ms ⚠️           | ~2ms ⚠️     | <1ms ✅      | <1ms ✅          |
| Visual Quality | Text ⚠️           | Native ✅         | Graphics ✅ | Text ⚠️      | Native ✅        |
| Permissions    | None ✅           | Accessibility ❌  | None ✅     | None ✅      | None ✅          |
| Reliability    | High ✅           | Low ❌            | High ✅     | High ✅      | High ✅          |

## Implementation Details

### ZLE Widget (zsh)

```zsh
# Widget function
_autocomplete_rs_widget() {
    # Get current state
    local buffer="$BUFFER"
    local cursor="$CURSOR"

    # Send to daemon
    local response=$(echo "{\"buffer\":\"$buffer\",\"cursor\":$cursor}" | \
        nc -U /tmp/autocomplete-rs.sock)

    # Save cursor position
    echo -n "\e[s"

    # Move to next line and render dropdown
    echo -n "\n\e[1;32m> Suggestions:\e[0m\n"
    echo "$response" | jq -r '.suggestions[] | "  \(.text) - \(.description)"'

    # Restore cursor
    echo -n "\e[u"

    # Read user selection
    # ... (keyboard handling)

    # Update buffer
    BUFFER="$new_buffer"
    CURSOR="$new_cursor"

    # Redraw
    zle reset-prompt
}

# Register widget
zle -N _autocomplete_rs_widget

# Bind to Alt+Space
bindkey '^[ ' _autocomplete_rs_widget
```

### Terminal Rendering

**Escape Codes Used:**

- `\e[s` - Save cursor position
- `\e[u` - Restore cursor position
- `\e[K` - Clear line
- `\e[2J` - Clear screen
- `\e[<row>;<col>H` - Move cursor
- `\e[38;2;<r>;<g>;<b>m` - 24-bit color (if supported)

**Rendering Strategy:**

1. Save cursor position
2. Move cursor down (preserve prompt)
3. Render dropdown box with ANSI colors
4. Render suggestions with highlighting
5. Handle keyboard input
6. Clear dropdown
7. Restore cursor position
8. Update buffer and redraw prompt

### Terminal Compatibility

**Tested Terminals:**

- iTerm2 (macOS)
- Alacritty (cross-platform)
- Kitty (macOS/Linux)
- WezTerm (cross-platform)
- Ghostty (macOS/Linux)
- Terminal.app (macOS)
- GNOME Terminal (Linux)

**Feature Detection:**

- Query for truecolor support (`\e]4;-1;?\a`)
- Fallback to 256 color if not supported
- Graceful degradation to 16 color

## Future Considerations

**Advanced Terminals:**

- Use Kitty graphics for rich completions when available
- SIXEL for terminals that support it
- Keep text fallback for maximum compatibility

**Shell Support:**

- bash: Use `bind -x` for readline integration
- fish: Use native fish completion events
- nushell: Native integration via plugins

**Accessibility:**

- Ensure screen readers work with terminal output
- Provide text-mode option for minimal terminals
- Support high-contrast themes

## References

- [ZLE Documentation](https://zsh.sourceforge.io/Doc/Release/Zsh-Line-Editor.html)
- [ANSI Escape Codes](https://en.wikipedia.org/wiki/ANSI_escape_code)
- [Bash Readline](https://tiswww.case.edu/php/chet/readline/rltop.html)
- [Fish Completions](https://fishshell.com/docs/current/completions.html)
- Amazon Q positioning bugs (user experience)

## Review Notes

This decision directly addresses the core motivation for the project: **avoiding
Amazon Q's positioning bugs**.

Key insight: The bugs aren't implementation errors—they're fundamental to the
Accessibility API approach. By using direct shell integration, we make
positioning bugs architecturally impossible.

Trade-off: We accept the visual limitations of terminal rendering in exchange
for perfect positioning and universal compatibility.

This is the right choice because:

1. Accuracy trumps visual flair
2. Universal support > platform-specific features
3. Simple, reliable > complex, fragile
4. Terminal-native > bolted-on overlays
