//! Platform-abstracted overlay positioning backend.
//!
//! Defines the [`OverlayBackend`] trait for querying the terminal window's
//! position and computing where to place the overlay panel. Each platform
//! provides its own implementation (e.g. macOS Accessibility API).

use super::positioning::OverlayRect;

/// Result of a successful position computation.
#[derive(Debug, Clone, Copy)]
pub struct OverlayPosition {
    /// Panel x coordinate (Cocoa / screen coordinates).
    pub x: f64,
    /// Panel y coordinate (Cocoa / screen coordinates).
    pub y: f64,
    /// Whether the panel was flipped above the cursor.
    pub flipped: bool,
}

impl From<OverlayRect> for OverlayPosition {
    fn from(rect: OverlayRect) -> Self {
        Self {
            x: rect.x,
            y: rect.y,
            flipped: rect.flipped,
        }
    }
}

/// Errors that can occur during overlay positioning.
#[derive(Debug, thiserror::Error)]
pub enum PositioningError {
    #[error("accessibility permission not granted")]
    AccessibilityNotGranted,

    #[error("no frontmost application found")]
    NoFrontmostApp,

    #[error("failed to query window bounds: {0}")]
    WindowQuery(String),

    #[error("no screen information available")]
    NoScreen,

    #[error("terminal size unavailable")]
    NoTerminalSize,
}

/// Trait for platform-specific overlay positioning.
///
/// Implementations query the OS for the frontmost terminal window's bounds
/// and compute pixel coordinates for the overlay panel.
pub trait OverlayBackend: Send + Sync {
    /// Compute the screen position for an overlay panel of the given dimensions.
    fn compute_position(
        &self,
        panel_width: f64,
        panel_height: f64,
    ) -> Result<OverlayPosition, PositioningError>;
}
