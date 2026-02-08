//! Proof-of-concept: Cursor-positioned macOS overlay dropdown using NSPanel.
//!
//! Demonstrates a borderless, transparent, always-on-top panel that positions
//! itself at a specific terminal cursor location without stealing focus.
//! Uses the macOS Accessibility API to query the frontmost terminal window's
//! bounds, then computes pixel coordinates from row/col + cell dimensions.
//!
//! Run: cargo run --example overlay_poc -- --row 5 --col 10
//!      cargo run --example overlay_poc -- --pid $(pgrep -x ghostty) --row 5 --col 10
//!
//! Prerequisites:
//!   Grant Accessibility permission to your terminal (or the built binary)
//!   in System Settings → Privacy & Security → Accessibility.
//!
//! What to expect:
//! - A dropdown panel appears at the specified cursor position in the terminal
//! - The panel does NOT steal focus from your terminal
//! - Press Enter in the terminal to dismiss and exit

use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use accessibility_sys::{
    AXIsProcessTrusted, AXUIElementCopyAttributeValue, AXUIElementCreateApplication,
    AXValueGetValue, kAXErrorSuccess, kAXFocusedWindowAttribute, kAXPositionAttribute,
    kAXSizeAttribute, kAXValueTypeCGPoint, kAXValueTypeCGSize,
};
use core_foundation::base::{CFRelease, CFTypeRef, TCFType};
use core_foundation::string::CFString;
use core_graphics::geometry::{CGPoint, CGSize};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadOnly, msg_send};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSColor, NSPanel, NSScreen,
    NSWindowCollectionBehavior, NSWindowStyleMask, NSWorkspace,
};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize, NSString};

// ---------------------------------------------------------------------------
// CLI argument parsing (no clap for an example)
// ---------------------------------------------------------------------------

struct Args {
    row: u16,
    col: u16,
    pid: Option<i32>,
    term_rows: Option<u16>,
    term_cols: Option<u16>,
}

fn parse_args() -> Args {
    let args: Vec<String> = std::env::args().collect();
    let mut result = Args {
        row: 1,
        col: 0,
        pid: None,
        term_rows: None,
        term_cols: None,
    };

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--row" => {
                i += 1;
                result.row = args[i].parse().expect("--row must be a number");
            }
            "--col" => {
                i += 1;
                result.col = args[i].parse().expect("--col must be a number");
            }
            "--pid" => {
                i += 1;
                result.pid = Some(args[i].parse().expect("--pid must be a number"));
            }
            "--rows" => {
                i += 1;
                result.term_rows = Some(args[i].parse().expect("--rows must be a number"));
            }
            "--cols" => {
                i += 1;
                result.term_cols = Some(args[i].parse().expect("--cols must be a number"));
            }
            "--help" | "-h" => {
                eprintln!("Usage: overlay_poc [OPTIONS]");
                eprintln!();
                eprintln!("Options:");
                eprintln!("  --row <n>   Cursor row (default: 1)");
                eprintln!("  --col <n>   Cursor column (default: 0)");
                eprintln!("  --pid <n>   Target terminal PID (default: frontmost app)");
                eprintln!("  --rows <n>  Terminal row count override (default: TIOCGWINSZ)");
                eprintln!("  --cols <n>  Terminal col count override (default: TIOCGWINSZ)");
                std::process::exit(0);
            }
            other => {
                eprintln!("Unknown argument: {other}");
                std::process::exit(1);
            }
        }
        i += 1;
    }

    result
}

// ---------------------------------------------------------------------------
// Terminal dimensions via TIOCGWINSZ
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Frontmost application PID via NSWorkspace
// ---------------------------------------------------------------------------

fn get_frontmost_pid(_mtm: MainThreadMarker) -> Option<i32> {
    let workspace = NSWorkspace::sharedWorkspace();
    let app = workspace.frontmostApplication()?;
    let pid = app.processIdentifier();
    if pid > 0 { Some(pid) } else { None }
}

// ---------------------------------------------------------------------------
// Accessibility API: query window position and size
// ---------------------------------------------------------------------------

struct WindowBounds {
    origin: CGPoint,
    size: CGSize,
}

fn get_window_bounds(pid: i32) -> Result<WindowBounds, String> {
    unsafe {
        // Check accessibility permission first
        if !AXIsProcessTrusted() {
            return Err("Accessibility permission not granted. \
                 Go to System Settings → Privacy & Security → Accessibility \
                 and add this terminal app."
                .into());
        }

        let app_ref = AXUIElementCreateApplication(pid);
        if app_ref.is_null() {
            return Err(format!("Failed to create AX element for PID {pid}"));
        }

        // Get the focused window
        let window_ref = get_ax_attribute(app_ref as CFTypeRef, kAXFocusedWindowAttribute)?;

        // Get window position (top-left corner in screen coordinates)
        let pos_ref = get_ax_attribute(window_ref, kAXPositionAttribute)?;
        let origin = get_ax_cgpoint(pos_ref)?;
        CFRelease(pos_ref);

        // Get window size
        let size_ref = get_ax_attribute(window_ref, kAXSizeAttribute)?;
        let size = get_ax_cgsize(size_ref)?;
        CFRelease(size_ref);

        CFRelease(window_ref);
        CFRelease(app_ref as CFTypeRef);

        Ok(WindowBounds { origin, size })
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

// ---------------------------------------------------------------------------
// Cursor position computation
// ---------------------------------------------------------------------------

struct CursorPosition {
    x: f64,
    y: f64,
}

fn compute_cursor_position(
    window: &WindowBounds,
    row: u16,
    col: u16,
    term_rows: u16,
    term_cols: u16,
    screen_height: f64,
) -> CursorPosition {
    let cell_width = window.size.width / f64::from(term_cols);
    let cell_height = window.size.height / f64::from(term_rows);

    // AXPosition gives top-left in screen coords where (0,0) is top-left of main display.
    // NSWindow/NSPanel uses bottom-left origin (Cocoa coordinates).
    // Convert: cocoa_y = screen_height - ax_y - window_height
    let window_cocoa_y = screen_height - window.origin.y - window.size.height;

    // Cursor x: window left edge + col offset
    let cursor_x = window.origin.x + (f64::from(col) * cell_width);

    // Cursor y in Cocoa coords: window bottom + remaining rows below cursor
    // Row 0 is at the top of the window, so row N is N cells down from the top.
    // In Cocoa coords (bottom-up): y = window_cocoa_y + window_height - (row+1)*cell_height
    // We place the panel just below the cursor row, so subtract one more cell_height.
    let cursor_y = window_cocoa_y + window.size.height - (f64::from(row) + 1.0) * cell_height;

    CursorPosition {
        x: cursor_x,
        y: cursor_y,
    }
}

// ---------------------------------------------------------------------------
// NSPanel creation and drawing (carried over from original POC)
// ---------------------------------------------------------------------------

/// Create an NSPanel configured as a non-activating overlay.
fn create_overlay_panel(
    mtm: MainThreadMarker,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Retained<NSPanel> {
    let frame = NSRect::new(NSPoint::new(x, y), NSSize::new(width, height));

    let style = NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel;

    let panel = NSPanel::initWithContentRect_styleMask_backing_defer(
        NSPanel::alloc(mtm),
        frame,
        style,
        NSBackingStoreType::Buffered,
        false,
    );

    unsafe {
        let _: () = msg_send![&panel, setLevel: 3i64];
    }

    panel.setOpaque(false);
    let bg = NSColor::colorWithSRGBRed_green_blue_alpha(0.12, 0.12, 0.18, 0.95);
    panel.setBackgroundColor(Some(&bg));
    panel.setHasShadow(true);

    panel.setCollectionBehavior(
        NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::FullScreenAuxiliary,
    );

    panel.setHidesOnDeactivate(false);

    panel
}

/// Draw completion items into the panel's content view using NSTextField labels.
fn draw_completions(panel: &NSPanel, items: &[(&str, &str)]) {
    let content_view = panel.contentView();
    let Some(content_view) = content_view else {
        return;
    };

    let item_height: f64 = 24.0;
    let padding: f64 = 4.0;
    let panel_height = (items.len() as f64 * item_height) + (padding * 2.0);

    let current_frame = panel.frame();
    let new_frame = NSRect::new(
        current_frame.origin,
        NSSize::new(current_frame.size.width, panel_height),
    );
    panel.setFrame_display(new_frame, true);

    for (i, (name, description)) in items.iter().enumerate() {
        let y = panel_height - ((i as f64 + 1.0) * item_height) - padding;
        let label_frame = NSRect::new(
            NSPoint::new(8.0, y),
            NSSize::new(current_frame.size.width - 16.0, item_height),
        );

        let text = format!("  {name}  \u{2014}  {description}");
        let ns_text = NSString::from_str(&text);

        unsafe {
            let text_field: Retained<AnyObject> = msg_send![
                objc2::class!(NSTextField),
                labelWithString: &*ns_text
            ];

            let _: () = msg_send![&*text_field, setFrame: label_frame];
            let _: () = msg_send![&*text_field, setDrawsBackground: false];
            let _: () = msg_send![&*text_field, setBezeled: false];
            let _: () = msg_send![&*text_field, setEditable: false];
            let _: () = msg_send![&*text_field, setSelectable: false];

            let white = NSColor::whiteColor();
            let _: () = msg_send![&*text_field, setTextColor: &*white];

            let font: *mut AnyObject = msg_send![objc2::class!(NSFont), systemFontOfSize: 13.0f64];
            let _: () = msg_send![&*text_field, setFont: font];

            let _: () = msg_send![&*content_view, addSubview: &*text_field];
        }
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let args = parse_args();

    // Capture the frontmost app PID *before* NSApplication init, because
    // initializing NSApplication makes this process the frontmost app.
    let target_pid = args.pid.or_else(|| {
        // We need MainThreadMarker for NSWorkspace, but we haven't initialized
        // NSApplication yet. MainThreadMarker::new() just checks we're on the
        // main thread — it doesn't require NSApplication.
        let mtm = MainThreadMarker::new().expect("must run on main thread");
        let pid = get_frontmost_pid(mtm);
        if let Some(p) = pid {
            eprintln!("Auto-detected frontmost app PID: {p}");
        }
        pid
    });

    let mtm = MainThreadMarker::new().expect("must run on main thread");

    // Get terminal size from TIOCGWINSZ or CLI overrides
    let (tty_rows, tty_cols) = get_terminal_size().unwrap_or((24, 80));
    let term_rows = args.term_rows.unwrap_or(tty_rows);
    let term_cols = args.term_cols.unwrap_or(tty_cols);
    eprintln!("Terminal size: {term_rows} rows x {term_cols} cols");
    eprintln!("Target cursor: row={}, col={}", args.row, args.col);

    // Query the target window's bounds via the Accessibility API
    let (panel_x, panel_y) = if let Some(pid) = target_pid {
        match get_window_bounds(pid) {
            Ok(bounds) => {
                eprintln!(
                    "Window bounds: origin=({:.0}, {:.0}), size=({:.0}x{:.0})",
                    bounds.origin.x, bounds.origin.y, bounds.size.width, bounds.size.height
                );

                // Get screen height for coordinate conversion
                let screen_height = NSScreen::mainScreen(mtm)
                    .map(|s| s.frame().size.height)
                    .unwrap_or(1080.0);

                let pos = compute_cursor_position(
                    &bounds,
                    args.row,
                    args.col,
                    term_rows,
                    term_cols,
                    screen_height,
                );
                (pos.x, pos.y)
            }
            Err(e) => {
                eprintln!("Warning: Could not query window bounds: {e}");
                eprintln!("Falling back to default position (400, 400).");
                (400.0, 400.0)
            }
        }
    } else {
        eprintln!("Warning: No target PID available. Using default position (400, 400).");
        (400.0, 400.0)
    };

    // Initialize NSApplication (must happen before creating windows)
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

    let items = [
        ("checkout", "Switch branches or restore files"),
        ("commit", "Record changes to the repository"),
        ("push", "Update remote refs along with objects"),
        ("pull", "Fetch and integrate with another repo"),
        ("stash", "Stash changes in a dirty working dir"),
    ];

    // Place the panel at the computed cursor position, sized to fit the dropdown
    let panel_width = 380.0;
    let panel_height = 150.0;
    // Offset panel below the cursor line
    let panel = create_overlay_panel(
        mtm,
        panel_x,
        panel_y - panel_height,
        panel_width,
        panel_height,
    );

    draw_completions(&panel, &items);
    panel.orderFrontRegardless();

    eprintln!(
        "Panel positioned at ({panel_x:.0}, {:.0})",
        panel_y - panel_height
    );
    println!(
        "Overlay panel is visible at row={}, col={}.",
        args.row, args.col
    );
    println!("Press Enter to dismiss and exit.");

    static SHOULD_QUIT: AtomicBool = AtomicBool::new(false);
    std::thread::spawn(|| {
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).ok();
        SHOULD_QUIT.store(true, Ordering::Relaxed);
    });

    while !SHOULD_QUIT.load(Ordering::Relaxed) {
        unsafe {
            let mode = NSString::from_str("kCFRunLoopDefaultMode");
            let event: Option<Retained<AnyObject>> = msg_send![
                &app,
                nextEventMatchingMask: u64::MAX,
                untilDate: std::ptr::null::<AnyObject>(),
                inMode: &*mode,
                dequeue: true
            ];
            if let Some(event) = event {
                let _: () = msg_send![&app, sendEvent: &*event];
            }
        }
        std::thread::sleep(Duration::from_millis(16));
    }

    panel.orderOut(None);
    println!("Panel dismissed. Exiting.");
}
