//! Native overlay dropdown for displaying completions at the terminal cursor.
//!
//! This module provides the UI layer that renders a floating dropdown panel
//! positioned at the terminal cursor. On macOS, it uses an NSPanel (via winit's
//! `with_panel(true)`) to avoid stealing focus from the terminal.
//!
//! ## Architecture
//!
//! - `font` — Bitmap 5x7 glyph data and text rendering primitives
//! - `renderer` — Renders completion lists into pixel buffers
//! - `positioning` — Pure coordinate math for cursor positioning and edge detection
//! - `backend` — Platform-abstracted cursor position queries
//! - `macos` — macOS Accessibility API backend
//! - `app` — winit `ApplicationHandler` implementation

pub mod app;
pub mod backend;
pub mod font;
#[cfg(target_os = "macos")]
pub mod macos;
pub mod positioning;
pub mod renderer;

use crate::protocol::Suggestion;

/// Messages sent from the daemon to the overlay window.
#[derive(Debug, Clone)]
pub enum OverlayMessage {
    /// Update the displayed completions.
    UpdateCompletions { suggestions: Vec<Suggestion> },
    /// Hide the overlay (e.g. when completions are dismissed).
    Hide,
    /// The daemon is shutting down — close the overlay.
    Shutdown,
}
