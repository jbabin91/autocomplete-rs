---
paths:
  - 'src/overlay/**'
  - 'docs/design/overlay.md'
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

## Module Structure

The overlay is implemented in `src/overlay/`:

- `mod.rs` — module facade, `OverlayMessage` enum (tagged for IPC)
- `app.rs` — winit `ApplicationHandler` (`OverlayApp`): window creation,
  message dispatch, keyboard navigation, rendering
- `renderer.rs` — pixel-buffer rendering of the completion dropdown
  (ARGB format for softbuffer). Platform-independent
- `font.rs` — bitmap 5×7 glyph data scaled 3× for HiDPI, `draw_char`/`draw_text`
- `positioning.rs` — pure coordinate math: cursor position from terminal
  geometry, overlay placement with edge detection + flip-above
- `backend.rs` — `OverlayBackend` trait, `OverlayPosition`, `PositioningError`
- `macos.rs` — macOS backend: Accessibility API window bounds + TIOCGWINSZ

Spike examples have been removed (superseded by the production module).
See git history for `examples/overlay_*.rs` if needed for reference.

## Event Loop Lifecycle

winit owns the main thread; Tokio runs on a background thread. They
communicate via `std::sync::mpsc` + `EventLoopProxy::wake_up()`.

- **`CloseRequested` must hide, never exit** — exiting the event loop while
  the daemon thread is still running causes a hang on `join()`. Only
  `OverlayMessage::Shutdown` (sent by the daemon) should call
  `event_loop.exit()`
- **Shutdown: daemon → overlay** via `OverlayMessage::Shutdown` (explicit)
- **Shutdown: daemon thread exit → overlay** via `proxy.wake_up()` after
  `run_daemon` returns. The overlay detects the dropped sender in
  `proxy_wake_up()` via `try_recv() == Disconnected` and calls
  `event_loop.exit()`. Without this wake, the event loop hangs forever
  because `try_iter()` on a disconnected channel returns empty (not an error)
- **Shutdown: overlay → daemon** is NOT automatic. Overlay exit drops the
  mpsc sender, but the daemon only logs send failures. Daemon shutdown
  must be triggered via its own cancellation token (see `y7s` bead)
- **No `expect()` in `ApplicationHandler` methods** — panics kill the entire
  daemon process. Use `tracing::error!` + `event_loop.exit()` for window/
  surface creation failures, `tracing::warn!` + early return for render errors

## Rendering

- **Guard public render functions** — `render_completions()` must handle
  `width == 0` and empty buffers (early return). Division by zero is
  reachable if callers pass degenerate dimensions
- **String width** — use `.chars().count()` for character-width calculations,
  not `.len()` (byte count). `.len()` breaks alignment for non-ASCII text
- **Buffer errors** — `surface.resize()`, `surface.buffer_mut()`, and
  `buf.present()` can all fail. Log and skip the frame, don't panic

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
