//! macOS overlay positioning via the Accessibility API.
//!
//! Queries the frontmost terminal window's bounds using `AXUIElement` and
//! computes overlay position using TIOCGWINSZ for terminal dimensions.
//! Requires Accessibility permission (System Settings > Privacy & Security).

use std::mem::MaybeUninit;

use accessibility_sys::{
    AXIsProcessTrusted, AXUIElementCopyAttributeValue, AXUIElementCreateApplication,
    AXValueGetValue, kAXErrorSuccess, kAXFocusedWindowAttribute, kAXPositionAttribute,
    kAXSizeAttribute, kAXValueTypeCGPoint, kAXValueTypeCGSize,
};
use core_foundation::base::{CFRelease, CFTypeRef, TCFType};
use core_foundation::string::CFString;
use core_graphics::geometry::{CGPoint, CGSize};
use objc2_app_kit::{NSScreen, NSWorkspace};
use objc2_foundation::MainThreadMarker;

use super::backend::{OverlayBackend, OverlayPosition, PositioningError};
use super::positioning::{self, WindowBounds};

/// macOS-specific overlay positioning using the Accessibility API.
pub struct MacosBackend;

impl OverlayBackend for MacosBackend {
    fn compute_position(
        &self,
        panel_width: f64,
        panel_height: f64,
    ) -> Result<OverlayPosition, PositioningError> {
        let pid = get_frontmost_pid().ok_or(PositioningError::NoFrontmostApp)?;
        let bounds = get_window_bounds(pid)?;

        let (term_rows, term_cols) = get_terminal_size().ok_or(PositioningError::NoTerminalSize)?;

        // SAFETY: MainThreadMarker::new() just checks we're on the main thread.
        // The overlay backend is only called from the winit event loop (main thread).
        let mtm = MainThreadMarker::new().expect("must run on main thread");
        let screen_height = NSScreen::mainScreen(mtm)
            .map(|s| s.frame().size.height)
            .ok_or(PositioningError::NoScreen)?;
        let screen_width = NSScreen::mainScreen(mtm)
            .map(|s| s.frame().size.width)
            .ok_or(PositioningError::NoScreen)?;

        // Use cursor row 1, col 1 as default (will be updated from shell buffer later).
        // For now, position at the bottom-left of the terminal window.
        let cursor_row = term_rows;
        let cursor_col = 1;

        let cursor_pos = positioning::compute_cursor_position(
            &bounds,
            cursor_row,
            cursor_col,
            term_rows,
            term_cols,
            screen_height,
        );

        let rect =
            positioning::position_overlay(&cursor_pos, panel_width, panel_height, screen_width);

        Ok(OverlayPosition::from(rect))
    }
}

/// Get the PID of the frontmost application.
fn get_frontmost_pid() -> Option<i32> {
    let workspace = NSWorkspace::sharedWorkspace();
    let app = workspace.frontmostApplication()?;
    let pid = app.processIdentifier();
    if pid > 0 { Some(pid) } else { None }
}

/// Query terminal dimensions via TIOCGWINSZ.
fn get_terminal_size() -> Option<(u16, u16)> {
    unsafe {
        let mut ws: libc::winsize = std::mem::zeroed();
        if libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut ws) == 0
            && ws.ws_col > 0
            && ws.ws_row > 0
        {
            Some((ws.ws_row, ws.ws_col))
        } else {
            None
        }
    }
}

/// Query the focused window's bounds for the given PID via Accessibility API.
fn get_window_bounds(pid: i32) -> Result<WindowBounds, PositioningError> {
    unsafe {
        if !AXIsProcessTrusted() {
            return Err(PositioningError::AccessibilityNotGranted);
        }

        let app_ref = AXUIElementCreateApplication(pid);
        if app_ref.is_null() {
            return Err(PositioningError::WindowQuery(format!(
                "failed to create AX element for PID {pid}"
            )));
        }

        let window_ref = get_ax_attribute(app_ref as CFTypeRef, kAXFocusedWindowAttribute)
            .map_err(PositioningError::WindowQuery)?;

        let pos_ref = get_ax_attribute(window_ref, kAXPositionAttribute)
            .map_err(PositioningError::WindowQuery)?;
        let origin = get_ax_cgpoint(pos_ref).map_err(PositioningError::WindowQuery)?;
        CFRelease(pos_ref);

        let size_ref = get_ax_attribute(window_ref, kAXSizeAttribute)
            .map_err(PositioningError::WindowQuery)?;
        let size = get_ax_cgsize(size_ref).map_err(PositioningError::WindowQuery)?;
        CFRelease(size_ref);

        CFRelease(window_ref);
        CFRelease(app_ref as CFTypeRef);

        Ok(WindowBounds {
            origin_x: origin.x,
            origin_y: origin.y,
            width: size.width,
            height: size.height,
        })
    }
}

unsafe fn get_ax_attribute(element: CFTypeRef, attr: &str) -> Result<CFTypeRef, String> {
    let cf_attr = CFString::new(attr);
    let mut value: CFTypeRef = std::ptr::null();
    let err = unsafe {
        AXUIElementCopyAttributeValue(
            element as *mut _,
            cf_attr.as_concrete_TypeRef() as *mut _,
            &mut value as *mut CFTypeRef,
        )
    };
    if err != kAXErrorSuccess {
        return Err(format!(
            "AXUIElementCopyAttributeValue({attr}) failed: error {err}"
        ));
    }
    if value.is_null() {
        return Err(format!(
            "AXUIElementCopyAttributeValue({attr}) returned null"
        ));
    }
    Ok(value)
}

unsafe fn get_ax_cgpoint(value: CFTypeRef) -> Result<CGPoint, String> {
    let mut point = MaybeUninit::<CGPoint>::uninit();
    if !unsafe {
        AXValueGetValue(
            value as *mut _,
            kAXValueTypeCGPoint,
            point.as_mut_ptr().cast(),
        )
    } {
        return Err("AXValueGetValue(CGPoint) failed".into());
    }
    Ok(unsafe { point.assume_init() })
}

unsafe fn get_ax_cgsize(value: CFTypeRef) -> Result<CGSize, String> {
    let mut size = MaybeUninit::<CGSize>::uninit();
    if !unsafe {
        AXValueGetValue(
            value as *mut _,
            kAXValueTypeCGSize,
            size.as_mut_ptr().cast(),
        )
    } {
        return Err("AXValueGetValue(CGSize) failed".into());
    }
    Ok(unsafe { size.assume_init() })
}
