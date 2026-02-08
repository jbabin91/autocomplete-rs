---
paths:
  - 'src/overlay/**'
  - 'examples/overlay_poc.rs'
---

# Native Overlay Dropdown Rules

## Target Architecture

The completion dropdown renders as a **native overlay window** positioned at
the terminal cursor — like Fig.io's approach, NOT inline ANSI rendering.
Platform-specific backends handle window creation and positioning.

See [ADR-0008](docs/adr/0008-native-overlay-dropdown.md) for the full decision.

Platform backends:

- **macOS:** NSPanel (NonactivatingPanel) + Accessibility API for cursor positioning
- **Linux X11:** override-redirect window (x11rb) + \_NET_ACTIVE_WINDOW
- **Linux Wayland:** wlr-layer-shell (smithay-client-toolkit) + shell integration for position
- **Windows:** WS_EX_NOACTIVATE + Win32 GetWindowRect

## Current State

The overlay dropdown is **not yet implemented** as a proper module. A
proof-of-concept exists at `examples/overlay_poc.rs` demonstrating NSPanel
cursor positioning on macOS.

## Key Properties

- **Non-focus-stealing:** The overlay must NOT steal keyboard focus from the terminal
- **Always-on-top:** Float above other windows (NSFloatingWindowLevel on macOS)
- **Cursor-anchored:** Positioned at the terminal cursor's pixel coordinates
- **Edge-aware:** Flip above cursor if panel would go off-screen below

## Key Bindings

- `Esc` → cancel (return None)
- `Enter` → select (return Some(Suggestion))
- `Down` → next item (wraps)
- `Up` → previous item (wraps)

## Constraints

- Max ~5-8 visible suggestions (scrollable)
- Smart positioning: render above cursor if near bottom of screen
- Must work on macOS initially; Linux X11/Wayland and Windows are follow-up
- Render time: <10ms target
