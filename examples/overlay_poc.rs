//! Proof-of-concept: Native macOS overlay dropdown using NSPanel.
//!
//! Demonstrates that we can create a borderless, transparent, always-on-top
//! panel that does NOT steal focus from the terminal. This is the same
//! approach Fig.io used for its autocomplete dropdown.
//!
//! Run: cargo run --example overlay_poc
//!
//! What to expect:
//! - A small dropdown panel appears near the top-left of the screen
//! - The panel does NOT steal focus from your terminal
//! - It renders a simple completion list with NSTextField labels
//! - Press Enter in the terminal to dismiss and exit

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadOnly, msg_send};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSColor, NSPanel,
    NSWindowCollectionBehavior, NSWindowStyleMask,
};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize, NSString};

/// Create an NSPanel configured as a non-activating overlay.
///
/// Key properties:
/// - `NonactivatingPanel` style: doesn't steal focus from the terminal
/// - Always-on-top via floating window level
/// - No titlebar, transparent background
fn create_overlay_panel(
    mtm: MainThreadMarker,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Retained<NSPanel> {
    let frame = NSRect::new(NSPoint::new(x, y), NSSize::new(width, height));

    // NonactivatingPanel is the critical flag — it prevents the panel from
    // becoming the key window and stealing focus from the terminal.
    let style = NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel;

    let panel = NSPanel::initWithContentRect_styleMask_backing_defer(
        NSPanel::alloc(mtm),
        frame,
        style,
        NSBackingStoreType::Buffered,
        false,
    );

    // Float above all other windows (NSFloatingWindowLevel = 3)
    unsafe {
        let _: () = msg_send![&panel, setLevel: 3i64];
    }

    // Dark semi-transparent background (rounded corners deferred to real renderer)
    panel.setOpaque(false);
    let bg = NSColor::colorWithSRGBRed_green_blue_alpha(0.1, 0.1, 0.15, 0.95);
    panel.setBackgroundColor(Some(&bg));

    // Collection behavior: work across spaces and with fullscreen apps
    panel.setCollectionBehavior(
        NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::FullScreenAuxiliary,
    );

    // Don't hide when app deactivates (we want it visible while terminal has focus)
    panel.setHidesOnDeactivate(false);

    panel
}

/// Draw completion items into the panel's content view using NSTextField labels.
/// In a real implementation, this would use wgpu or Metal for GPU rendering.
fn draw_completions(panel: &NSPanel, items: &[(&str, &str)]) {
    let content_view = panel.contentView();
    let Some(content_view) = content_view else {
        return;
    };

    let item_height: f64 = 24.0;
    let padding: f64 = 4.0;
    let panel_height = (items.len() as f64 * item_height) + (padding * 2.0);

    // Resize panel to fit content
    let current_frame = panel.frame();
    let new_frame = NSRect::new(
        current_frame.origin,
        NSSize::new(current_frame.size.width, panel_height),
    );
    panel.setFrame_display(new_frame, true);

    // Add text labels for each completion item
    for (i, (name, description)) in items.iter().enumerate() {
        let y = panel_height - ((i as f64 + 1.0) * item_height) - padding;
        let label_frame = NSRect::new(
            NSPoint::new(8.0, y),
            NSSize::new(current_frame.size.width - 16.0, item_height),
        );

        let text = format!("  {}  —  {}", name, description);
        let ns_text = NSString::from_str(&text);

        unsafe {
            // Create an NSTextField as a label
            let text_field: Retained<AnyObject> = msg_send![
                objc2::class!(NSTextField),
                labelWithString: &*ns_text
            ];

            let _: () = msg_send![&*text_field, setFrame: label_frame];
            let _: () = msg_send![&*text_field, setDrawsBackground: false];
            let _: () = msg_send![&*text_field, setBezeled: false];
            let _: () = msg_send![&*text_field, setEditable: false];
            let _: () = msg_send![&*text_field, setSelectable: false];

            // Set text color to white
            let white = NSColor::whiteColor();
            let _: () = msg_send![&*text_field, setTextColor: &*white];

            // Set small system font
            let font: *mut AnyObject = msg_send![objc2::class!(NSFont), systemFontOfSize: 13.0f64];
            let _: () = msg_send![&*text_field, setFont: font];

            let _: () = msg_send![&*content_view, addSubview: &*text_field];
        }
    }
}

fn main() {
    // MainThreadMarker proves we're on the main thread (required for AppKit)
    let mtm = MainThreadMarker::new().expect("must run on main thread");

    // Initialize the application (required for NSPanel to work)
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

    // Sample completion items
    let items = [
        ("checkout", "Switch branches or restore files"),
        ("commit", "Record changes to the repository"),
        ("push", "Update remote refs along with objects"),
        ("pull", "Fetch and integrate with another repo"),
        ("stash", "Stash changes in a dirty working dir"),
    ];

    // Position the panel — in a real implementation, this would be computed
    // from the terminal window position + cursor row/col + cell dimensions
    let panel = create_overlay_panel(mtm, 200.0, 400.0, 350.0, 150.0);

    // Draw the completion items
    draw_completions(&panel, &items);

    // Show the panel without activating (focus stays on terminal)
    panel.orderFrontRegardless();

    println!("Overlay panel is visible. It should NOT steal focus from this terminal.");
    println!("The panel shows 5 git subcommand completions.");
    println!("Press Enter to dismiss and exit.");

    // Wait for user input (the terminal retains focus)
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).ok();

    // Hide the panel
    panel.orderOut(None);

    println!("Panel dismissed. Exiting.");
}
