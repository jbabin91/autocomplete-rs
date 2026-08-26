# Overlay Dropdown — Design Spec

This document details the planned design for the native overlay completion
dropdown.

**Status:** macOS MVP implemented in `src/overlay/`. The daemon runs the
overlay on the main thread (winit event loop) with Tokio on a background
thread. Completions are forwarded to the overlay via `OverlayChannel`
(mpsc + `EventLoopProxy::wake_up()`). The previous inline ANSI approach
has been superseded — see [ADR-0008](../adr/0008-native-overlay-dropdown.md).
Linux and Windows backends are follow-up work.

## Overview

The overlay dropdown renders completions in a native floating window positioned
at the terminal cursor. Unlike inline ANSI rendering (which pushes terminal
content down), the overlay floats above existing content — matching Fig.io's
original UX.

## Design Principles

1. **Native overlay:** Floating window, not terminal text
2. **Fast:** <10ms render time target
3. **Non-focus-stealing:** Terminal keeps keyboard focus at all times
4. **Edge-aware:** Flip above cursor when near screen bottom
5. **Platform-specific:** Per-platform backends behind a common trait

## Architecture

```text
┌─────────────────────────────────┐
│  OverlayBackend trait           │
│  - show(suggestions, position)  │
│  - hide()                       │
│  - update_selection(index)      │
│  - reposition(row, col)         │
└─────────────────────────────────┘
        │
        ├── MacOSBackend (NSPanel + Accessibility API)
        ├── X11Backend (override-redirect + x11rb)
        ├── WaylandBackend (layer-shell + smithay)
        └── WindowsBackend (WS_EX_NOACTIVATE + Win32)
```

## Positioning Pipeline

Per [ADR-0009](../adr/0009-dsr-cursor-positioning.md), the cursor cell will come
from the terminal itself (DSR `ESC[6n`, queried by the shell widget); the
pipeline below supplies the window rect on every platform and is the full
fallback when a terminal ignores DSR.

1. **Query terminal window bounds** — platform-specific (Accessibility API on
   macOS, X11 window attributes on Linux, Win32 on Windows)
2. **Get terminal grid dimensions** — `TIOCGWINSZ` ioctl (or Windows console API)
3. **Compute cell pixel position** — `cell_width = window_width / cols`,
   `cursor_x = window_x + col * cell_width`
4. **Convert coordinate systems** — AX screen coords → Cocoa coords on macOS
5. **Edge detection** — flip above if panel would go off-screen below;
   shift left if off-screen right

## Visual Design

- Dark background with slight transparency (0.95 alpha)
- Shadow for depth
- White text on dark background
- Highlighted selected item
- Max 5-8 visible items, scrollable

## Key Bindings

- `Esc` → dismiss, no selection
- `Enter` → accept selected suggestion
- `↓` / `↑` → navigate suggestions (wrap around)
- Typing → filter suggestions (future)

## Platform Notes

### macOS

- `NSPanel` with `NonactivatingPanel` style — prevents focus stealing
- `NSFloatingWindowLevel` (level 3) — always on top
- `CanJoinAllSpaces | FullScreenAuxiliary` — works across Spaces
- `setHidesOnDeactivate(false)` — stays visible when app deactivates
- Accessibility API for window position (requires permission grant)

### Linux X11

- Override-redirect window bypasses window manager
- `_NET_ACTIVE_WINDOW` + `XGetWindowAttributes` for terminal position
- `x11rb` crate for X11 protocol

### Linux Wayland

- `wlr-layer-shell` protocol for overlay rendering
- Cannot query other windows' positions (Wayland design limitation)
- Shell integration must report cursor pixel coordinates

### Windows

- `WS_EX_NOACTIVATE` extended style — non-activating window
- `Win32 GetWindowRect` for terminal position
- `windows` crate for Win32 API

## Limitation: Remote Shells

A native overlay can only track a terminal on the local machine. In an SSH
session the shell — and therefore the widget and daemon — runs on the remote
host, where there is no display to draw on. In-grid tools (PTY proxies, ANSI
popups) work over SSH because their UI travels inside the terminal byte stream;
ours does not. This is a deliberate trade documented in
[fig-successor-cohort.md](../research/fig-successor-cohort.md) §6.1: a richer
local UI that never sits in the terminal's I/O path, at the cost of the overlay
in remote sessions. The planned degradation is in-ZLE rendering — ghost text
via `POSTDISPLAY` first — never a PTY proxy or wrapper. Selecting it needs a
remote-session check that survives multiplexers: `$SSH_TTY` is absent from
tmux's default `update-environment` list, so a pane in a remote tmux may not
carry it and would wrongly read as local. Local tmux is unaffected in principle
(the terminal and cursor are local), but both the detection and the positioning
inside multiplexers need their own testing.
