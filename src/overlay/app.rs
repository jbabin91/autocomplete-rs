//! winit `ApplicationHandler` for the overlay dropdown.
//!
//! `OverlayApp` receives [`OverlayMessage`]s from the daemon via an mpsc channel
//! and renders completions using softbuffer. The window starts hidden and becomes
//! visible when non-empty suggestions arrive.

use std::num::NonZeroU32;
use std::rc::Rc;
use std::sync::mpsc;

use softbuffer::Surface;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::window::{Window, WindowAttributes, WindowId, WindowLevel};

#[cfg(target_os = "macos")]
use winit::platform::macos::WindowAttributesMacOS;

use super::OverlayMessage;
use super::renderer::{self, PANEL_WIDTH};
use crate::protocol::Suggestion;

/// The overlay application, driven by winit's event loop.
pub struct OverlayApp {
    rx: mpsc::Receiver<OverlayMessage>,
    window: Option<Rc<dyn Window>>,
    surface: Option<Surface<Rc<dyn Window>, Rc<dyn Window>>>,
    last_size: (u32, u32),
    suggestions: Vec<Suggestion>,
    selected: usize,
    visible: bool,
}

impl OverlayApp {
    /// Create a new `OverlayApp` with the given message receiver.
    pub fn new(rx: mpsc::Receiver<OverlayMessage>) -> Self {
        Self {
            rx,
            window: None,
            surface: None,
            last_size: (0, 0),
            suggestions: Vec::new(),
            selected: 0,
            visible: false,
        }
    }

    /// Create a winit [`EventLoopProxy`] and mpsc channel pair for communicating
    /// with the overlay from the daemon.
    ///
    /// Returns `(proxy, sender, receiver)`. The caller passes `sender` + a closure
    /// over `proxy.wake_up()` to the daemon thread, and `receiver` to `OverlayApp::new()`.
    pub fn create_channel(
        event_loop: &EventLoop,
    ) -> (
        EventLoopProxy,
        mpsc::Sender<OverlayMessage>,
        mpsc::Receiver<OverlayMessage>,
    ) {
        let proxy = event_loop.create_proxy();
        let (tx, rx) = mpsc::channel();
        (proxy, tx, rx)
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
            if let Err(e) = surface.resize(w, h) {
                tracing::warn!("surface resize failed: {e}");
                return;
            }
            self.last_size = (width, height);
        }
        let mut buf = match surface.buffer_mut() {
            Ok(buf) => buf,
            Err(e) => {
                tracing::warn!("failed to get surface buffer: {e}");
                return;
            }
        };

        renderer::render_completions(&mut buf, width, &self.suggestions, self.selected);

        if let Err(e) = buf.present() {
            tracing::warn!("failed to present frame: {e}");
        }
    }

    fn show_window(&mut self) {
        if let Some(window) = self.window.as_ref()
            && !self.visible
        {
            window.set_visible(true);
            self.visible = true;
        }
    }

    fn hide_window(&mut self) {
        if let Some(window) = self.window.as_ref()
            && self.visible
        {
            window.set_visible(false);
            self.visible = false;
        }
    }

    fn resize_window(&self) {
        if let Some(window) = self.window.as_ref() {
            let new_height = renderer::panel_height(self.suggestions.len());
            let _ = window.request_surface_size(LogicalSize::new(PANEL_WIDTH, new_height).into());
        }
    }
}

impl ApplicationHandler for OverlayApp {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        let panel_height = renderer::panel_height(0);

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
                .with_visible(false)
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
            .with_active(false)
            .with_visible(false);

        let window: Rc<dyn Window> = match event_loop.create_window(attrs) {
            Ok(w) => Rc::from(w),
            Err(e) => {
                tracing::error!("failed to create overlay window: {e}");
                event_loop.exit();
                return;
            }
        };

        let context = match softbuffer::Context::new(Rc::clone(&window)) {
            Ok(ctx) => ctx,
            Err(e) => {
                tracing::error!("failed to create softbuffer context: {e}");
                event_loop.exit();
                return;
            }
        };
        let surface = match Surface::new(&context, Rc::clone(&window)) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("failed to create softbuffer surface: {e}");
                event_loop.exit();
                return;
            }
        };

        tracing::info!("overlay window created (hidden)");

        self.window = Some(window);
        self.surface = Some(surface);
    }

    fn proxy_wake_up(&mut self, event_loop: &dyn ActiveEventLoop) {
        // Drain all pending messages into a Vec first to avoid borrowing
        // self.rx for the duration of the loop body.
        let messages: Vec<_> = self.rx.try_iter().collect();

        for msg in messages {
            match msg {
                OverlayMessage::UpdateCompletions { suggestions } => {
                    self.suggestions = suggestions;
                    self.selected = 0;

                    if self.suggestions.is_empty() {
                        self.hide_window();
                    } else {
                        self.resize_window();
                        self.show_window();
                        self.render();
                    }
                }
                OverlayMessage::Hide => {
                    self.hide_window();
                }
                OverlayMessage::Shutdown => {
                    tracing::info!("overlay received shutdown");
                    event_loop.exit();
                    return;
                }
            }
        }
    }

    fn window_event(
        &mut self,
        _event_loop: &dyn ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                // Treat close as hide — the daemon drives shutdown explicitly
                // via OverlayMessage::Shutdown. Exiting the event loop here
                // would leave the daemon thread running and cause a hang.
                self.hide_window();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                use winit::event::ElementState;
                use winit::keyboard::{Key, NamedKey};

                if event.state == ElementState::Pressed {
                    match event.logical_key {
                        Key::Named(NamedKey::Escape) => {
                            // Hide overlay, don't exit daemon
                            self.hide_window();
                        }
                        Key::Named(NamedKey::ArrowDown) => {
                            if !self.suggestions.is_empty() {
                                self.selected = (self.selected + 1) % self.suggestions.len();
                                self.render();
                            }
                        }
                        Key::Named(NamedKey::ArrowUp) => {
                            if !self.suggestions.is_empty() {
                                self.selected = if self.selected == 0 {
                                    self.suggestions.len() - 1
                                } else {
                                    self.selected - 1
                                };
                                self.render();
                            }
                        }
                        Key::Named(NamedKey::Enter) => {
                            if let Some(item) = self.suggestions.get(self.selected) {
                                tracing::debug!(selection = %item.text, "completion accepted");
                            }
                            self.hide_window();
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
