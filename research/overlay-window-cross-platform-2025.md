# Cross-Platform Overlay Window Research (2025-2026)

**Research Date:** 2026-02-08
**Purpose:** Evaluate Rust crate options for building a non-focus-stealing overlay window for terminal autocomplete dropdown

---

## Executive Summary

Building a cross-platform non-focus-stealing overlay window in Rust remains **challenging** in 2025-2026. No single crate provides out-of-the-box support for all platforms (macOS NSPanel-like, Linux X11 override-redirect, Linux Wayland layer-shell, Windows WS_EX_NOACTIVATE). Platform-specific implementations are required.

**Key Findings:**

1. **winit** and **tao** have ongoing issues with focus-stealing across platforms
2. **Wayland fundamentally prohibits querying window positions** by design (critical blocker)
3. Platform-specific approaches work: **x11rb** (X11), **wayland-protocols-wlr** (Wayland), **tao + NSPanel extensions** (macOS)
4. Terminal position querying is only reliable on X11; WSL/Wayland have severe limitations
5. **ratatui inline viewport** is the recommended approach for pure terminal-based dropdowns (no separate window)

---

## 1. Cross-Platform Windowing Crates

### 1.1 winit (rust-windowing/winit)

**Status:** Active development, widely used
**Latest Version:** 0.30.x (as of 2025)
**Repository:** <https://github.com/rust-windowing/winit>

**Platform Support:**

- Windows, macOS, Linux (X11/Wayland), iOS, Android, Redox OS

**Non-Focus-Stealing Issues:**

- **X11:** [No way to avoid focus stealing on X11 (#1160)](https://github.com/rust-windowing/winit/issues/1160) — Open issue since 2019
- **macOS:** [Cannot create unfocused window (#3072)](https://github.com/rust-windowing/winit/issues/3072) — `with_active(false)` doesn't prevent focus stealing
- **Windows:** Partial support via extended window styles, but not exposed directly in winit API

**Key Limitations:**

- No `WindowBuilder` option to disable focus on mapping
- New windows appear on top and steal focus even with `with_active(false)`
- Platform-specific extensions exist but are limited

**Platform Extensions:**

- [`WindowBuilderExtMacOS`](https://docs.rs/winit/latest/x86_64-apple-darwin/winit/platform/macos/trait.WindowBuilderExtMacOS.html) — macOS-specific methods (titlebar customization, etc.)
- No direct NSPanel support
- No direct override-redirect or WS_EX_NOACTIVATE support

**Verdict:** ❌ **Not suitable** for overlay dropdowns without significant platform-specific workarounds

---

### 1.2 tao (tauri-apps/tao)

**Status:** Active, maintained by Tauri team
**Latest Version:** Check [crates.io/crates/tao](https://crates.io/crates/tao)
**Repository:** <https://github.com/tauri-apps/tao>

**Platform Support:**

- Windows, macOS, Linux (X11/Wayland), iOS, Android
- Built for Tauri, fork of winit with additional features

**NSPanel Support on macOS:**

- **Third-party plugin:** [tauri-nspanel](https://github.com/ahkohd/tauri-nspanel) — Converts Tauri windows to macOS NSPanel
- **Open issue:** [NSPanel behavior needed (#414)](https://github.com/tauri-apps/tao/issues/414) — Discussion about adding NSPanel support directly to tao
- Approach: Subclass `NSWindow` with `NSPanel` or add platform-specific builder field

**Windows Support:**

- Windows API constants like `WS_EX_NOACTIVATE` are available via [`windows` crate](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/UI/WindowsAndMessaging/constant.WS_EX_NOACTIVATE.html)
- Not directly exposed in tao API — requires platform-specific extensions

**Verdict:** ⚠️ **Partial support** via tauri-nspanel on macOS; Windows/Linux require manual platform code

---

### 1.3 glutin (rust-windowing/glutin)

**Status:** Active, OpenGL context creation library
**Repository:** <https://github.com/rust-windowing/glutin>

**Features:**

- Transparent windows via `with_transparent(true)` on `WindowAttributes`
- Built on top of winit — inherits winit's focus-stealing issues

**Verdict:** ❌ **Not suitable** for overlay windows (OpenGL context focus, same issues as winit)

---

## 2. Wayland Overlay Approach

### 2.1 Layer-Shell Protocol (`wlr-layer-shell`)

**Protocol:** [wlr-layer-shell-unstable-v1](https://wayland.app/protocols/wlr-layer-shell-unstable-v1)
**Purpose:** Create surfaces in "layers" of the desktop (z-depth), suitable for desktop shell components (panels, docks, notifications)

**Rust Crate: wayland-protocols / wayland-protocols-wlr**

- **Crate:** [wayland-protocols](https://crates.io/crates/wayland-protocols) (includes wlr protocols)
- **Latest Versions:** 0.28.x, 0.32.x
- **Struct:** [`ZwlrLayerShellV1`](https://smithay.github.io/wayland-rs/wayland_protocols_wlr/layer_shell/v1/client/zwlr_layer_shell_v1/struct.ZwlrLayerShellV1.html)
- **Module:** `wayland_protocols::wlr::unstable::layer_shell::v1::client`

**Example Usage:**

```rust
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::ZwlrLayerShellV1;
```

**Layer Enum:**

```rust
pub enum Layer {
    Background,
    Bottom,
    Top,
    Overlay,
}
```

Use `Layer::Overlay` for overlay dropdowns that should appear above regular windows.

**Integration with smithay-client-toolkit:**

- **Crate:** [smithay-client-toolkit](https://github.com/Smithay/client-toolkit)
- **Purpose:** Toolkit for writing Wayland clients, built on `wayland-client`
- Provides abstractions for window creation (toplevels), keyboard handling, etc.
- Layer-shell support available in recent versions

**Real-World Examples:**

- **rofi-wayland** — [PR #1139](https://github.com/davatorium/rofi/pull/1139) added Wayland support via layer-shell
- **wofi** — [Rofi-inspired launcher](https://manpages.ubuntu.com/manpages/jammy/man1/wofi.1.html) using wlr-layer-shell by default
- Both use layer-shell protocol for overlay menus on Wayland compositors (sway, Hyprland, etc.)

**Verdict:** ✅ **Recommended** for Wayland overlays

---

### 2.2 Wayland Window Position Query Limitations

**Critical Design Limitation:**

Wayland was **explicitly designed** to prohibit clients from introspecting or programmatically changing global window coordinates. This is an **intentional limitation** with no workarounds.

**Sources:**

- [Wayland's Never-Ending Opposition To Multi-Window Positioning](https://hackaday.com/2025/11/11/waylands-never-ending-opposition-to-multi-window-positioning/)
- [Blender Issue: Unable to access window position](https://developer.blender.org/T98928)
- [SDL Issue: Wayland not possible to set window position (#7197)](https://github.com/libsdl-org/SDL/issues/7197)

**Quote:**

> "Wayland can't access window positions, and this is an intentional limitation with no ways to access this information. The compositor is the sole authority on window placement."

**Impact on Terminal Overlay:**

- **Cannot query terminal emulator's window position** on Wayland
- **Cannot position overlay relative to terminal window**
- **Must rely on compositor-provided positioning** (layer-shell anchoring)

**Workaround (Compositor-Specific):**

Some compositors like **sway** expose IPC interfaces (i3-compatible) that allow querying window tree, bypassing protocol-level restrictions. This is **compositor-specific** and not portable.

**Verdict:** 🚫 **Window position querying is impossible** on pure Wayland — use layer-shell anchoring instead

---

## 3. X11 Overlay Approach

### 3.1 x11rb (X11 Rust Bindings)

**Crate:** [x11rb](https://crates.io/crates/x11rb)
**Repository:** <https://github.com/psychon/x11rb>
**Latest Version:** Check [docs.rs/x11rb](https://docs.rs/x11rb)

**Description:**

Pure-Rust X11 bindings, similar to XCB (X11 C Bindings). Provides low-level access to X11 protocol.

**Override-Redirect Windows:**

Set the `override-redirect` attribute to `true` to bypass window manager mapping mechanism.

**Example Code:**

```rust
use x11rb::protocol::xproto::*;

// Create window with override-redirect
let window = conn.generate_id()?;
let values = CreateWindowAux::new()
    .override_redirect(1)  // Bypass WM
    .event_mask(EventMask::EXPOSURE | EventMask::KEY_PRESS);

conn.create_window(
    COPY_FROM_PARENT as u8,
    window,
    parent_window,
    x, y, width, height,
    border_width,
    WindowClass::INPUT_OUTPUT,
    visual_id,
    &values,
)?;

conn.map_window(window)?;
conn.flush()?;
```

**Composite Overlay Window:**

[`get_overlay_window`](https://docs.rs/x11rb/latest/x11rb/protocol/composite/fn.get_overlay_window.html) — For compositor-managed overlays

**Examples:**

- [tutorial.rs](https://github.com/psychon/x11rb/blob/master/x11rb/examples/tutorial.rs)
- [simple_window_manager.rs](https://github.com/psychon/x11rb/blob/master/x11rb/examples/simple_window_manager.rs)

**Verdict:** ✅ **Recommended** for X11 overlays

---

### 3.2 xcb (X11 Bindings via XCB)

**Crate:** [xcb](https://crates.io/crates/xcb)
**Repository:** <https://github.com/rust-x-bindings/rust-xcb>
**Documentation:** [docs.rs/xcb](https://docs.rs/xcb)

**Description:**

Rust bindings to XCB (X C Binding), wraps the core XCB functions to connect and communicate with X server.

**Usage:**

```rust
use xcb::xproto::*;

// Set override-redirect via window attributes
let values = CreateWindowAux::new()
    .override_redirect(1);
```

**Comparison with x11rb:**

- **x11rb:** Pure Rust, more modern, better ergonomics
- **xcb:** FFI to C library, more mature, closer to C API

**Verdict:** ⚠️ **Alternative to x11rb**, but x11rb is preferred for pure Rust

---

### 3.3 X11 Window Position Querying

**Crates:**

- **active-win-pos-rs** — [crates.io](https://crates.io/crates/active-win-pos-rs)
- **x11_get_windows** — [GitHub](https://github.com/HiruNya/x11_get_windows)

**active-win-pos-rs:**

- **Version:** 0.9.0 (latest as of 2025)
- **Platforms:** Windows, macOS, Linux (X11)
- **Features:**
  - Get position, size, title of active window
  - Returns `ActiveWindow { window_id, process_id, position: WindowPosition { x, y, width, height } }`

**X11 Implementation:**

Uses `_NET_ACTIVE_WINDOW` and `XGetWindowAttributes`:

```rust
// Pseudo-code
let active_window = query_property("_NET_ACTIVE_WINDOW");
let attrs = XGetWindowAttributes(display, active_window);
let position = (attrs.x, attrs.y, attrs.width, attrs.height);
```

**Verdict:** ✅ **Works reliably on X11** for querying terminal window position

---

## 4. macOS Overlay Approach (NSPanel)

### 4.1 NSPanel via tao + tauri-nspanel

**Plugin:** [tauri-nspanel](https://github.com/ahkohd/tauri-nspanel)
**Description:** Create macOS panels (NSPanel) for Tauri apps

**Features:**

- Converts regular Tauri window to NSPanel
- Panels float above other windows
- Auxiliary controls, tool palettes, inspectors, HUD displays

**Usage:**

```rust
// Platform-specific macOS extension
use tao::platform::macos::WindowBuilderExtMacOS;

let window = WindowBuilder::new()
    // Custom macOS panel configuration
    .build(&event_loop)?;
```

**NSPanel Properties:**

- Shares all methods of `NSWindow` (no breaking changes)
- Three additional NSPanel-specific methods
- Non-activating behavior (doesn't steal focus)

**Verdict:** ✅ **Recommended** for macOS overlays via Tauri ecosystem

---

### 4.2 Direct Cocoa/Objective-C Bindings

**Crate:** [cocoa](https://crates.io/crates/cocoa) or [objc2](https://crates.io/crates/objc2)

**Manual NSPanel Creation:**

```rust
use cocoa::appkit::NSPanel;
use cocoa::base::{id, nil};

unsafe {
    let panel: id = NSPanel::alloc(nil);
    let panel = panel.initWithContentRect_styleMask_backing_defer_(
        rect,
        NSNonactivatingPanelMask,
        NSBackingStoreBuffered,
        false,
    );
    panel.setFloatingPanel_(true);
    panel.setHidesOnDeactivate_(false);
}
```

**Verdict:** ⚠️ **Advanced** — requires unsafe code, Objective-C knowledge

---

## 5. Windows Overlay Approach

### 5.1 WS_EX_NOACTIVATE via windows-rs

**Crate:** [windows](https://crates.io/crates/windows)
**Repository:** <https://github.com/microsoft/windows-rs>

**Constant:** [`WS_EX_NOACTIVATE`](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/UI/WindowsAndMessaging/constant.WS_EX_NOACTIVATE.html)

**Description:**

Extended window style that prevents the window from becoming the foreground window when the user clicks it.

**Usage:**

```rust
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, WS_EX_NOACTIVATE, WS_EX_LAYERED, WS_EX_TOOLWINDOW
};

let hwnd = CreateWindowExW(
    WS_EX_NOACTIVATE | WS_EX_LAYERED | WS_EX_TOOLWINDOW,
    class_name,
    window_name,
    WS_POPUP,
    x, y, width, height,
    None, None, hinstance, None,
);
```

**Additional Flags:**

- `WS_EX_TOOLWINDOW` — Remove from taskbar and Alt-Tab switcher
- `WS_EX_LAYERED` — Enable transparency (for click-through overlays)

**Verdict:** ✅ **Recommended** for Windows overlays

---

### 5.2 WSL X11 Window Position Querying

**Issue:** [gtk_window_get_position returns always (0, 0) in X11 mode (#355)](https://github.com/microsoft/wslg/issues/355)

**Status:** Known bug in WSLg (WSL GUI support)

**Impact:**

- Window position queries return `(0, 0)` instead of actual coordinates
- Affects X11 apps running in WSL
- May require workarounds or alternative positioning strategies

**Verdict:** ⚠️ **Limited reliability** on WSL — test thoroughly

---

## 6. Window Position Querying Across Platforms

### Summary Table

| Platform          | Method                                        | Reliability            | Crate                        |
| ----------------- | --------------------------------------------- | ---------------------- | ---------------------------- |
| **Linux X11**     | `_NET_ACTIVE_WINDOW` + `XGetWindowAttributes` | ✅ Reliable            | `active-win-pos-rs`, `x11rb` |
| **Linux Wayland** | _Impossible by design_                        | ❌ Not possible        | N/A                          |
| **macOS**         | Accessibility API                             | ✅ Reliable            | `active-win-pos-rs`          |
| **Windows**       | Win32 API                                     | ✅ Reliable            | `active-win-pos-rs`          |
| **WSL X11**       | `XGetWindowAttributes`                        | ⚠️ Buggy (returns 0,0) | `x11rb` (limited)            |

**Recommendation:**

Use **active-win-pos-rs** (v0.9.0) for cross-platform window querying where supported. **Fallback to terminal-relative positioning** on Wayland.

---

## 7. Terminal Size Detection

### 7.1 terminal_size Crate

**Crate:** [terminal_size](https://crates.io/crates/terminal_size)
**Description:** Gets the size of your Linux or Windows terminal

**Platform Support:**

- Linux, macOS, Windows, illumos

**Usage:**

```rust
use terminal_size::{Width, Height, terminal_size};

if let Some((Width(w), Height(h))) = terminal_size() {
    println!("Terminal size: {}x{}", w, h);
}
```

**Under the Hood:**

- Linux/macOS: Uses `libc::ioctl(TIOCGWINSZ)`
- Windows: Uses Windows Console API

**Comparison with Raw TIOCGWINSZ:**

- **terminal_size:** Cross-platform abstraction, handles platform differences
- **Raw `libc::ioctl(TIOCGWINSZ)`:** Linux/Unix only, more control

**Verdict:** ✅ **Recommended** — Better than raw `ioctl` for cross-platform support

---

### 7.2 crossterm Terminal Module

**Crate:** [crossterm](https://crates.io/crates/crossterm)
**Repository:** <https://github.com/crossterm-rs/crossterm>

**Features:**

- Cross-platform terminal manipulation (ANSI escape codes)
- Supports all UNIX and Windows terminals (Windows 7+)

**Terminal Size:**

```rust
use crossterm::terminal;

let (width, height) = terminal::size()?;
```

**Verdict:** ✅ **Alternative** to terminal_size, especially if already using crossterm

---

## 8. Real-World Examples

### 8.1 Fig.io / Amazon Q CLI

**Architecture:**

- **Core:** Node.js server + Rust client
- **Autocomplete Specs:** TypeScript (community-editable)
- **Open Source:** [autocomplete specs](https://github.com/withfig/autocomplete), not the overlay implementation

**Overlay Implementation:**

- Not open source — proprietary
- Likely uses platform-specific native windows (NSPanel on macOS, etc.)

**References:**

- [Autocomplete Deep Dive](https://www.blog.brightcoding.dev/2025/09/10/autocomplete-for-terminal-commands-a-deep-dive-into-figs-open-source-engine/)
- [Fig Acquisition by Amazon](https://fig.io/)

**Verdict:** 🔒 **Closed source** — no Rust overlay implementation available

---

### 8.2 Kiro CLI (Successor to Fig/Amazon Q)

**Status:** Active as of November 2025
**Documentation:** <https://kiro.dev/docs/cli/autocomplete/>

**Autocomplete Features:**

- **Dropdown Menu:** Appears to the right of cursor, shows available options
- **Inline Suggestions:** Gray "ghost text"

**Architecture:**

- Autocomplete specs in TypeScript (`@fig/autocomplete-tools`)
- Respects legacy Fig path: `~/.fig/autocomplete/build/<cli-name>.js`
- Visual theme customization via `kiro-cli settings`

**Overlay Implementation:**

- No technical details available in public docs
- Likely inherits Fig's approach (platform-specific native windows)

**Verdict:** 🔒 **Implementation details not public**

---

### 8.3 Warp Terminal

**Status:** Active, open-sourced UI framework planned
**Repository:** <https://github.com/warpdotdev/Warp>

**Architecture:**

- **Core:** Rust (98% code shared between macOS/Linux)
- **Rendering:** GPU-accelerated (Metal on macOS, wgpu on Linux)
- **UI Framework:** Custom Rust framework (inspired by Flutter), built by Nathan Sobo (Atom co-founder)
- **Performance:** >144 FPS, 1.9ms average screen redraw

**Technology Stack:**

- **wgpu** — Cross-platform GPU API
- **winit** — Window management
- **cosmic-text** — Text rendering (from System76)

**Overlay Approach:**

- **Built-in terminal** — Overlays are rendered within the terminal app itself, not separate windows
- No separate overlay window needed

**References:**

- [How Warp Works](https://www.warp.dev/blog/how-warp-works)
- [Warp Review](https://thenewstack.io/a-review-of-warp-another-rust-based-terminal/)

**Verdict:** 📚 **Educational** — Custom UI framework, but integrated approach (not separate overlay window)

---

### 8.4 Alacritty

**Repository:** <https://github.com/alacritty/alacritty>
**Crates:** `alacritty`, `alacritty_terminal`

**Architecture:**

- **Crate Separation:** `alacritty` (GUI/window), `alacritty_terminal` (terminal emulation core)
- **Rendering:** OpenGL (GPU-accelerated)
- **Windowing:** winit

**Overlay Support:**

- No built-in autocomplete overlay
- Terminal emulation only

**Verdict:** 📚 **Educational** — Good reference for terminal architecture, but no overlay implementation

---

## 9. Inline Terminal Dropdown (Alternative Approach)

### 9.1 ratatui Inline Viewport

**Crate:** [ratatui](https://crates.io/crates/ratatui)
**Repository:** <https://github.com/ratatui/ratatui>
**Version:** 0.30.0+ (split into multiple crates: ratatui-core, ratatui-widgets)

**Viewport Types:**

```rust
pub enum Viewport {
    Fullscreen,
    Inline(u16),  // Reserve N lines at bottom of terminal
    Fixed(Rect),
}
```

**Inline Viewport Example:**

```rust
use ratatui::{Terminal, backend::CrosstermBackend, Viewport};
use crossterm::terminal;

let backend = CrosstermBackend::new(std::io::stdout());
let mut terminal = Terminal::with_options(
    backend,
    TerminalOptions {
        viewport: Viewport::Inline(5),  // Reserve 5 lines
    },
)?;
```

**How It Works:**

- Reserves a rectangular section at the bottom of the terminal
- Does **not** use alternate screen buffer
- Automatically resizes internal buffers on terminal resize
- Content renders inline with shell output

**References:**

- [Inline Viewport Example](https://ratatui.rs/examples/apps/inline/)
- [Viewport Docs](https://docs.rs/ratatui/latest/ratatui/enum.Viewport.html)

**Verdict:** ✅ **HIGHLY RECOMMENDED** — Avoids all window positioning issues, works on all platforms

---

### 9.2 Raw ANSI Escape Codes (crossterm)

**Alternative to ratatui:**

Use **crossterm** for raw terminal manipulation without TUI framework overhead.

**Example:**

```rust
use crossterm::{
    cursor,
    terminal::{self, ClearType},
    style::{self, Stylize},
    execute,
};
use std::io::{stdout, Write};

// Save cursor position
execute!(stdout(), cursor::SavePosition)?;

// Move to dropdown position (below current line)
execute!(stdout(), cursor::MoveToNextLine(1))?;

// Render dropdown items
for item in items {
    println!("{}", item.on_dark_grey());
}

// Restore cursor
execute!(stdout(), cursor::RestorePosition)?;
```

**Pros:**

- No dependencies (besides crossterm)
- Full control over rendering
- Works everywhere (no window manager issues)

**Cons:**

- Manual state management
- No built-in widgets
- More complex layout logic

**Verdict:** ⚠️ **Advanced** — Use if ratatui is too heavyweight

---

## 10. Recommended Architecture by Platform

### Option A: Separate Overlay Window (Platform-Specific)

| Platform          | Crate                   | Approach                        |
| ----------------- | ----------------------- | ------------------------------- |
| **macOS**         | `tao` + `tauri-nspanel` | NSPanel overlay                 |
| **Linux X11**     | `x11rb`                 | Override-redirect window        |
| **Linux Wayland** | `wayland-protocols-wlr` | Layer-shell (top/overlay layer) |
| **Windows**       | `windows` crate         | `WS_EX_NOACTIVATE` window       |

**Position Querying:**

- **X11/macOS/Windows:** `active-win-pos-rs` (v0.9.0)
- **Wayland:** Not possible — use compositor anchoring

**Rendering:**

- **softbuffer** (CPU rendering) or **wgpu** (GPU rendering)
- **ratatui** for TUI widgets (if needed)

**Complexity:** 🔴 **High** — Requires platform-specific code for each OS

---

### Option B: Inline Terminal Dropdown (Cross-Platform)

| Component              | Crate                                            |
| ---------------------- | ------------------------------------------------ |
| **Terminal Rendering** | `ratatui` with `Viewport::Inline`                |
| **Backend**            | `crossterm`                                      |
| **Terminal Size**      | `terminal_size` or `crossterm::terminal::size()` |

**Approach:**

1. Capture shell buffer + cursor position (via ZLE widget)
2. Reserve N lines at bottom of terminal using inline viewport
3. Render dropdown items with ratatui widgets
4. Handle keyboard input (arrow keys, Enter, Esc)
5. Insert selected completion into buffer

**Complexity:** 🟢 **Low** — Works on all platforms without window management

**Recommendation:** ✅ **Start here** — Defer separate overlay window to later iteration

---

## 11. Concrete Recommendations

### Phase 1: Inline Terminal Dropdown (MVP)

**Stack:**

- `ratatui` (v0.30.0+) with `Viewport::Inline`
- `crossterm` (v0.28.x) for terminal backend
- `terminal_size` (latest) for terminal dimensions

**Why:**

- ✅ Works on all platforms (macOS, Linux X11/Wayland, Windows, WSL)
- ✅ No window positioning issues
- ✅ No focus-stealing concerns
- ✅ Simple architecture, faster to implement
- ✅ Matches AGENTS.md directive: "render completions inline below the cursor using raw ANSI escape codes via crossterm"

**Limitations:**

- Cannot render outside terminal bounds
- May scroll terminal if near bottom
- Less "polished" than floating window (but acceptable for autocomplete)

---

### Phase 2: Separate Overlay Window (Future)

**If** inline dropdown proves limiting, implement platform-specific overlay:

**Stack:**

- **macOS:** `tao` + `tauri-nspanel` (NSPanel)
- **Linux X11:** `x11rb` (override-redirect)
- **Linux Wayland:** `wayland-protocols-wlr` (layer-shell)
- **Windows:** `windows` crate (`WS_EX_NOACTIVATE`)
- **Position Querying:** `active-win-pos-rs` (X11/macOS/Windows), compositor anchoring (Wayland)
- **Rendering:** `wgpu` (GPU) or `softbuffer` (CPU)

**Why Defer:**

- ❌ High complexity (4 platform-specific implementations)
- ❌ Wayland cannot query terminal position (architectural blocker)
- ❌ WSL position querying is buggy
- ❌ winit/tao focus-stealing issues unresolved

---

## 12. Open Questions for Follow-Up

1. **Wayland terminal position:** Can we query terminal PID and use compositor-specific IPC (sway only)?
2. **WSL window position bug:** Has Microsoft fixed the `gtk_window_get_position` bug in WSLg?
3. **tao NSPanel integration:** Will tao merge NSPanel support directly (issue #414)?
4. **ratatui dropdown widget:** Is there a community dropdown widget, or do we build custom?
5. **Performance:** How does inline ratatui rendering perform with 1000+ completion items?

---

## 13. Sources

### Windowing Libraries

- [winit: No way to avoid focus stealing on X11 (#1160)](https://github.com/rust-windowing/winit/issues/1160)
- [winit: macOS unfocused window (#3072)](https://github.com/rust-windowing/winit/issues/3072)
- [tao: NSPanel behavior needed (#414)](https://github.com/tauri-apps/tao/issues/414)
- [tauri-nspanel plugin](https://github.com/ahkohd/tauri-nspanel)
- [glutin: Transparent windows](https://github.com/rust-windowing/glutin)
- [winit WindowBuilderExtMacOS](https://docs.rs/winit/latest/x86_64-apple-darwin/winit/platform/macos/trait.WindowBuilderExtMacOS.html)

### Wayland

- [wlr-layer-shell protocol](https://wayland.app/protocols/wlr-layer-shell-unstable-v1)
- [wayland-protocols-wlr crate](https://lib.rs/crates/wayland-protocols-wlr)
- [smithay-client-toolkit](https://github.com/Smithay/client-toolkit)
- [smithay compositor framework](https://github.com/Smithay/smithay)
- [rofi-wayland layer-shell PR (#1139)](https://github.com/davatorium/rofi/pull/1139)
- [wofi manual](https://manpages.ubuntu.com/manpages/jammy/man1/wofi.1.html)
- [Wayland's Never-Ending Opposition to Window Positioning](https://hackaday.com/2025/11/11/waylands-never-ending-opposition-to-multi-window-positioning/)
- [Blender: Wayland window position issue](https://developer.blender.org/T98928)
- [SDL: Wayland window position (#7197)](https://github.com/libsdl-org/SDL/issues/7197)

### X11

- [x11rb repository](https://github.com/psychon/x11rb)
- [x11rb docs](https://docs.rs/x11rb)
- [xcb crate](https://crates.io/crates/xcb)
- [active-win-pos-rs](https://crates.io/crates/active-win-pos-rs)
- [x11_get_windows](https://github.com/HiruNya/x11_get_windows)
- [XGetWindowAttributes docs](https://docs.rs/x11/latest/x11/xlib/fn.XGetWindowAttributes.html)

### Windows

- [WS_EX_NOACTIVATE](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/UI/WindowsAndMessaging/constant.WS_EX_NOACTIVATE.html)
- [WSLg gtk_window_get_position bug (#355)](https://github.com/microsoft/wslg/issues/355)

### Terminal Tools

- [terminal_size crate](https://crates.io/crates/terminal_size)
- [crossterm repository](https://github.com/crossterm-rs/crossterm)
- [ratatui](https://ratatui.rs/)
- [ratatui inline viewport example](https://ratatui.rs/examples/apps/inline/)
- [ratatui viewport docs](https://docs.rs/ratatui/latest/ratatui/enum.Viewport.html)

### Real-World Examples

- [Fig autocomplete specs](https://github.com/withfig/autocomplete)
- [Kiro CLI autocomplete docs](https://kiro.dev/docs/cli/autocomplete/)
- [Warp terminal](https://github.com/warpdotdev/Warp)
- [Warp: How It Works](https://www.warp.dev/blog/how-warp-works)
- [Alacritty](https://github.com/alacritty/alacritty)

### Rendering

- [softbuffer](https://github.com/rust-windowing/softbuffer)
- [wgpu](https://wgpu.rs/)

---

## Conclusion

**For autocomplete-rs, the recommended path is:**

1. **Phase 1 (MVP):** Implement inline dropdown using `ratatui` + `crossterm` with `Viewport::Inline`
   - Avoids all platform-specific windowing complexity
   - Works reliably on all platforms (X11, Wayland, macOS, Windows, WSL)
   - Aligns with AGENTS.md directive

2. **Phase 2 (Future):** Evaluate separate overlay window if inline approach proves limiting
   - macOS: `tao` + `tauri-nspanel`
   - Linux X11: `x11rb` override-redirect
   - Linux Wayland: `wayland-protocols-wlr` layer-shell
   - Windows: `windows` crate with `WS_EX_NOACTIVATE`

**Critical Blocker:** Wayland's inability to query window positions makes terminal-relative overlay positioning fundamentally impossible on Wayland without compositor-specific hacks. Inline rendering sidesteps this entirely.
