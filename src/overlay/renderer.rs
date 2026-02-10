//! Renders a completion dropdown into a pixel buffer.
//!
//! Takes a list of [`Suggestion`]s and a selected index, and draws the dropdown
//! into a `&mut [u32]` pixel buffer (ARGB format for softbuffer). Platform-
//! independent — just pixel math.

use crate::protocol::Suggestion;

use super::font::{self, CHAR_WIDTH, FONT_HEIGHT};

/// Panel width in physical pixels.
pub const PANEL_WIDTH: u32 = 700;

/// Height of each completion item row in pixels.
pub const ITEM_HEIGHT: u32 = 32;

/// Vertical padding at top and bottom of the panel.
pub const PADDING: u32 = 6;

/// Maximum number of visible items before scrolling.
pub const MAX_VISIBLE_ITEMS: usize = 8;

/// Left margin for text content.
const LEFT_MARGIN: u32 = 12;

// Colors (ARGB format for softbuffer).
const BG_COLOR: u32 = 0xFF1E1E2E;
const SELECTED_BG: u32 = 0xFF3E3E5E;
const TEXT_COLOR: u32 = 0xFFCDD6F4;
const DESC_COLOR: u32 = 0xFF6C7086;

/// Compute the panel height in pixels for the given number of suggestions.
pub fn panel_height(suggestion_count: usize) -> u32 {
    let visible = suggestion_count.min(MAX_VISIBLE_ITEMS);
    (visible as u32 * ITEM_HEIGHT) + (PADDING * 2)
}

/// Render completions into a pixel buffer.
///
/// The buffer must be at least `width * panel_height(suggestions.len())` elements.
/// `selected` is the 0-based index of the highlighted item.
pub fn render_completions(
    buf: &mut [u32],
    width: u32,
    suggestions: &[Suggestion],
    selected: usize,
) {
    let height = buf.len() as u32 / width;

    // Fill background
    buf.fill(BG_COLOR);

    let visible = suggestions.len().min(MAX_VISIBLE_ITEMS);

    for (i, suggestion) in suggestions.iter().enumerate().take(visible) {
        let item_y = PADDING + (i as u32 * ITEM_HEIGHT);

        // Highlight selected item
        if i == selected {
            let start_row = item_y.min(height);
            let end_row = (item_y + ITEM_HEIGHT).min(height);
            let buf_len = buf.len();
            for row in start_row..end_row {
                let start = (row * width) as usize;
                let end = ((row + 1) * width) as usize;
                if start < buf_len {
                    buf[start..end.min(buf_len)].fill(SELECTED_BG);
                }
            }
        }

        // Draw item name
        let text_y = item_y + (ITEM_HEIGHT - FONT_HEIGHT) / 2;
        font::draw_text(
            buf,
            width,
            LEFT_MARGIN,
            text_y,
            &suggestion.text,
            TEXT_COLOR,
        );

        // Draw description after a dash separator
        let desc_x = LEFT_MARGIN + (suggestion.text.len() as u32 + 3) * CHAR_WIDTH;
        font::draw_text(buf, width, desc_x - 2 * CHAR_WIDTH, text_y, "-", DESC_COLOR);
        font::draw_text(
            buf,
            width,
            desc_x,
            text_y,
            &suggestion.description,
            DESC_COLOR,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_height_empty() {
        assert_eq!(panel_height(0), PADDING * 2);
    }

    #[test]
    fn panel_height_few_items() {
        assert_eq!(panel_height(3), 3 * ITEM_HEIGHT + PADDING * 2);
    }

    #[test]
    fn panel_height_capped_at_max() {
        assert_eq!(
            panel_height(20),
            MAX_VISIBLE_ITEMS as u32 * ITEM_HEIGHT + PADDING * 2
        );
    }

    #[test]
    fn render_empty_suggestions() {
        let height = panel_height(0);
        let mut buf = vec![0u32; (PANEL_WIDTH * height) as usize];
        render_completions(&mut buf, PANEL_WIDTH, &[], 0);
        // All pixels should be background color
        assert!(buf.iter().all(|&p| p == BG_COLOR));
    }

    #[test]
    fn render_with_suggestions() {
        let suggestions = vec![
            Suggestion {
                text: "checkout".into(),
                description: "Switch branches".into(),
            },
            Suggestion {
                text: "commit".into(),
                description: "Record changes".into(),
            },
        ];
        let height = panel_height(suggestions.len());
        let mut buf = vec![0u32; (PANEL_WIDTH * height) as usize];
        render_completions(&mut buf, PANEL_WIDTH, &suggestions, 0);
        // Should have some non-background pixels (text was drawn)
        assert!(buf.iter().any(|&p| p != BG_COLOR));
        // First item should have selected background
        assert!(buf.contains(&SELECTED_BG));
    }

    #[test]
    fn render_selected_second_item() {
        let suggestions = vec![
            Suggestion {
                text: "a".into(),
                description: "first".into(),
            },
            Suggestion {
                text: "b".into(),
                description: "second".into(),
            },
        ];
        let height = panel_height(suggestions.len());
        let mut buf = vec![0u32; (PANEL_WIDTH * height) as usize];
        render_completions(&mut buf, PANEL_WIDTH, &suggestions, 1);
        // Second item row should have SELECTED_BG pixels
        let second_row_start = (PADDING + ITEM_HEIGHT) * PANEL_WIDTH;
        let second_row_end = second_row_start + ITEM_HEIGHT * PANEL_WIDTH;
        let second_slice = &buf[second_row_start as usize..second_row_end as usize];
        assert!(second_slice.contains(&SELECTED_BG));
    }
}
