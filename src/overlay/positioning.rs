//! Pure coordinate math for overlay positioning.
//!
//! Converts terminal cursor position (row, col) into screen pixel coordinates,
//! handling edge detection and flip-above logic. No platform dependencies — just
//! f64 math.

/// The computed cursor position in screen coordinates (Cocoa bottom-left origin).
#[derive(Debug, Clone, Copy)]
pub struct CursorPosition {
    /// X coordinate of the cursor (left edge of the character cell).
    pub x: f64,
    /// Bottom edge of the cursor row — panel goes below here.
    pub below_y: f64,
    /// Top edge of the cursor row — panel goes above here when flipped.
    pub above_y: f64,
}

/// Result of positioning the overlay panel on screen.
#[derive(Debug, Clone, Copy)]
pub struct OverlayRect {
    /// Panel x position.
    pub x: f64,
    /// Panel y position (Cocoa coordinates — bottom-left origin).
    pub y: f64,
    /// Whether the panel was flipped above the cursor.
    pub flipped: bool,
}

/// Window bounds as reported by the Accessibility API.
///
/// Origin is top-left in screen coordinates (AX convention).
/// Size is the content area of the window.
#[derive(Debug, Clone, Copy)]
pub struct WindowBounds {
    pub origin_x: f64,
    pub origin_y: f64,
    pub width: f64,
    pub height: f64,
}

/// Compute the cursor position in Cocoa coordinates from terminal grid position.
///
/// Converts from the Accessibility API's top-left coordinate system to Cocoa's
/// bottom-left system. `row` and `col` are 1-based (terminal convention).
pub fn compute_cursor_position(
    window: &WindowBounds,
    row: u16,
    col: u16,
    term_rows: u16,
    term_cols: u16,
    screen_height: f64,
) -> CursorPosition {
    let cell_width = window.width / f64::from(term_cols);
    let cell_height = window.height / f64::from(term_rows);

    // AXPosition gives top-left in screen coords where (0,0) is top-left of main display.
    // NSWindow/NSPanel uses bottom-left origin (Cocoa coordinates).
    // Convert: cocoa_y = screen_height - ax_y - window_height
    let window_cocoa_y = screen_height - window.origin_y - window.height;

    // CLI row/col are 1-based; convert to 0-based indices.
    let row_index = row.saturating_sub(1);
    let col_index = col.saturating_sub(1);

    // Cursor x: window left edge + col offset
    let cursor_x = window.origin_x + (f64::from(col_index) * cell_width);

    // Cursor in Cocoa coords (bottom-up origin):
    // Row 1 is at the top of the window, so row_index N is N cells down from the top.
    // Top edge:    window_cocoa_y + window_height - row_index * cell_height
    // Bottom edge: window_cocoa_y + window_height - (row_index+1) * cell_height
    let cursor_top = window_cocoa_y + window.height - f64::from(row_index) * cell_height;
    let cursor_bottom = window_cocoa_y + window.height - (f64::from(row_index) + 1.0) * cell_height;

    CursorPosition {
        x: cursor_x,
        below_y: cursor_bottom,
        above_y: cursor_top,
    }
}

/// Position the overlay panel relative to the cursor, with edge detection.
///
/// Places the panel below the cursor by default. If it would extend below the
/// bottom of the screen, flips it above the cursor. If the panel would extend
/// past the right edge of the screen, shifts it left.
pub fn position_overlay(
    cursor: &CursorPosition,
    panel_width: f64,
    panel_height: f64,
    screen_width: f64,
) -> OverlayRect {
    // Try placing below cursor
    let below = cursor.below_y - panel_height;
    let (y, flipped) = if below >= 0.0 {
        (below, false)
    } else {
        // Flip above
        (cursor.above_y, true)
    };

    // Shift left if panel extends past screen right edge
    let x = if cursor.x + panel_width > screen_width {
        (screen_width - panel_width).max(0.0)
    } else {
        cursor.x
    };

    OverlayRect { x, y, flipped }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_window() -> WindowBounds {
        WindowBounds {
            origin_x: 100.0,
            origin_y: 100.0,
            width: 800.0,
            height: 480.0,
        }
    }

    #[test]
    fn cursor_at_top_left() {
        let pos = compute_cursor_position(&sample_window(), 1, 1, 24, 80, 1080.0);
        // Row 1, col 1 should be near the top-left of the window
        assert!((pos.x - 100.0).abs() < 0.1);
        // above_y should be near the top of the window in Cocoa coords
        assert!(pos.above_y > pos.below_y);
    }

    #[test]
    fn cursor_position_increases_with_row() {
        let pos1 = compute_cursor_position(&sample_window(), 1, 1, 24, 80, 1080.0);
        let pos2 = compute_cursor_position(&sample_window(), 10, 1, 24, 80, 1080.0);
        // Later rows have lower Cocoa y (closer to screen bottom)
        assert!(pos2.below_y < pos1.below_y);
    }

    #[test]
    fn cursor_position_increases_with_col() {
        let pos1 = compute_cursor_position(&sample_window(), 1, 1, 24, 80, 1080.0);
        let pos2 = compute_cursor_position(&sample_window(), 1, 40, 24, 80, 1080.0);
        assert!(pos2.x > pos1.x);
    }

    #[test]
    fn position_overlay_below_when_space() {
        let cursor = CursorPosition {
            x: 200.0,
            below_y: 500.0,
            above_y: 520.0,
        };
        let rect = position_overlay(&cursor, 300.0, 150.0, 1920.0);
        assert!(!rect.flipped);
        assert!((rect.y - 350.0).abs() < 0.1); // 500 - 150
    }

    #[test]
    fn position_overlay_flips_above_at_bottom() {
        let cursor = CursorPosition {
            x: 200.0,
            below_y: 100.0,
            above_y: 120.0,
        };
        let rect = position_overlay(&cursor, 300.0, 150.0, 1920.0);
        assert!(rect.flipped);
        assert!((rect.y - 120.0).abs() < 0.1); // above_y
    }

    #[test]
    fn position_overlay_shifts_left_at_right_edge() {
        let cursor = CursorPosition {
            x: 1800.0,
            below_y: 500.0,
            above_y: 520.0,
        };
        let rect = position_overlay(&cursor, 300.0, 150.0, 1920.0);
        assert!((rect.x - 1620.0).abs() < 0.1); // 1920 - 300
    }

    #[test]
    fn position_overlay_no_shift_when_fits() {
        let cursor = CursorPosition {
            x: 200.0,
            below_y: 500.0,
            above_y: 520.0,
        };
        let rect = position_overlay(&cursor, 300.0, 150.0, 1920.0);
        assert!((rect.x - 200.0).abs() < 0.1);
    }
}
