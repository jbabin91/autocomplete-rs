//! Proof-of-concept: winit 0.31 overlay dropdown using NSPanel.
//!
//! Tests whether winit's `with_panel(true)` (NSPanel + NonactivatingPanel)
//! provides a non-focus-stealing overlay suitable for terminal autocomplete.
//! Uses softbuffer for CPU rendering of the dropdown content.
//!
//! Run: cargo run --example overlay_winit
//!
//! What to expect:
//! - A dark floating panel appears at (200, 200) on screen
//! - The panel does NOT steal focus from your terminal
//! - Press Escape in the panel (if focused) or close to exit
//!
//! Key questions this spike answers:
//! 1. Does winit's NSPanel actually prevent focus stealing?
//! 2. Can we position at specific screen coordinates?
//! 3. Can we render with softbuffer (no GPU framework needed)?
//! 4. Can we set always-on-top, borderless, transparent?

use std::num::NonZeroU32;
use std::rc::Rc;

use softbuffer::Surface;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId, WindowLevel};

#[cfg(target_os = "macos")]
use winit::platform::macos::WindowAttributesMacOS;

// ---------------------------------------------------------------------------
// Completion items (same as overlay_poc for comparison)
// ---------------------------------------------------------------------------

const ITEMS: &[(&str, &str)] = &[
    ("checkout", "Switch branches or restore files"),
    ("commit", "Record changes to the repository"),
    ("push", "Update remote refs along with objects"),
    ("pull", "Fetch and integrate with another repo"),
    ("stash", "Stash changes in a dirty working dir"),
];

const PANEL_WIDTH: u32 = 700;
const ITEM_HEIGHT: u32 = 32;
const PADDING: u32 = 6;

// ---------------------------------------------------------------------------
// Colors (ARGB format for softbuffer)
// ---------------------------------------------------------------------------

const BG_COLOR: u32 = 0xFF1E1E2E; // dark background
const SELECTED_BG: u32 = 0xFF3E3E5E; // highlighted item
const TEXT_COLOR: u32 = 0xFFCDD6F4; // light text
const DESC_COLOR: u32 = 0xFF6C7086; // dimmed description

// ---------------------------------------------------------------------------
// Simple pixel text rendering (8x8 bitmap font — just enough for a POC)
// ---------------------------------------------------------------------------

/// Scale factor for the bitmap font (2 = double size, 3 = triple).
/// Retina displays need at least 2x to be readable.
const FONT_SCALE: u32 = 3;

/// Render a single ASCII character at (x, y) into the buffer, scaled by FONT_SCALE.
fn draw_char(buf: &mut [u32], stride: u32, x: u32, y: u32, ch: char, color: u32) {
    // Minimal 5x7 font stored as 7 bytes per glyph (each byte = row bitmask)
    // Only includes chars we need for the demo
    let glyph = match ch {
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
        'R' => [
            0b11100, 0b10010, 0b10010, 0b11100, 0b10100, 0b10010, 0b00000,
        ],
        'S' => [
            0b01110, 0b10000, 0b01100, 0b00010, 0b00010, 0b11100, 0b00000,
        ],
        'U' => [
            0b10010, 0b10010, 0b10010, 0b10010, 0b10010, 0b01100, 0b00000,
        ],
        'W' => [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b01010, 0b00000,
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
        _ => [
            0b11110, 0b11110, 0b11110, 0b11110, 0b11110, 0b11110, 0b00000,
        ], // block for unknown
    };

    for (row, &bits) in glyph.iter().enumerate() {
        for col in 0..5 {
            if bits & (1 << (4 - col)) != 0 {
                // Draw a FONT_SCALE x FONT_SCALE block for each pixel
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

/// Draw a string at (x, y) with scaled character spacing.
fn draw_text(buf: &mut [u32], stride: u32, x: u32, y: u32, text: &str, color: u32) {
    let char_width = 6 * FONT_SCALE; // 5px glyph + 1px gap, scaled
    for (i, ch) in text.chars().enumerate() {
        draw_char(buf, stride, x + (i as u32 * char_width), y, ch, color);
    }
}

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

struct App {
    window: Option<Rc<dyn Window>>,
    surface: Option<Surface<Rc<dyn Window>, Rc<dyn Window>>>,
    last_size: (u32, u32),
    selected: usize,
}

impl App {
    fn render(&mut self) {
        let Some(window) = self.window.as_ref() else {
            return;
        };

        let size = window.surface_size();
        let width = size.width;
        let height = size.height;

        let Some(w) = NonZeroU32::new(width) else {
            return;
        };
        let Some(h) = NonZeroU32::new(height) else {
            return;
        };

        let Some(surface) = self.surface.as_mut() else {
            return;
        };

        // Only resize when dimensions actually change
        if (width, height) != self.last_size {
            surface.resize(w, h).expect("resize");
            self.last_size = (width, height);
        }
        let mut buf = surface.buffer_mut().expect("buffer");

        // Fill background
        buf.fill(BG_COLOR);

        // Draw each completion item
        for (i, (name, desc)) in ITEMS.iter().enumerate() {
            let item_y = PADDING + (i as u32 * ITEM_HEIGHT);

            // Highlight selected item using slice fill (avoids per-pixel branching)
            if i == self.selected {
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
            let font_height = 7 * FONT_SCALE;
            let text_y = item_y + (ITEM_HEIGHT - font_height) / 2;
            let char_width = 6 * FONT_SCALE;
            let left_margin = 12;
            draw_text(&mut buf, width, left_margin, text_y, name, TEXT_COLOR);

            // Draw description after a dash
            let desc_x = left_margin + (name.len() as u32 + 3) * char_width;
            draw_text(
                &mut buf,
                width,
                desc_x - 2 * char_width,
                text_y,
                "-",
                DESC_COLOR,
            );
            draw_text(&mut buf, width, desc_x, text_y, desc, DESC_COLOR);
        }

        buf.present().expect("present");
    }
}

impl ApplicationHandler for App {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        let panel_height = (ITEMS.len() as u32 * ITEM_HEIGHT) + (PADDING * 2);

        #[cfg(target_os = "macos")]
        let attrs = {
            let macos_attrs = WindowAttributesMacOS::default()
                .with_panel(true) // NSPanel + NonactivatingPanel
                .with_has_shadow(true);

            WindowAttributes::default()
                .with_decorations(false)
                .with_transparent(true)
                .with_window_level(WindowLevel::AlwaysOnTop)
                .with_position(LogicalPosition::new(200.0, 200.0))
                .with_surface_size(LogicalSize::new(PANEL_WIDTH as f64, panel_height as f64))
                .with_resizable(false)
                .with_active(false)
                .with_platform_attributes(Box::new(macos_attrs))
        };

        #[cfg(not(target_os = "macos"))]
        let attrs = WindowAttributes::default()
            .with_decorations(false)
            .with_transparent(true)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_position(LogicalPosition::new(200.0, 200.0))
            .with_surface_size(LogicalSize::new(PANEL_WIDTH as f64, panel_height as f64))
            .with_resizable(false)
            .with_active(false);

        let window: Rc<dyn Window> = Rc::from(
            event_loop
                .create_window(attrs)
                .expect("failed to create window"),
        );

        let context = softbuffer::Context::new(Rc::clone(&window)).expect("softbuffer context");
        let surface = Surface::new(&context, Rc::clone(&window)).expect("softbuffer surface");

        eprintln!(
            "Window created at (200, 200), size {}x{panel_height}",
            PANEL_WIDTH
        );
        eprintln!("Panel style: NSPanel={}", cfg!(target_os = "macos"));
        eprintln!("Focus test: click in your terminal — it should keep focus.");
        eprintln!("Press Escape to exit.");

        self.window = Some(window);
        self.surface = Some(surface);
        self.render();
    }

    fn window_event(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. } => {
                use winit::event::ElementState;
                use winit::keyboard::{Key, NamedKey};

                if event.state == ElementState::Pressed {
                    match event.logical_key {
                        Key::Named(NamedKey::Escape) => event_loop.exit(),
                        Key::Named(NamedKey::ArrowDown) => {
                            self.selected = (self.selected + 1) % ITEMS.len();
                            eprintln!("Selected: {} ({})", ITEMS[self.selected].0, self.selected);
                            self.render();
                        }
                        Key::Named(NamedKey::ArrowUp) => {
                            self.selected = if self.selected == 0 {
                                ITEMS.len() - 1
                            } else {
                                self.selected - 1
                            };
                            eprintln!("Selected: {} ({})", ITEMS[self.selected].0, self.selected);
                            self.render();
                        }
                        Key::Named(NamedKey::Enter) => {
                            eprintln!("Accepted: {}", ITEMS[self.selected].0);
                            event_loop.exit();
                        }
                        _ => {}
                    }
                }
            }
            WindowEvent::RedrawRequested => self.render(),
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("winit overlay spike — testing NSPanel non-focus-stealing behavior");

    let event_loop = EventLoop::new()?;
    event_loop.run_app(App {
        window: None,
        surface: None,
        last_size: (0, 0),
        selected: 0,
    })?;

    eprintln!("Exited cleanly.");
    Ok(())
}
