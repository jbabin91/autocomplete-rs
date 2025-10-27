# ADR-0005: Ratatui for Terminal UI

**Status:** Accepted **Date:** 2025-10-25 **Decision Makers:** Project Team
**Technical Story:** Choose TUI framework for rendering completion dropdown

## Context

We need to render a visually appealing dropdown completion menu in the terminal.
The UI must:

- Render quickly (<10ms)
- Support keyboard navigation (up/down arrows, Enter)
- Display with proper styling (colors, borders, highlighting)
- Handle dynamic resizing
- Work across terminals (iTerm2, Alacritty, Kitty, Wezterm, Ghostty, etc.)
- Support themes (Catppuccin variants)

### UI Requirements

**Layout:**

- Dropdown box with border
- List of suggestions (scrollable if >10 items)
- Highlighted selected item
- Description for each suggestion
- Icons/symbols for suggestion types (optional)

**Interaction:**

- Arrow keys navigate
- Enter selects
- Esc cancels
- Type-to-filter

**Performance:**

- Render <10ms (part of 20ms total budget)
- Smooth scrolling
- No flicker

**Visual:**

- Catppuccin theme support
- Respect terminal capabilities (truecolor/256/16)
- Clean, modern appearance

## Decision

We will use **Ratatui** (v0.29) as our TUI framework, with **Crossterm** (v0.29)
as the backend.

### Why Ratatui + Crossterm

**Ratatui** provides:

- High-level widget system (List, Block, Paragraph)
- Immediate-mode rendering (redraw each frame)
- Layout system (flexbox-like constraints)
- Style system (colors, modifiers, themes)

**Crossterm** provides:

- Low-level terminal control (escape codes)
- Event handling (keyboard, mouse)
- Terminal capability detection
- Cross-platform support

## Consequences

### Positive

**Developer Experience:**

- Declarative UI (describe what you want, not how to draw it)
- Widget composition (build complex UIs from simple parts)
- No manual cursor management
- Type-safe API (Rust compiler catches errors)

**Performance:**

- Immediate mode is fast for small UIs like dropdowns
- Efficient diffing (only send changed cells)
- Backend abstraction (can switch from Crossterm if needed)
- <5ms typical render time for completion dropdown

**Ecosystem:**

- Active maintenance (last release: Jan 2025)
- Large community (6.8k+ GitHub stars)
- Good documentation and examples
- Used by major projects (gitui, bottom, kdash)

**Features:**

- Built-in widgets (List, Block, Paragraph, Table)
- Flexible layout system
- Theme support (custom color schemes)
- Unicode and emoji support
- Mouse support (optional)

**Reliability:**

- Mature (fork of tui-rs with 4+ years development)
- Well-tested
- Stable API (semantic versioning)
- Cross-platform (macOS, Linux, Windows)

**Flexibility:**

- Multiple backends (Crossterm, Termion, Termwiz)
- Custom widgets easy to create
- Full control over rendering
- Can drop down to raw terminal if needed

### Negative

**Overhead:**

- ~2MB added to binary size
- Learning curve (immediate-mode paradigm)
- More complex than raw ANSI codes for simple UIs
- Event loop abstraction adds slight complexity

**Immediate Mode:**

- Must redraw entire UI each frame
- More CPU than retained mode (negligible for small UI)
- Not ideal for huge UIs (fine for dropdowns)

**Abstraction:**

- Extra layer between code and terminal
- Debugging requires understanding Ratatui internals
- Some terminal features not exposed
- Occasionally need to work around framework

## Alternatives Considered

### Option 1: Raw Crossterm (No Framework)

**How It Works:**

- Use Crossterm directly for terminal control
- Manually manage cursor, colors, clearing
- Hand-code all rendering logic

**Pros:**

- Minimal dependencies
- Full control
- Smallest binary size (~1MB smaller)
- Simplest conceptually

**Cons:**

- Must implement everything from scratch
- Error-prone (off-by-one errors, cursor bugs)
- No layout system (manual positioning)
- Tedious for anything complex
- Hard to maintain

**Why Not Chosen:** Too much work for too little benefit. Would spend weeks
reimplementing what Ratatui provides. Time better spent on features.

### Option 2: tui-rs (Ratatui's Predecessor)

**How It Works:**

- Original TUI framework
- Ratatui is a fork of tui-rs

**Pros:**

- Same API as Ratatui (mostly)
- Proven track record
- Stable

**Cons:**

- **Unmaintained** (last release 2021)
- No new features
- Security vulnerabilities not fixed
- Dependencies outdated
- Community moved to Ratatui

**Why Not Chosen:** Deprecated. Ratatui is the active continuation with same API
and active maintenance.

### Option 3: Cursive

**How It Works:**

- Retained-mode TUI framework
- Different paradigm (widgets persist)

**Pros:**

- Retained mode (less redrawing)
- Rich widget set
- Event-driven architecture
- Good for complex UIs

**Cons:**

- Heavier than Ratatui (~3MB)
- More complex for simple UIs
- Different mental model
- Less flexible layout system
- Smaller community than Ratatui

**Why Not Chosen:** Overkill for completion dropdown. Retained mode adds
complexity we don't need. Ratatui's immediate mode better for dynamic content.

### Option 4: Termion

**How It Works:**

- Low-level terminal library (like Crossterm)
- Can be Ratatui backend

**Pros:**

- Minimal and fast
- Pure Rust
- No unsafe code

**Cons:**

- Unix-only (no Windows support)
- Less feature-complete than Crossterm
- Smaller community
- Ratatui recommends Crossterm over Termion

**Why Not Chosen:** Crossterm is more universal and better maintained. Using
Termion would limit Windows support.

### Option 5: BubbleTea (Port from Go)

**How It Works:**

- Elm architecture TUI framework (from Go)
- Community working on Rust port

**Pros:**

- Elm architecture (messages, update, view)
- Proven design
- Popular in Go ecosystem

**Cons:**

- Rust port incomplete/experimental
- Small community
- Less mature than Ratatui
- Different paradigm (more complex)

**Why Not Chosen:** Too experimental. Ratatui is production-ready and
battle-tested.

### Option 6: Ncurses Bindings

**How It Works:**

- Rust bindings to C ncurses library
- Low-level terminal control

**Pros:**

- Mature (decades old)
- Widely available
- Well-documented

**Cons:**

- FFI overhead
- C dependencies (build complexity)
- Less Rusty API
- Harder to debug (C stack traces)
- ncurses quirks and portability issues

**Why Not Chosen:** Pure Rust solution preferred. Ncurses adds complexity
without benefits for our use case.

## Comparison Matrix

| Criterion      | Ratatui   | Raw Crossterm | tui-rs  | Cursive   | Termion      | Ncurses   |
| -------------- | --------- | ------------- | ------- | --------- | ------------ | --------- |
| Ease of Use    | High ✅   | Low ❌        | High ✅ | Medium ⚠️ | Low ❌       | Low ❌    |
| Performance    | <5ms ✅   | <2ms ✅       | <5ms ✅ | ~8ms ⚠️   | <2ms ✅      | ~10ms ⚠️  |
| Maintenance    | Active ✅ | Active ✅     | Dead ❌ | Active ✅ | Slow ⚠️      | Legacy ⚠️ |
| Binary Size    | +2MB ⚠️   | +1MB ✅       | +2MB ⚠️ | +3MB ❌   | +1MB ✅      | +2MB ⚠️   |
| Features       | Rich ✅   | Minimal ❌    | Rich ✅ | Rich ✅   | Minimal ❌   | Medium ⚠️ |
| Cross-platform | Yes ✅    | Yes ✅        | Yes ✅  | Yes ✅    | Unix only ❌ | Issues ⚠️ |
| Community      | Large ✅  | Large ✅      | Dead ❌ | Medium ⚠️ | Small ❌     | Legacy ⚠️ |

## Implementation Details

### Basic Structure

```rust
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Direction},
    widgets::{Block, Borders, List, ListItem, ListState},
    style::{Color, Modifier, Style},
    Terminal,
};

pub struct CompletionUI {
    suggestions: Vec<String>,
    selected: usize,
}

impl CompletionUI {
    pub fn render(&self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0)])
                .split(f.area());

            let items: Vec<ListItem> = self
                .suggestions
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let style = if i == self.selected {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };
                    ListItem::new(s.as_str()).style(style)
                })
                .collect();

            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("Completions"));

            f.render_widget(list, chunks[0]);
        })?;

        Ok(())
    }
}
```

### Theme Support

```rust
use ratatui::style::{Color, Style};

pub struct Theme {
    pub border: Color,
    pub selected_bg: Color,
    pub selected_fg: Color,
    pub text: Color,
    pub description: Color,
}

// Catppuccin Mocha
pub const CATPPUCCIN_MOCHA: Theme = Theme {
    border: Color::Rgb(180, 190, 254),      // Lavender
    selected_bg: Color::Rgb(137, 180, 250), // Blue
    selected_fg: Color::Rgb(30, 30, 46),    // Base
    text: Color::Rgb(205, 214, 244),        // Text
    description: Color::Rgb(186, 194, 222), // Subtext0
};
```

### Event Handling

```rust
use crossterm::event::{self, Event, KeyCode};

pub fn handle_input(&mut self) -> Result<Action> {
    if event::poll(Duration::from_millis(100))? {
        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Up => {
                    self.selected = self.selected.saturating_sub(1);
                    Ok(Action::Continue)
                }
                KeyCode::Down => {
                    self.selected = (self.selected + 1).min(self.suggestions.len() - 1);
                    Ok(Action::Continue)
                }
                KeyCode::Enter => Ok(Action::Select(self.selected)),
                KeyCode::Esc => Ok(Action::Cancel),
                _ => Ok(Action::Continue),
            }
        } else {
            Ok(Action::Continue)
        }
    } else {
        Ok(Action::Continue)
    }
}
```

### Performance Optimization

**Minimize Redraws:**

- Only redraw when state changes
- Use event polling (don't spin)
- Batch updates

**Efficient Rendering:**

- Render only visible items (viewport)
- Use stateful widgets (ListState)
- Cache computed layouts

```rust
impl CompletionUI {
    pub fn render_optimized(&self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        // Only redraw if dirty flag set
        if !self.dirty {
            return Ok(());
        }

        terminal.draw(|f| {
            // ... render logic
        })?;

        self.dirty = false;
        Ok(())
    }
}
```

## Future Considerations

**Advanced Features:**

- Mouse support (click to select)
- Fuzzy search highlighting
- Icons for suggestion types
- Preview pane for long descriptions
- Smooth scrolling animations

**Alternative Backends:**

- Try Termwiz for better Windows support if needed
- Consider GPU-accelerated terminals (Alacritty, Kitty)

**Accessibility:**

- Screen reader hints
- High-contrast mode
- Configurable key bindings

## References

- [Ratatui Documentation](https://ratatui.rs/)
- [Ratatui GitHub](https://github.com/ratatui-org/ratatui)
- [Crossterm Documentation](https://docs.rs/crossterm/)
- [Immediate Mode GUI](https://en.wikipedia.org/wiki/Immediate_mode_GUI)
- Projects using Ratatui: [gitui](https://github.com/extrawurst/gitui),
  [bottom](https://github.com/ClementTsang/bottom)

## Review Notes

Decision prioritizes:

1. **Developer velocity** - Rich widget system saves weeks of development
2. **Maintainability** - Active community and continued development
3. **Reliability** - Battle-tested in production projects
4. **Flexibility** - Can start simple, add features incrementally

Trade-offs:

- Accept ~2MB binary increase for significant DX improvement
- Accept framework abstraction for productivity gain
- Accept immediate-mode overhead (negligible for small UI)

Ratatui is the clear choice for TUI in Rust today. It's what tui-rs users
migrated to, it has momentum, and it provides exactly what we need without
over-engineering.

This decision would be revisited only if:

- Ratatui becomes unmaintained (unlikely given community size)
- We need extreme binary size optimization (can drop to raw Crossterm)
- We expand to complex multi-pane UIs (might consider Cursive)

For autocomplete dropdown, Ratatui is perfect.
