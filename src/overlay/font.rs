//! Bitmap 5x7 glyph data and text rendering primitives.
//!
//! Provides a minimal ASCII bitmap font sufficient for rendering completion
//! item names and descriptions in the overlay dropdown. Each glyph is 5 pixels
//! wide by 7 pixels tall, stored as 7 bytes (one per row, MSB-first bitmask).

/// Scale factor for the bitmap font (pixels per glyph pixel).
/// Retina displays need at least 2x to be readable.
pub const FONT_SCALE: u32 = 3;

/// Glyph width in logical pixels (before scaling).
const GLYPH_WIDTH: u32 = 5;

/// Glyph height in logical pixels (before scaling).
const GLYPH_HEIGHT: usize = 7;

/// Character cell width including 1px gap, scaled.
pub const CHAR_WIDTH: u32 = (GLYPH_WIDTH + 1) * FONT_SCALE;

/// Font height in physical pixels (scaled).
pub const FONT_HEIGHT: u32 = GLYPH_HEIGHT as u32 * FONT_SCALE;

/// Look up the 5x7 bitmap glyph for an ASCII character.
///
/// Returns 7 bytes, each representing one row of the glyph as a bitmask
/// (bit 4 = leftmost pixel, bit 0 = rightmost pixel). Unknown characters
/// render as a solid block.
fn glyph(ch: char) -> [u8; GLYPH_HEIGHT] {
    match ch {
        'a' => [
            0b00000, 0b01110, 0b00010, 0b01110, 0b10010, 0b01110, 0b00000,
        ],
        'b' => [
            0b10000, 0b10000, 0b11100, 0b10010, 0b10010, 0b11100, 0b00000,
        ],
        'c' => [
            0b00000, 0b01110, 0b10000, 0b10000, 0b10000, 0b01110, 0b00000,
        ],
        'd' => [
            0b00010, 0b00010, 0b01110, 0b10010, 0b10010, 0b01110, 0b00000,
        ],
        'e' => [
            0b00000, 0b01100, 0b10010, 0b11110, 0b10000, 0b01110, 0b00000,
        ],
        'f' => [
            0b00110, 0b01000, 0b11100, 0b01000, 0b01000, 0b01000, 0b00000,
        ],
        'g' => [
            0b00000, 0b01110, 0b10010, 0b01110, 0b00010, 0b01100, 0b00000,
        ],
        'h' => [
            0b10000, 0b10000, 0b11100, 0b10010, 0b10010, 0b10010, 0b00000,
        ],
        'i' => [
            0b00100, 0b00000, 0b00100, 0b00100, 0b00100, 0b00100, 0b00000,
        ],
        'j' => [
            0b00010, 0b00000, 0b00010, 0b00010, 0b10010, 0b01100, 0b00000,
        ],
        'k' => [
            0b10000, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b00000,
        ],
        'l' => [
            0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110, 0b00000,
        ],
        'm' => [
            0b00000, 0b11010, 0b10101, 0b10101, 0b10001, 0b10001, 0b00000,
        ],
        'n' => [
            0b00000, 0b10100, 0b11010, 0b10010, 0b10010, 0b10010, 0b00000,
        ],
        'o' => [
            0b00000, 0b01100, 0b10010, 0b10010, 0b10010, 0b01100, 0b00000,
        ],
        'p' => [
            0b00000, 0b11100, 0b10010, 0b11100, 0b10000, 0b10000, 0b00000,
        ],
        'q' => [
            0b00000, 0b01110, 0b10010, 0b01110, 0b00010, 0b00010, 0b00000,
        ],
        'r' => [
            0b00000, 0b10110, 0b11000, 0b10000, 0b10000, 0b10000, 0b00000,
        ],
        's' => [
            0b00000, 0b01110, 0b10000, 0b01100, 0b00010, 0b11100, 0b00000,
        ],
        't' => [
            0b01000, 0b11100, 0b01000, 0b01000, 0b01000, 0b00110, 0b00000,
        ],
        'u' => [
            0b00000, 0b10010, 0b10010, 0b10010, 0b10010, 0b01110, 0b00000,
        ],
        'v' => [
            0b00000, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100, 0b00000,
        ],
        'w' => [
            0b00000, 0b10001, 0b10001, 0b10101, 0b10101, 0b01010, 0b00000,
        ],
        'x' => [
            0b00000, 0b10010, 0b01100, 0b01100, 0b01100, 0b10010, 0b00000,
        ],
        'y' => [
            0b00000, 0b10010, 0b10010, 0b01110, 0b00010, 0b01100, 0b00000,
        ],
        'z' => [
            0b00000, 0b11110, 0b00100, 0b01000, 0b10000, 0b11110, 0b00000,
        ],
        'A' => [
            0b01100, 0b10010, 0b10010, 0b11110, 0b10010, 0b10010, 0b00000,
        ],
        'B' => [
            0b11100, 0b10010, 0b11100, 0b10010, 0b10010, 0b11100, 0b00000,
        ],
        'C' => [
            0b01110, 0b10000, 0b10000, 0b10000, 0b10000, 0b01110, 0b00000,
        ],
        'D' => [
            0b11100, 0b10010, 0b10010, 0b10010, 0b10010, 0b11100, 0b00000,
        ],
        'E' => [
            0b11110, 0b10000, 0b11100, 0b10000, 0b10000, 0b11110, 0b00000,
        ],
        'F' => [
            0b11110, 0b10000, 0b11100, 0b10000, 0b10000, 0b10000, 0b00000,
        ],
        'G' => [
            0b01110, 0b10000, 0b10000, 0b10110, 0b10010, 0b01110, 0b00000,
        ],
        'H' => [
            0b10010, 0b10010, 0b11110, 0b10010, 0b10010, 0b10010, 0b00000,
        ],
        'I' => [
            0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110, 0b00000,
        ],
        'J' => [
            0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100, 0b00000,
        ],
        'K' => [
            0b10010, 0b10100, 0b11000, 0b11000, 0b10100, 0b10010, 0b00000,
        ],
        'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11110, 0b00000,
        ],
        'M' => [
            0b10001, 0b11011, 0b10101, 0b10001, 0b10001, 0b10001, 0b00000,
        ],
        'N' => [
            0b10010, 0b11010, 0b10110, 0b10010, 0b10010, 0b10010, 0b00000,
        ],
        'O' => [
            0b01100, 0b10010, 0b10010, 0b10010, 0b10010, 0b01100, 0b00000,
        ],
        'P' => [
            0b11100, 0b10010, 0b10010, 0b11100, 0b10000, 0b10000, 0b00000,
        ],
        'Q' => [
            0b01100, 0b10010, 0b10010, 0b10010, 0b10110, 0b01110, 0b00000,
        ],
        'R' => [
            0b11100, 0b10010, 0b10010, 0b11100, 0b10100, 0b10010, 0b00000,
        ],
        'S' => [
            0b01110, 0b10000, 0b01100, 0b00010, 0b00010, 0b11100, 0b00000,
        ],
        'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00000,
        ],
        'U' => [
            0b10010, 0b10010, 0b10010, 0b10010, 0b10010, 0b01100, 0b00000,
        ],
        'V' => [
            0b10001, 0b10001, 0b10001, 0b01010, 0b01010, 0b00100, 0b00000,
        ],
        'W' => [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b01010, 0b00000,
        ],
        'X' => [
            0b10010, 0b10010, 0b01100, 0b01100, 0b10010, 0b10010, 0b00000,
        ],
        'Y' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00000,
        ],
        'Z' => [
            0b11110, 0b00010, 0b00100, 0b01000, 0b10000, 0b11110, 0b00000,
        ],
        '0' => [
            0b01100, 0b10010, 0b10110, 0b11010, 0b10010, 0b01100, 0b00000,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b01110, 0b00000,
        ],
        '2' => [
            0b01100, 0b10010, 0b00100, 0b01000, 0b10000, 0b11110, 0b00000,
        ],
        '3' => [
            0b01100, 0b10010, 0b00100, 0b00010, 0b10010, 0b01100, 0b00000,
        ],
        '4' => [
            0b00010, 0b00110, 0b01010, 0b11110, 0b00010, 0b00010, 0b00000,
        ],
        '5' => [
            0b11110, 0b10000, 0b11100, 0b00010, 0b10010, 0b01100, 0b00000,
        ],
        '6' => [
            0b01100, 0b10000, 0b11100, 0b10010, 0b10010, 0b01100, 0b00000,
        ],
        '7' => [
            0b11110, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000, 0b00000,
        ],
        '8' => [
            0b01100, 0b10010, 0b01100, 0b10010, 0b10010, 0b01100, 0b00000,
        ],
        '9' => [
            0b01100, 0b10010, 0b10010, 0b01110, 0b00010, 0b01100, 0b00000,
        ],
        ' ' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000,
        ],
        '-' => [
            0b00000, 0b00000, 0b00000, 0b11110, 0b00000, 0b00000, 0b00000,
        ],
        '.' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00100, 0b00000,
        ],
        '/' => [
            0b00010, 0b00010, 0b00100, 0b01000, 0b10000, 0b10000, 0b00000,
        ],
        ':' => [
            0b00000, 0b00100, 0b00000, 0b00000, 0b00100, 0b00000, 0b00000,
        ],
        '#' => [
            0b01010, 0b11111, 0b01010, 0b01010, 0b11111, 0b01010, 0b00000,
        ],
        '(' => [
            0b00010, 0b00100, 0b00100, 0b00100, 0b00100, 0b00010, 0b00000,
        ],
        ')' => [
            0b01000, 0b00100, 0b00100, 0b00100, 0b00100, 0b01000, 0b00000,
        ],
        ',' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00100, 0b01000, 0b00000,
        ],
        '+' => [
            0b00000, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00000,
        ],
        '=' => [
            0b00000, 0b00000, 0b11110, 0b00000, 0b11110, 0b00000, 0b00000,
        ],
        _ => [
            0b11110, 0b11110, 0b11110, 0b11110, 0b11110, 0b11110, 0b00000,
        ],
    }
}

/// Render a single character at pixel position `(x, y)` into a pixel buffer.
///
/// The buffer is a row-major `u32` array with `stride` pixels per row.
/// Each glyph pixel is scaled by [`FONT_SCALE`] for readability on HiDPI displays.
pub fn draw_char(buf: &mut [u32], stride: u32, x: u32, y: u32, ch: char, color: u32) {
    let glyph = glyph(ch);

    for (row, &bits) in glyph.iter().enumerate() {
        for col in 0..GLYPH_WIDTH {
            if bits & (1 << (4 - col)) != 0 {
                for sy in 0..FONT_SCALE {
                    for sx in 0..FONT_SCALE {
                        let px = x + col * FONT_SCALE + sx;
                        let py = y + row as u32 * FONT_SCALE + sy;
                        if px < stride {
                            let idx = (py * stride + px) as usize;
                            if idx < buf.len() {
                                buf[idx] = color;
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Render a string at pixel position `(x, y)` into a pixel buffer.
///
/// Characters are spaced by [`CHAR_WIDTH`] pixels (glyph width + 1px gap, scaled).
pub fn draw_text(buf: &mut [u32], stride: u32, x: u32, y: u32, text: &str, color: u32) {
    for (i, ch) in text.chars().enumerate() {
        draw_char(buf, stride, x + (i as u32 * CHAR_WIDTH), y, ch, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draw_char_within_bounds() {
        let mut buf = vec![0u32; 100 * 100];
        draw_char(&mut buf, 100, 10, 10, 'A', 0xFFFFFFFF);
        // At least some pixels should be set
        assert!(buf.contains(&0xFFFFFFFF));
    }

    #[test]
    fn draw_char_out_of_bounds_does_not_panic() {
        let mut buf = vec![0u32; 10];
        draw_char(&mut buf, 5, 100, 100, 'X', 0xFFFFFFFF);
        // Should not panic, buffer unchanged
    }

    #[test]
    fn draw_text_renders_multiple_chars() {
        let mut buf = vec![0u32; 200 * 50];
        draw_text(&mut buf, 200, 0, 0, "ab", 0xFF00FF00);
        assert!(buf.contains(&0xFF00FF00));
    }

    #[test]
    fn space_glyph_is_empty() {
        let mut buf = vec![0u32; 100 * 100];
        draw_char(&mut buf, 100, 10, 10, ' ', 0xFFFFFFFF);
        // Space should not set any pixels
        assert!(!buf.contains(&0xFFFFFFFF));
    }
}
