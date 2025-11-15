use crate::daemon::Suggestion;
use anyhow::Result;
use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
};
use std::io;

pub struct CompletionUI {
    suggestions: Vec<Suggestion>,
    selected: usize,
}

impl CompletionUI {
    pub fn new(suggestions: Vec<Suggestion>) -> Self {
        Self {
            suggestions,
            selected: 0,
        }
    }

    /// Display the TUI and return the selected suggestion (if any)
    pub fn run(&mut self) -> Result<Option<Suggestion>> {
        // Don't show TUI if no suggestions
        if self.suggestions.is_empty() {
            return Ok(None);
        }

        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        stdout.execute(EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let result = self.run_app(&mut terminal);

        // Restore terminal
        disable_raw_mode()?;
        terminal.backend_mut().execute(LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        result
    }

    fn run_app<B: ratatui::backend::Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
    ) -> Result<Option<Suggestion>> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Esc => return Ok(None),
                    KeyCode::Enter => {
                        return Ok(Some(self.suggestions[self.selected].clone()));
                    }
                    KeyCode::Down => {
                        // Wrap around to beginning
                        self.selected = (self.selected + 1) % self.suggestions.len();
                    }
                    KeyCode::Up => {
                        // Wrap around to end
                        if self.selected == 0 {
                            self.selected = self.suggestions.len() - 1;
                        } else {
                            self.selected -= 1;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn ui(&self, f: &mut ratatui::Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0)])
            .split(f.area());

        let items: Vec<ListItem> = self
            .suggestions
            .iter()
            .enumerate()
            .map(|(i, suggestion)| {
                let is_selected = i == self.selected;

                // Build the line with text and description
                let mut spans = vec![Span::styled(
                    &suggestion.text,
                    if is_selected {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    },
                )];

                // Add description if present
                if !suggestion.description.is_empty() {
                    spans.push(Span::raw(" - "));
                    spans.push(Span::styled(
                        &suggestion.description,
                        if is_selected {
                            Style::default().fg(Color::Yellow)
                        } else {
                            Style::default().fg(Color::Gray)
                        },
                    ));
                }

                ListItem::new(Line::from(spans))
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Completions")
                .style(Style::default().fg(Color::Cyan)),
        );

        f.render_widget(list, chunks[0]);
    }
}
