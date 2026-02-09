# ADR-0008: Native Overlay Dropdown

**Status:** Accepted **Date:** 2026-02-08 **Decision Makers:** Project Team
**Technical Story:** Replace inline ANSI rendering with native GUI overlay
for completion dropdown
**Supersedes:** [ADR-0004](0004-direct-terminal-control.md) (Direct Terminal
Control), [ADR-0006](0006-inline-ansi-dropdown.md) (Inline ANSI Dropdown)

## Context

ADR-0004 chose "direct terminal control" (inline ANSI rendering) over native
overlays, citing Fig.io's positioning bugs as evidence that the overlay
approach was fundamentally flawed. ADR-0006 refined this to use raw ANSI
escape codes via crossterm.

After deeper research and prototyping, we found:

1. **Fig's bugs were implementation quality issues, not architectural flaws.**
   Fig used the macOS Accessibility API for cursor positioning. The bugs
   came from poor error handling, missing edge cases, and a Node.js +
   Electron stack — not from the overlay concept itself.

2. **Inline ANSI rendering has significant UX limitations.** It pushes
   terminal content down, interferes with scrollback, causes flicker on
   redraw, and can't overlay existing content. No serious autocomplete
   tool (Fig, Kiro CLI, Warp) uses inline ANSI — they all use native
   overlays.

3. **The overlay approach works correctly when implemented properly.** A
   proof-of-concept using macOS NSPanel + Accessibility API + TIOCGWINSZ
   demonstrated accurate cursor-positioned overlays without focus stealing,
   including flip-above behavior when the panel would go off-screen.

## Decision

Use **platform-specific native overlay windows** to render the completion
dropdown, positioned at the terminal cursor via platform APIs.

### Architecture

```text
┌──────────────────────────────────────────┐
│  Overlay Process                         │
│  ┌────────────────────────────────────┐  │
│  │  Platform Backend (trait)          │  │
│  │  - macOS: NSPanel + Accessibility  │  │
│  │  - Linux X11: override-redirect    │  │
│  │  - Linux Wayland: layer-shell      │  │
│  │  - Windows: WS_EX_NOACTIVATE      │  │
│  └────────────────────────────────────┘  │
│  ┌────────────────────────────────────┐  │
│  │  Positioning Pipeline              │  │
│  │  1. Query terminal window bounds   │  │
│  │  2. Get terminal grid dimensions   │  │
│  │  3. Compute cell pixel position    │  │
│  │  4. Flip above if near edge        │  │
│  └────────────────────────────────────┘  │
└──────────────────────────────────────────┘
```

### Platform Backends

| Platform      | Overlay Window                            | Window Position Query                      | Terminal Size       |
| ------------- | ----------------------------------------- | ------------------------------------------ | ------------------- |
| macOS         | NSPanel via winit 0.31 `with_panel(true)` | Accessibility API (AXPosition/AXSize)      | TIOCGWINSZ          |
| Linux X11     | override-redirect window (x11rb)          | \_NET_ACTIVE_WINDOW + XGetWindowAttributes | TIOCGWINSZ          |
| Linux Wayland | wlr-layer-shell (smithay-client-toolkit)  | Shell integration reports position         | TIOCGWINSZ          |
| Windows       | WS_EX_NOACTIVATE (windows crate)          | Win32 GetWindowRect                        | Windows console API |

### Key Properties

- **Non-focus-stealing:** The overlay must not steal keyboard focus from the
  terminal. On macOS this is `NSWindowStyleMask::NonactivatingPanel`.
- **Always-on-top:** The overlay floats above other windows.
- **Cursor-anchored:** Positioned at the terminal cursor's pixel coordinates.
- **Edge-aware:** Flips above cursor when it would go off-screen below;
  shifts left when it would go off-screen right.
- **Cross-space:** Visible across macOS Spaces and with fullscreen apps.

## Consequences

### Positive

- **Native feel:** The overlay looks like a first-class OS UI element, not
  terminal text. Shadows, transparency, custom rendering.
- **No content disruption:** Overlays float above terminal content without
  pushing it down or interfering with scrollback.
- **Rich rendering:** Can use GPU rendering, custom fonts, images, icons —
  not limited to terminal character grid.
- **Proven approach:** This is how Fig.io, Kiro CLI, and Warp all render
  their autocomplete UI.

### Negative

- **Platform-specific code:** Each platform needs its own overlay backend.
  This is unavoidable — no cross-platform crate abstracts non-activating
  overlay windows.
- **Accessibility permissions (macOS):** Querying window position requires
  the user to grant Accessibility permission in System Settings.
- **Wayland limitations:** Wayland does not allow querying other windows'
  positions by design. The shell integration must report cursor pixel
  coordinates instead (via terminal-specific escape sequences or env vars).
- **More complex than inline:** Window management, coordinate systems,
  multi-monitor handling, HiDPI scaling.

### Mitigations

- **Trait abstraction:** A common `OverlayBackend` trait with per-platform
  implementations keeps the complexity contained.
- **Graceful fallback:** If Accessibility permission is denied or the
  platform backend fails, log a warning — don't crash.
- **Shell integration assists:** The ZLE widget can pass cursor coordinates
  to the daemon, reducing reliance on platform APIs for positioning.

## Alternatives Considered

### Inline ANSI Rendering (ADR-0004/0006)

The previous decision. Rejected because:

- Pushes content down, disrupts terminal context
- Flicker on redraw despite synchronized output
- Limited to terminal character grid
- No serious autocomplete tool uses this approach
- Feels like a terminal widget, not a native dropdown

### Ratatui with Inline Viewport (ADR-0005)

Ratatui's `Viewport::Inline` reserves lines below the cursor. Rejected for
the same reasons as raw ANSI — it's still inline terminal rendering.

### Cross-Platform Windowing (winit/tao)

**Update (2026-02-09):** winit 0.31 added `WindowAttributesMacOS::with_panel(true)`
which creates an NSPanel with `NonactivatingPanel` style mask. A spike
(`examples/overlay_winit.rs`) confirmed this works on macOS — the panel does not
steal focus from the terminal. winit 0.31 also changed `Window` from a struct to
a trait, returned as `Box<dyn Window>`.

winit remains unsuitable for non-focus-stealing overlays on X11 and Windows
(open issues, no platform-specific escape hatches). On macOS, `with_panel(true)`
makes winit a viable option, reducing the need for raw objc2 NSPanel code.
Platform-specific backends are still required for X11 (override-redirect) and
Wayland (layer-shell).

## Proof of Concept

Two spike examples validate the overlay approach:

### `examples/overlay_poc.rs` (raw objc2, macOS-only)

1. **NSPanel without focus stealing** — `NonactivatingPanel` style mask
2. **Accessibility API window query** — AXPosition + AXSize for the
   frontmost terminal
3. **TIOCGWINSZ for grid dimensions** — cell width/height from window
   size / terminal columns/rows
4. **Coordinate conversion** — AX screen coords (top-left origin) to
   Cocoa coords (bottom-left origin)
5. **Edge detection** — flip panel above cursor when it would go
   off-screen below

### `examples/overlay_winit.rs` (winit 0.31, cross-platform window creation)

1. **winit `with_panel(true)`** — creates NSPanel + NonactivatingPanel on macOS,
   falls back to regular window on other platforms
2. **softbuffer CPU rendering** — bitmap font rendering at 3x scale for HiDPI
3. **Arrow key navigation** — selection highlighting with redraw
4. **Non-focus-stealing confirmed** — clicking terminal keeps keyboard focus

## References

- [Fig.io architecture](../research/dropdown-rendering-architecture.md)
- Overlay POC (raw objc2): `examples/overlay_poc.rs`
- Overlay POC (winit 0.31): `examples/overlay_winit.rs`
- Cross-platform research: `research/overlay-window-cross-platform-2025.md`
- NSPanel documentation: Apple Developer
- wlr-layer-shell protocol: wayland.app
- x11rb: github.com/psychon/x11rb
