# TUI Architecture

This document details the design and implementation of the terminal user
interface.

## Overview

The TUI is responsible for:

- Rendering completion dropdown in terminal
- Handling keyboard navigation
- Applying themes (Catppuccin)
- Managing UI state
- Rendering in <10ms

## Design Principles

1. **Fast:** <10ms render time, immediate response to input
2. **Clean:** Clear visual hierarchy, easy to scan
3. **Adaptive:** Work across terminals (16/256/truecolor)
4. **Accessible:** Clear without relying solely on color
5. **Customizable:** Theme support for user preferences

## Architecture

### Component Diagram

```text
┌─────────────────────────────────────────────────────────────┐
│                    TUI (src/tui/)                            │
│                                                               │
│  ┌────────────────────────────────────────────────────────┐ │
│  │  mod.rs - Main TUI logic                              │ │
│  │  pub struct CompletionUI                              │ │
│  │  pub fn render()                                      │ │
│  │  pub fn handle_input()                                │ │
│  └────────────────────────────────────────────────────────┘ │
│             │                │               │               │
│             ▼                ▼               ▼               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │  Widgets     │  │   Theme      │  │   Layout     │     │
│  │ widgets.rs   │  │  theme.rs    │  │  layout.rs   │     │
│  │              │  │              │  │              │     │
│  │ Custom       │  │ Catppuccin   │  │ Positioning  │     │
│  │ components   │  │ colors       │  │ & sizing     │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
│                                                               │
│  ┌────────────────────────────────────────────────────────┐ │
│  │         Ratatui + Crossterm                           │ │
│  │  - Terminal abstraction                               │ │
│  │  - Widget rendering                                   │ │
│  │  - Event handling                                     │ │
│  └────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### State Management

```rust
pub struct CompletionUI {
    /// Suggestions to display
    suggestions: Vec<Suggestion>,

    /// Currently selected index
    selected: usize,

    /// Scroll offset for long lists
    scroll: usize,

    /// Theme colors
    theme: Theme,

    /// Terminal size
    terminal_size: (u16, u16), // (width, height)

    /// Maximum visible items
    max_visible: usize,
}

pub struct Suggestion {
    /// Text to insert
    pub text: String,

    /// Description (optional)
    pub description: Option<String>,

    /// Type (command, option, argument)
    pub suggestion_type: SuggestionType,

    /// Match score (for sorting, future)
    pub score: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuggestionType {
    Command,
    Subcommand,
    Option,
    Argument,
    File,
    Directory,
}
```

## Rendering

### Rendering Pipeline

```text
1. Clear previous UI
   └─▶ Save cursor position
       └─▶ Clear lines below cursor

2. Calculate layout
   ├─▶ Measure terminal size
   ├─▶ Calculate dropdown dimensions
   └─▶ Determine scroll position

3. Build widgets
   ├─▶ Create border block
   ├─▶ Create list items
   └─▶ Apply theme colors

4. Render to terminal
   ├─▶ Ratatui draws to buffer
   └─▶ Crossterm writes ANSI codes

5. Restore state
   └─▶ Position cursor for input
```

### Implementation

```rust
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Terminal,
};

impl CompletionUI {
    pub fn render(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        terminal.draw(|f| {
            // 1. Get layout area
            let area = self.calculate_area(f.area());

            // 2. Create border block
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(self.theme.border))
                .title("Completions");

            // 3. Calculate inner area (inside borders)
            let inner = block.inner(area);

            // 4. Create list items
            let items: Vec<ListItem> = self
                .visible_suggestions()
                .map(|(idx, suggestion)| {
                    let is_selected = idx == self.selected;
                    self.create_list_item(suggestion, is_selected)
                })
                .collect();

            // 5. Create list widget
            let list = List::new(items)
                .block(block)
                .highlight_style(
                    Style::default()
                        .bg(self.theme.selected_bg)
                        .fg(self.theme.selected_fg)
                        .add_modifier(Modifier::BOLD)
                );

            // 6. Render with state
            let mut state = ListState::default();
            state.select(Some(self.selected - self.scroll));

            f.render_stateful_widget(list, area, &mut state);
        })?;

        Ok(())
    }

    fn create_list_item(&self, suggestion: &Suggestion, is_selected: bool) -> ListItem {
        let style = if is_selected {
            Style::default()
                .bg(self.theme.selected_bg)
                .fg(self.theme.selected_fg)
        } else {
            Style::default().fg(self.theme.text)
        };

        // Build line with icon, text, and description
        let mut spans = vec![];

        // Icon (if enabled)
        if self.show_icons {
            let icon = self.get_icon(suggestion.suggestion_type);
            spans.push(Span::styled(format!("{} ", icon), style));
        }

        // Text
        spans.push(Span::styled(&suggestion.text, style));

        // Description
        if let Some(desc) = &suggestion.description {
            spans.push(Span::raw("  "));
            spans.push(Span::styled(
                desc,
                Style::default().fg(self.theme.description)
            ));
        }

        ListItem::new(Line::from(spans))
    }

    fn calculate_area(&self, full_area: Rect) -> Rect {
        // Position dropdown below cursor
        // Leave 1 line for prompt

        let max_height = self.suggestions.len().min(self.max_visible) as u16 + 2; // +2 for borders
        let width = self.calculate_width();

        Rect {
            x: 0,
            y: 1, // Below prompt
            width: width.min(full_area.width),
            height: max_height.min(full_area.height - 1),
        }
    }

    fn calculate_width(&self) -> u16 {
        // Find longest suggestion
        let max_text_len = self.suggestions
            .iter()
            .map(|s| {
                let text_len = s.text.len();
                let desc_len = s.description.as_ref().map(|d| d.len() + 2).unwrap_or(0);
                text_len + desc_len
            })
            .max()
            .unwrap_or(40);

        (max_text_len as u16 + 4).min(80) // +4 for icon and padding, max 80
    }
}
```

### Visual Layout

```text
┌─ Completions ──────────────────────────────┐
│  checkout       Switch branches or restore │
│  cherry         Apply changes from commits │
│ → cherry-pick   Apply changes from commits │  ← Selected (highlighted)
│  clean          Remove untracked files     │
│  clone          Clone a repository         │
│  commit         Record changes             │
│  config         Get/set configuration      │
│                                             │
│                                   [3/42]   │  ← Scroll indicator
└─────────────────────────────────────────────┘
```

## Keyboard Handling

### Event Loop

```rust
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

pub fn handle_input(&mut self) -> Result<Action> {
    // Poll with timeout (non-blocking)
    if !event::poll(Duration::from_millis(100))? {
        return Ok(Action::Continue);
    }

    // Read event
    let event = event::read()?;

    match event {
        Event::Key(key_event) => self.handle_key(key_event),
        Event::Resize(width, height) => {
            self.terminal_size = (width, height);
            Ok(Action::Redraw)
        }
        _ => Ok(Action::Continue),
    }
}

fn handle_key(&mut self, key: KeyEvent) -> Result<Action> {
    match (key.code, key.modifiers) {
        // Navigation
        (KeyCode::Up, _) | (KeyCode::Char('k'), KeyModifiers::CONTROL) => {
            self.move_selection(-1);
            Ok(Action::Redraw)
        }

        (KeyCode::Down, _) | (KeyCode::Char('j'), KeyModifiers::CONTROL) => {
            self.move_selection(1);
            Ok(Action::Redraw)
        }

        (KeyCode::PageUp, _) => {
            self.move_selection(-(self.max_visible as i32));
            Ok(Action::Redraw)
        }

        (KeyCode::PageDown, _) => {
            self.move_selection(self.max_visible as i32);
            Ok(Action::Redraw)
        }

        (KeyCode::Home, _) => {
            self.selected = 0;
            self.scroll = 0;
            Ok(Action::Redraw)
        }

        (KeyCode::End, _) => {
            self.selected = self.suggestions.len().saturating_sub(1);
            self.update_scroll();
            Ok(Action::Redraw)
        }

        // Selection
        (KeyCode::Enter, _) => {
            Ok(Action::Select(self.selected))
        }

        // Cancel
        (KeyCode::Esc, _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
            Ok(Action::Cancel)
        }

        // Type-to-filter (future)
        (KeyCode::Char(c), KeyModifiers::NONE) => {
            self.filter_suggestions(c);
            Ok(Action::Redraw)
        }

        // Tab for next match (future)
        (KeyCode::Tab, KeyModifiers::NONE) => {
            self.next_match();
            Ok(Action::Redraw)
        }

        _ => Ok(Action::Continue),
    }
}
```

### Actions

```rust
#[derive(Debug, Clone)]
pub enum Action {
    /// Continue showing UI
    Continue,

    /// Redraw UI
    Redraw,

    /// User selected suggestion
    Select(usize),

    /// User cancelled
    Cancel,
}
```

### Selection Movement

```rust
impl CompletionUI {
    fn move_selection(&mut self, delta: i32) {
        let new_idx = (self.selected as i32 + delta)
            .max(0)
            .min((self.suggestions.len() - 1) as i32) as usize;

        self.selected = new_idx;
        self.update_scroll();
    }

    fn update_scroll(&mut self) {
        // Ensure selected item is visible
        if self.selected < self.scroll {
            // Scrolled above view
            self.scroll = self.selected;
        } else if self.selected >= self.scroll + self.max_visible {
            // Scrolled below view
            self.scroll = self.selected - self.max_visible + 1;
        }
    }

    fn visible_suggestions(&self) -> impl Iterator<Item = (usize, &Suggestion)> {
        self.suggestions
            .iter()
            .enumerate()
            .skip(self.scroll)
            .take(self.max_visible)
    }
}
```

## Themes

### Catppuccin Themes

```rust
#[derive(Debug, Clone)]
pub struct Theme {
    /// Border color
    pub border: Color,

    /// Selected item background
    pub selected_bg: Color,

    /// Selected item foreground
    pub selected_fg: Color,

    /// Normal text
    pub text: Color,

    /// Description text
    pub description: Color,

    /// Error text
    pub error: Color,

    /// Warning text
    pub warning: Color,
}

// Catppuccin Mocha (default)
pub const MOCHA: Theme = Theme {
    border: Color::Rgb(180, 190, 254),      // Lavender
    selected_bg: Color::Rgb(137, 180, 250), // Blue
    selected_fg: Color::Rgb(30, 30, 46),    // Base
    text: Color::Rgb(205, 214, 244),        // Text
    description: Color::Rgb(186, 194, 222), // Subtext0
    error: Color::Rgb(243, 139, 168),       // Red
    warning: Color::Rgb(249, 226, 175),     // Yellow
};

// Catppuccin Macchiato
pub const MACCHIATO: Theme = Theme {
    border: Color::Rgb(183, 189, 248),
    selected_bg: Color::Rgb(138, 173, 244),
    selected_fg: Color::Rgb(36, 39, 58),
    text: Color::Rgb(202, 211, 245),
    description: Color::Rgb(184, 192, 224),
    error: Color::Rgb(237, 135, 150),
    warning: Color::Rgb(238, 212, 159),
};

// Catppuccin Frappe
pub const FRAPPE: Theme = Theme {
    border: Color::Rgb(186, 187, 241),
    selected_bg: Color::Rgb(140, 170, 238),
    selected_fg: Color::Rgb(48, 52, 70),
    text: Color::Rgb(198, 208, 245),
    description: Color::Rgb(181, 191, 226),
    error: Color::Rgb(231, 130, 132),
    warning: Color::Rgb(229, 200, 144),
};

// Catppuccin Latte (light)
pub const LATTE: Theme = Theme {
    border: Color::Rgb(114, 135, 253),
    selected_bg: Color::Rgb(30, 102, 245),
    selected_fg: Color::Rgb(239, 241, 245),
    text: Color::Rgb(76, 79, 105),
    description: Color::Rgb(108, 111, 133),
    error: Color::Rgb(210, 15, 57),
    warning: Color::Rgb(223, 142, 29),
};
```

### Theme Selection

```rust
impl Theme {
    pub fn from_name(name: &str) -> Option<Theme> {
        match name {
            "mocha" => Some(MOCHA),
            "macchiato" => Some(MACCHIATO),
            "frappe" => Some(FRAPPE),
            "latte" => Some(LATTE),
            _ => None,
        }
    }

    pub fn detect_terminal_theme() -> Theme {
        // Check terminal background
        // If light, use LATTE, else MOCHA
        if is_light_background() {
            LATTE
        } else {
            MOCHA
        }
    }
}
```

### Color Adaptation

```rust
pub fn adapt_color(color: Color, terminal_caps: TerminalCapabilities) -> Color {
    match terminal_caps {
        TerminalCapabilities::TrueColor => color,

        TerminalCapabilities::Color256 => {
            // Convert RGB to nearest 256-color
            match color {
                Color::Rgb(r, g, b) => {
                    Color::Indexed(rgb_to_256(r, g, b))
                }
                _ => color,
            }
        }

        TerminalCapabilities::Color16 => {
            // Map to basic ANSI colors
            match color {
                Color::Rgb(r, g, b) => {
                    Color::from_ansi(rgb_to_ansi(r, g, b))
                }
                _ => color,
            }
        }
    }
}
```

## Icons & Symbols

### Suggestion Type Icons

```rust
pub fn get_icon(suggestion_type: SuggestionType) -> &'static str {
    match suggestion_type {
        SuggestionType::Command => "⚡",      // Lightning
        SuggestionType::Subcommand => "▶",   // Triangle
        SuggestionType::Option => "⚙",       // Gear
        SuggestionType::Argument => "📝",    // Memo
        SuggestionType::File => "📄",        // File
        SuggestionType::Directory => "📁",   // Folder
    }
}

// ASCII fallback for terminals without Unicode
pub fn get_icon_ascii(suggestion_type: SuggestionType) -> &'static str {
    match suggestion_type {
        SuggestionType::Command => ">",
        SuggestionType::Subcommand => "-",
        SuggestionType::Option => "*",
        SuggestionType::Argument => ":",
        SuggestionType::File => "f",
        SuggestionType::Directory => "d",
    }
}
```

### Selection Indicator

```rust
pub fn render_indicator(is_selected: bool) -> &'static str {
    if is_selected {
        "→"  // Arrow
    } else {
        " "  // Space
    }
}

// ASCII fallback
pub fn render_indicator_ascii(is_selected: bool) -> &'static str {
    if is_selected { ">" } else { " " }
}
```

## Performance Optimization

### Dirty Flag

```rust
pub struct CompletionUI {
    // ... fields

    /// Whether UI needs redraw
    dirty: bool,
}

impl CompletionUI {
    pub fn render(&mut self, terminal: &mut Terminal<_>) -> Result<()> {
        // Skip render if not dirty
        if !self.dirty {
            return Ok(());
        }

        // ... render logic

        self.dirty = false;
        Ok(())
    }

    pub fn move_selection(&mut self, delta: i32) {
        let old_selected = self.selected;
        // ... update selection
        if self.selected != old_selected {
            self.dirty = true;
        }
    }
}
```

### Incremental Rendering

```rust
pub struct CompletionUI {
    // Cache rendered items
    rendered_cache: Vec<ListItem<'static>>,

    // Track what changed
    changed_indices: Vec<usize>,
}

impl CompletionUI {
    pub fn render_incremental(&mut self, terminal: &mut Terminal<_>) -> Result<()> {
        if self.changed_indices.is_empty() {
            return Ok(());
        }

        // Only re-render changed items
        for idx in &self.changed_indices {
            let item = self.create_list_item(&self.suggestions[*idx], *idx == self.selected);
            self.rendered_cache[*idx] = item;
        }

        self.changed_indices.clear();

        // ... render
    }
}
```

### Double Buffering

Ratatui handles this automatically:

- Renders to internal buffer
- Diffs against previous frame
- Only sends changed cells to terminal

## Terminal Integration

### Cursor Management

```rust
pub struct TerminalState {
    /// Saved cursor position
    cursor_position: (u16, u16),

    /// Number of lines used by UI
    ui_lines: u16,
}

impl TerminalState {
    pub fn save_cursor(&mut self) -> Result<()> {
        // Save cursor position
        execute!(
            stdout(),
            SavePosition
        )?;

        Ok(())
    }

    pub fn restore_cursor(&mut self) -> Result<()> {
        // Clear UI lines
        for _ in 0..self.ui_lines {
            execute!(
                stdout(),
                MoveDown(1),
                Clear(ClearType::CurrentLine)
            )?;
        }

        // Restore cursor
        execute!(
            stdout(),
            RestorePosition
        )?;

        Ok(())
    }
}
```

### Screen Management

```rust
pub fn enter_ui_mode() -> Result<()> {
    // Enable raw mode
    crossterm::terminal::enable_raw_mode()?;

    // Hide cursor
    execute!(stdout(), Hide)?;

    // Enter alternate screen (optional)
    // execute!(stdout(), EnterAlternateScreen)?;

    Ok(())
}

pub fn exit_ui_mode() -> Result<()> {
    // Restore cursor
    execute!(stdout(), Show)?;

    // Disable raw mode
    crossterm::terminal::disable_raw_mode()?;

    // Exit alternate screen
    // execute!(stdout(), LeaveAlternateScreen)?;

    Ok(())
}
```

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_move_selection_wraps() {
        let mut ui = CompletionUI::new(vec![
            Suggestion { text: "a".into(), ..Default::default() },
            Suggestion { text: "b".into(), ..Default::default() },
        ]);

        assert_eq!(ui.selected, 0);

        ui.move_selection(1);
        assert_eq!(ui.selected, 1);

        ui.move_selection(1);
        assert_eq!(ui.selected, 1); // Stops at end

        ui.move_selection(-1);
        assert_eq!(ui.selected, 0);

        ui.move_selection(-1);
        assert_eq!(ui.selected, 0); // Stops at start
    }

    #[test]
    fn test_scroll_follows_selection() {
        let suggestions = (0..50)
            .map(|i| Suggestion {
                text: format!("item {}", i),
                ..Default::default()
            })
            .collect();

        let mut ui = CompletionUI::new(suggestions);
        ui.max_visible = 10;

        // Select item 15
        ui.selected = 15;
        ui.update_scroll();

        // Scroll should position item 15 in view
        assert!(ui.scroll <= 15);
        assert!(15 < ui.scroll + ui.max_visible);
    }
}
```

### Visual Tests

```rust
#[test]
fn test_render_snapshot() {
    let mut ui = CompletionUI::new(vec![
        Suggestion {
            text: "checkout".into(),
            description: Some("Switch branches".into()),
            suggestion_type: SuggestionType::Subcommand,
            ..Default::default()
        },
    ]);

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    ui.render(&mut terminal)?;

    // Verify rendered output matches snapshot
    let buffer = terminal.backend().buffer();
    insta::assert_snapshot!(buffer_to_string(buffer));
}
```

### Performance Tests

```rust
#[bench]
fn bench_render(b: &mut Bencher) {
    let suggestions = (0..100)
        .map(|i| Suggestion {
            text: format!("suggestion {}", i),
            ..Default::default()
        })
        .collect();

    let mut ui = CompletionUI::new(suggestions);
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    b.iter(|| {
        ui.dirty = true;
        ui.render(&mut terminal).unwrap()
    });
}
```

**Performance Target:** <10ms render time

## Future Enhancements

### Syntax Highlighting

Highlight matched portions of suggestions:

```rust
pub fn highlight_match(text: &str, query: &str) -> Vec<Span> {
    // Find matching positions
    let matches = find_match_positions(text, query);

    // Create spans with highlighted matches
    let mut spans = vec![];
    let mut last_end = 0;

    for (start, end) in matches {
        // Normal text before match
        if start > last_end {
            spans.push(Span::raw(&text[last_end..start]));
        }

        // Highlighted match
        spans.push(Span::styled(
            &text[start..end],
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        ));

        last_end = end;
    }

    // Remaining text
    if last_end < text.len() {
        spans.push(Span::raw(&text[last_end..]));
    }

    spans
}
```

### Preview Pane

Show detailed info for selected suggestion:

```text
┌─ Completions ─────────┬─ Preview ──────────────┐
│  checkout             │ git checkout <branch>  │
│  cherry               │                        │
│ → cherry-pick         │ Apply changes from     │
│  clean                │ existing commits       │
│  clone                │                        │
│                       │ Examples:              │
│                       │   git cherry-pick abc  │
└───────────────────────┴────────────────────────┘
```

### Fuzzy Search Live Filtering

```rust
pub fn filter_suggestions(&mut self, query: &str) {
    // Filter suggestions by fuzzy match
    self.suggestions.retain(|s| {
        fuzzy_match(query, &s.text).is_some()
    });

    // Resort by match score
    self.suggestions.sort_by_key(|s| {
        -(fuzzy_match(query, &s.text).unwrap() as i32)
    });

    self.selected = 0;
    self.dirty = true;
}
```

### Mouse Support

```rust
fn handle_mouse(&mut self, mouse_event: MouseEvent) -> Result<Action> {
    match mouse_event.kind {
        MouseEventKind::ScrollUp => {
            self.move_selection(-1);
            Ok(Action::Redraw)
        }
        MouseEventKind::ScrollDown => {
            self.move_selection(1);
            Ok(Action::Redraw)
        }
        MouseEventKind::Down(MouseButton::Left) => {
            // Convert click position to item index
            let clicked_idx = self.position_to_index(mouse_event.row);
            self.selected = clicked_idx;
            Ok(Action::Select(clicked_idx))
        }
        _ => Ok(Action::Continue),
    }
}
```

## Related Documents

- [Architecture Overview](overview.md) - System architecture
- [Daemon Architecture](daemon.md) - Daemon design
- [Parser Architecture](parser.md) - Parser algorithms
- [ADR-0005: Ratatui for TUI](../adr/0005-ratatui-for-tui.md) - Design decision
- [Configuration Guide](../user-guide/configuration.md) - Theme customization
