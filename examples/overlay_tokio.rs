//! Spike: winit event loop + Tokio async runtime coexistence.
//!
//! Tests whether we can run a Tokio async runtime on a background thread while
//! winit owns the main thread for the overlay window. This is the key question
//! for deciding whether the daemon and overlay live in one process or two.
//!
//! Architecture:
//!   Main thread:       winit event loop (NSPanel overlay)
//!   Background thread: Tokio runtime (simulated daemon accepting Unix socket connections)
//!   Communication:     std::sync::mpsc + EventLoopProxy::wake_up()
//!
//! Run: cargo run --example overlay_tokio
//!
//! What to expect:
//! - A dark floating panel appears (does NOT steal terminal focus)
//! - Every 2 seconds, the Tokio runtime "discovers" new completions and sends
//!   them to the winit overlay via mpsc channel
//! - The overlay renders the updated completions in real-time
//! - A simulated Unix socket listener accepts connections on a temp socket
//! - Press Escape to exit (Ctrl+C terminates ungracefully — no cleanup)
//!
//! Key questions this spike answers:
//! 1. Can Tokio run on a background thread while winit owns main?
//! 2. What's the latency of cross-thread mpsc + wake_up() communication?
//! 3. Can we shut down both runtimes cleanly?
//! 4. Can the Tokio side accept real Unix socket connections?
//! 5. Does the overlay remain responsive while Tokio is doing async I/O?

use std::num::NonZeroU32;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use softbuffer::Surface;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::window::{Window, WindowAttributes, WindowId, WindowLevel};

#[cfg(target_os = "macos")]
use winit::platform::macos::WindowAttributesMacOS;

// ---------------------------------------------------------------------------
// Messages from Tokio → winit
// ---------------------------------------------------------------------------

/// Messages sent from the Tokio runtime to the winit overlay.
#[allow(dead_code)] // Shutdown used in production, not this demo
enum OverlayMessage {
    /// New set of completions to display, with the sender's timestamp for
    /// measuring cross-thread wake latency.
    UpdateCompletions(Vec<CompletionItem>, Instant),
    /// The Tokio runtime is shutting down.
    Shutdown,
}

#[derive(Clone, Debug)]
struct CompletionItem {
    name: String,
    description: String,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const PANEL_WIDTH: u32 = 700;
const ITEM_HEIGHT: u32 = 32;
const PADDING: u32 = 6;
const MAX_VISIBLE_ITEMS: usize = 8;

const BG_COLOR: u32 = 0xFF1E1E2E;
const SELECTED_BG: u32 = 0xFF3E3E5E;
const TEXT_COLOR: u32 = 0xFFCDD6F4;
const DESC_COLOR: u32 = 0xFF6C7086;
const HEADER_COLOR: u32 = 0xFF89B4FA;

const FONT_SCALE: u32 = 3;

// ---------------------------------------------------------------------------
// Bitmap font (same as overlay_winit.rs — minimal 5x7)
// ---------------------------------------------------------------------------

fn draw_char(buf: &mut [u32], stride: u32, x: u32, y: u32, ch: char, color: u32) {
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
        ':' => [
            0b00000, 0b00100, 0b00000, 0b00000, 0b00100, 0b00000, 0b00000,
        ],
        '#' => [
            0b01010, 0b11111, 0b01010, 0b01010, 0b11111, 0b01010, 0b00000,
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
        _ => [
            0b11110, 0b11110, 0b11110, 0b11110, 0b11110, 0b11110, 0b00000,
        ],
    };

    for (row, &bits) in glyph.iter().enumerate() {
        for col in 0..5 {
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

fn draw_text(buf: &mut [u32], stride: u32, x: u32, y: u32, text: &str, color: u32) {
    let char_width = 6 * FONT_SCALE;
    for (i, ch) in text.chars().enumerate() {
        draw_char(buf, stride, x + (i as u32 * char_width), y, ch, color);
    }
}

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

struct App {
    /// Receive completions from the Tokio background thread.
    rx: mpsc::Receiver<OverlayMessage>,
    window: Option<Rc<dyn Window>>,
    surface: Option<Surface<Rc<dyn Window>, Rc<dyn Window>>>,
    last_size: (u32, u32),
    selected: usize,
    items: Vec<CompletionItem>,
    /// Status line showing cross-thread communication timing.
    status: String,
    /// Count of updates received from Tokio.
    update_count: u64,
}

impl App {
    fn panel_height(&self) -> u32 {
        let visible = self.items.len().min(MAX_VISIBLE_ITEMS);
        let items_height = visible as u32 * ITEM_HEIGHT;
        // Header line + items + status line
        PADDING + ITEM_HEIGHT + items_height + ITEM_HEIGHT + PADDING
    }

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

        if (width, height) != self.last_size {
            surface.resize(w, h).expect("resize");
            self.last_size = (width, height);
        }
        let mut buf = surface.buffer_mut().expect("buffer");

        buf.fill(BG_COLOR);

        // Header: show update count
        let header = format!(
            "Completions #{} - {} items",
            self.update_count,
            self.items.len()
        );
        draw_text(&mut buf, width, 12, PADDING + 4, &header, HEADER_COLOR);
        let font_height = 7 * FONT_SCALE;

        // Draw completion items
        let items_start_y = PADDING + ITEM_HEIGHT;
        let visible = self.items.len().min(MAX_VISIBLE_ITEMS);
        for i in 0..visible {
            let item = &self.items[i];
            let item_y = items_start_y + (i as u32 * ITEM_HEIGHT);

            // Highlight selected
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

            let text_y = item_y + (ITEM_HEIGHT - font_height) / 2;
            let char_width = 6 * FONT_SCALE;
            draw_text(&mut buf, width, 12, text_y, &item.name, TEXT_COLOR);

            let desc_x = 12 + (item.name.len() as u32 + 3) * char_width;
            draw_text(
                &mut buf,
                width,
                desc_x - 2 * char_width,
                text_y,
                "-",
                DESC_COLOR,
            );
            draw_text(
                &mut buf,
                width,
                desc_x,
                text_y,
                &item.description,
                DESC_COLOR,
            );
        }

        // Status line at bottom
        let status_y = items_start_y + (visible as u32 * ITEM_HEIGHT) + 4;
        draw_text(&mut buf, width, 12, status_y, &self.status, DESC_COLOR);

        buf.present().expect("present");
    }

    fn resize_window(&self) {
        if let Some(window) = self.window.as_ref() {
            let new_height = self.panel_height();
            // request_surface_size returns the actual size (may differ from requested);
            // we don't need it — the next render will observe the real surface size.
            let _actual =
                window.request_surface_size(LogicalSize::new(PANEL_WIDTH, new_height).into());
        }
    }
}

impl ApplicationHandler for App {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        let panel_height = self.panel_height();

        #[cfg(target_os = "macos")]
        let attrs = {
            let macos_attrs = WindowAttributesMacOS::default()
                .with_panel(true)
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

        eprintln!("Overlay created. Waiting for Tokio completions...");
        eprintln!("Panel style: NSPanel={}", cfg!(target_os = "macos"));
        eprintln!("Focus test: click your terminal — it should keep focus.");
        eprintln!("Press Escape to exit.");

        self.window = Some(window);
        self.surface = Some(surface);
        self.render();
    }

    fn proxy_wake_up(&mut self, event_loop: &dyn ActiveEventLoop) {
        // Drain all pending messages (wake-ups may be coalesced; last update wins)
        for msg in self.rx.try_iter() {
            match msg {
                OverlayMessage::UpdateCompletions(items, sent_at) => {
                    let latency = sent_at.elapsed();
                    self.update_count += 1;
                    self.items = items;
                    self.selected = 0;
                    self.status = format!(
                        "Update #{} - send-to-receive latency: {}us",
                        self.update_count,
                        latency.as_micros()
                    );
                    eprintln!(
                        "[overlay] received update #{}, {} items, send-to-receive latency: {:?}",
                        self.update_count,
                        self.items.len(),
                        latency
                    );
                }
                OverlayMessage::Shutdown => {
                    eprintln!("[overlay] received shutdown from Tokio");
                    event_loop.exit();
                    return;
                }
            }
        }

        // Resize and redraw
        self.resize_window();
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
                            if !self.items.is_empty() {
                                self.selected = (self.selected + 1) % self.items.len();
                                self.render();
                            }
                        }
                        Key::Named(NamedKey::ArrowUp) => {
                            if !self.items.is_empty() {
                                self.selected = if self.selected == 0 {
                                    self.items.len() - 1
                                } else {
                                    self.selected - 1
                                };
                                self.render();
                            }
                        }
                        Key::Named(NamedKey::Enter) => {
                            if let Some(item) = self.items.get(self.selected) {
                                eprintln!("[overlay] accepted: {}", item.name);
                            }
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
// Tokio background runtime
// ---------------------------------------------------------------------------

/// Unique socket path counter.
static SOCKET_COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_socket_path() -> PathBuf {
    let pid = std::process::id();
    let id = SOCKET_COUNTER.fetch_add(1, Ordering::Relaxed);
    PathBuf::from(format!(
        "/tmp/autocomplete-rs-overlay-spike-{}-{}.sock",
        pid, id
    ))
}

/// Simulated completion sets that rotate every 2 seconds.
fn completion_sets() -> Vec<Vec<CompletionItem>> {
    vec![
        vec![
            CompletionItem {
                name: "checkout".into(),
                description: "Switch branches".into(),
            },
            CompletionItem {
                name: "commit".into(),
                description: "Record changes".into(),
            },
            CompletionItem {
                name: "cherry-pick".into(),
                description: "Apply a commit".into(),
            },
        ],
        vec![
            CompletionItem {
                name: "push".into(),
                description: "Update remote refs".into(),
            },
            CompletionItem {
                name: "pull".into(),
                description: "Fetch and merge".into(),
            },
            CompletionItem {
                name: "prune".into(),
                description: "Remove stale refs".into(),
            },
            CompletionItem {
                name: "pack-refs".into(),
                description: "Pack loose refs".into(),
            },
        ],
        vec![
            CompletionItem {
                name: "stash".into(),
                description: "Stash changes".into(),
            },
            CompletionItem {
                name: "status".into(),
                description: "Show working tree".into(),
            },
            CompletionItem {
                name: "switch".into(),
                description: "Switch branches".into(),
            },
            CompletionItem {
                name: "submodule".into(),
                description: "Manage submodules".into(),
            },
            CompletionItem {
                name: "show".into(),
                description: "Show objects".into(),
            },
            CompletionItem {
                name: "shortlog".into(),
                description: "Summarize log".into(),
            },
        ],
    ]
}

/// Run the Tokio runtime on a background thread.
///
/// Binds a Unix socket listener (proving real async I/O works alongside winit),
/// and periodically sends simulated completion updates to the overlay.
fn spawn_tokio_runtime(
    tx: mpsc::Sender<OverlayMessage>,
    proxy: EventLoopProxy,
) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("tokio-runtime".into())
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .worker_threads(2)
                .thread_name("tokio-worker")
                .build()
                .expect("failed to build Tokio runtime");

            rt.block_on(async {
                let socket_path = temp_socket_path();

                // Remove stale socket from a previous run of this same process before binding
                if let Err(e) = std::fs::remove_file(&socket_path)
                    && e.kind() != std::io::ErrorKind::NotFound
                {
                    eprintln!("[tokio] failed to clean stale socket: {e}");
                }

                // Bind a real Unix socket to prove async I/O works
                let listener = match tokio::net::UnixListener::bind(&socket_path) {
                    Ok(l) => {
                        // Restrict socket access to the owning user
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            if let Err(e) =
                                std::fs::set_permissions(&socket_path, std::fs::Permissions::from_mode(0o600))
                            {
                                eprintln!("[tokio] failed to set socket permissions: {e}");
                            }
                        }
                        eprintln!("[tokio] listening on {}", socket_path.display());
                        Some(l)
                    }
                    Err(e) => {
                        eprintln!("[tokio] failed to bind socket: {e} (continuing without)");
                        None
                    }
                };

                let sets = completion_sets();
                // First tick fires immediately (sends an initial completion set right
                // away), then subsequent ticks fire every 2 seconds.
                let mut interval = tokio::time::interval(Duration::from_secs(2));
                let mut cycle = 0usize;
                let mut connection_count = 0u64;

                loop {
                    tokio::select! {
                        biased;

                        _ = interval.tick() => {
                            let items = sets[cycle % sets.len()].clone();
                            let send_start = Instant::now();

                            if let Err(e) = tx.send(OverlayMessage::UpdateCompletions(items, send_start)) {
                                eprintln!("[tokio] overlay closed ({e}), shutting down");
                                break;
                            }
                            proxy.wake_up();

                            let send_latency = send_start.elapsed();
                            eprintln!(
                                "[tokio] sent completion set #{} (cycle {}), send latency: {:?}",
                                cycle + 1,
                                cycle % sets.len(),
                                send_latency
                            );
                            cycle += 1;
                        }

                        // Accept connections on the Unix socket (proves async I/O works)
                        result = async {
                            if let Some(ref listener) = listener {
                                listener.accept().await
                            } else {
                                // No listener — just pend forever
                                std::future::pending().await
                            }
                        } => {
                            match result {
                                Ok((_stream, _addr)) => {
                                    connection_count += 1;
                                    eprintln!(
                                        "[tokio] accepted connection #{connection_count} \
                                         (send a line to test)"
                                    );
                                }
                                Err(e) => {
                                    eprintln!("[tokio] accept error: {e}");
                                }
                            }
                        }
                    }
                }

                // Clean up socket
                if let Err(e) = std::fs::remove_file(&socket_path)
                    && e.kind() != std::io::ErrorKind::NotFound
                {
                    eprintln!("[tokio] failed to clean up socket: {e}");
                }

                eprintln!("[tokio] runtime exiting");
            });
        })
        .expect("failed to spawn Tokio thread")
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("=== winit + Tokio coexistence spike ===");
    eprintln!("Main thread: winit event loop (overlay window)");
    eprintln!("Background thread: Tokio runtime (async I/O + completions)");
    eprintln!();

    let event_loop = EventLoop::new()?;
    let proxy = event_loop.create_proxy();
    let (tx, rx) = mpsc::channel();

    // Spawn the Tokio runtime on a background thread
    let tokio_handle = spawn_tokio_runtime(tx, proxy);

    // Run winit on the main thread (blocks until exit)
    event_loop.run_app(App {
        rx,
        window: None,
        surface: None,
        last_size: (0, 0),
        selected: 0,
        items: vec![],
        status: "Waiting for Tokio...".into(),
        update_count: 0,
    })?;

    // winit exited — wait for Tokio thread to notice and shut down
    eprintln!("[main] winit exited, waiting for Tokio thread...");

    // The Tokio thread will notice the channel is closed on next send
    // and exit its loop. Give it a moment.
    match tokio_handle.join() {
        Ok(()) => eprintln!("[main] Tokio thread joined cleanly"),
        Err(e) => eprintln!("[main] Tokio thread panicked: {e:?}"),
    }

    eprintln!("[main] clean exit");
    Ok(())
}
