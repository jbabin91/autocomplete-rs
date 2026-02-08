# Dropdown Rendering Architecture Research

> **Date:** 2026-02-08
> **Status:** Complete
> **Motivation:** The existing research and ADR-0006 assumed ANSI inline rendering
> would replicate Fig.io's UX. User feedback identified that Fig actually used a
> **native GUI overlay window** floating above the terminal, not in-band terminal
> text. This document captures a deep dive into how Fig and similar tools actually
> rendered their autocomplete dropdowns.

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [How Fig.io Actually Worked](#2-how-figio-actually-worked)
3. [Kiro CLI (Current Successor)](#3-kiro-cli-current-successor)
4. [Other Tools Comparison](#4-other-tools-comparison)
5. [Cursor Position Tracking](#5-cursor-position-tracking)
6. [Platform Feasibility](#6-platform-feasibility)
7. [Terminal Extension APIs](#7-terminal-extension-apis)
8. [Rust Implementation Options](#8-rust-implementation-options)
9. [Architecture Paths for autocomplete-rs](#9-architecture-paths-for-autocomplete-rs)
10. [Recommendation](#10-recommendation)

---

## 1. Executive Summary

There are two fundamentally different approaches to rendering an autocomplete
dropdown in a terminal:

**Approach A: Native GUI Overlay** — A separate OS window (borderless,
transparent, always-on-top) floats above the terminal, positioned at the
cursor's screen-space pixel coordinates. The dropdown is rendered using web
technologies (React in a WebView) or native UI frameworks. This is what
**Fig.io, Amazon Q CLI, and Kiro CLI** do.

**Approach B: In-Band Terminal Text** — Completions are rendered as styled text
within the terminal itself using ANSI escape codes. The dropdown is "fake" — it
is terminal text that looks like a dropdown but is actually part of the terminal
output. This is what **fish shell, inshellisense, Atuin, and Nushell/Reedline**
do.

**Key finding:** No terminal emulator provides an API for external programs to
create positioned overlay windows. This means Approach A requires OS-level APIs
(Accessibility, IME) to track the cursor's pixel position — APIs that are
platform-specific, fragile, and require user permissions.

### Quick Comparison

| Aspect          | Native GUI Overlay (Fig)               | In-Band ANSI (fish-style) |
| --------------- | -------------------------------------- | ------------------------- |
| Visual quality  | Rich (HTML/CSS, shadows, transparency) | Terminal cells only       |
| Cross-platform  | Hard (OS-specific cursor tracking)     | Easy (any ANSI terminal)  |
| Terminal compat | Requires per-terminal cursor tracking  | Universal                 |
| tmux/screen     | Broken (overlay can't see inside mux)  | Works transparently       |
| SSH sessions    | Broken (no GUI on remote)              | Works transparently       |
| Permissions     | Accessibility + IME permissions        | None                      |
| Binary size     | Large (webview runtime)                | Minimal                   |
| Latency         | IPC + webview render                   | Direct terminal writes    |

---

## 2. How Fig.io Actually Worked

### 2.1 Architecture Overview

Fig was a **multi-process system** with five major components:

```text
┌─────────────────────────────────────────────────────┐
│  Terminal Emulator (iTerm2, Kitty, etc.)             │
│                                                       │
│  Shell (zsh/bash/fish)                               │
│    └── figterm (Rust PTY proxy)                      │
│          - Intercepts terminal I/O                    │
│          - Injects invisible OSC markers into prompt  │
│          - Reconstructs edit buffer from screen model │
│          - Sends buffer to fig_desktop via IPC        │
│                                                       │
│  ┌─ fig_desktop (Rust: tao + wry) ─────────────┐    │
│  │  Borderless NSWindow, always-on-top          │    │
│  │  ┌── WKWebView ──────────────────────────┐   │    │
│  │  │   React 18 app (autocomplete UI)      │   │    │
│  │  │   - Tailwind CSS styling              │   │    │
│  │  │   - react-window (virtualized list)   │   │    │
│  │  │   - zustand state management          │   │    │
│  │  └───────────────────────────────────────┘   │    │
│  └──────────────────────────────────────────────┘    │
│                                                       │
│  fig_input_method (macOS IME, bundle: io.fig.cursor) │
│  Shell integration (ZLE widget / readline hooks)      │
└───────────────────────────────────────────────────────┘

IPC: Protocol Buffers over Unix Domain Sockets
  - fig.proto:     React app ←→ fig_desktop
  - figterm.proto: CLI ←→ figterm
  - remote.proto:  figterm ←→ fig_desktop
```

### 2.2 The Overlay Window

Fig's dropdown was **NOT Electron**. It was a native macOS application:

- **Original (Fig era):** Swift/Objective-C app using `NSPanel` (subclass of
  `NSWindow`) with `WKWebView` for rendering
- **Post-Amazon acquisition:** Rewritten in Rust using `tao` (cross-platform
  windowing, from Tauri ecosystem) + `wry` (cross-platform WebView wrapper)

The window was configured as:

- `styleMask: [.borderless]` — no title bar or chrome
- `isOpaque = false`, `backgroundColor = .clear` — transparent background
- `level = .popUpMenu` (101) — floats above normal windows
- `ignoresMouseEvents` — toggled for click-through behavior
- `collectionBehavior = [.canJoinAllSpaces]` — follows between Spaces

The React autocomplete app rendered inside the WebView used:

- `react-window` for virtualized rendering of long completion lists
- `@aws/amazon-q-developer-cli-autocomplete-parser` for shell parsing
- `@aws/amazon-q-developer-cli-fuzzysort` for fuzzy matching

**Source:** Brendan Falk (co-founder) on
[HN](https://news.ycombinator.com/item?id=27277819): _"We are built natively
for macOS in swift. We render it using a WKWebView (not Electron)."_

### 2.3 The Pseudoterminal (figterm)

`figterm` was Fig's most innovative component — a **passthrough PTY** sitting
between the terminal emulator and the shell:

1. Shell startup hook launches `figterm`, which spawns a child shell in a new PTY
2. All I/O passes through bidirectionally (transparent to the user)
3. `figterm` maintains an internal screen representation (originally C, rewritten
   to Rust with an embedded fork of `alacritty_terminal`)
4. Shell integration injects invisible OSC escape codes into the prompt, marking
   boundaries between "prompt", "edit buffer", "suggestion", and "output"
5. `figterm` reads only the cells tagged as "edit buffer" to extract user input
6. The buffer is sent to `fig_desktop` via `EditBufferHook` over Unix socket IPC

**autocomplete-rs parallel:** Our ZLE widget achieves the same goal (getting the
edit buffer) without the PTY interception layer — zsh gives us the buffer
directly via `$BUFFER` and `$CURSOR`.

### 2.4 Cursor Position Tracking

Fig used **three methods**, evolved over time:

| Method                                | Terminals                            | How It Works                                                                                     |
| ------------------------------------- | ------------------------------------ | ------------------------------------------------------------------------------------------------ |
| **Accessibility API** (`AXUIElement`) | Terminal.app, iTerm2, VS Code        | Queries `kAXBoundsForRangeParameterizedAttribute` for the cursor's screen-space `CGRect`         |
| **InputMethodKit (IME)**              | Alacritty, Kitty, WezTerm, JetBrains | Custom input method (`io.fig.cursor`) intercepts `firstRectForCharacterRange:` from the terminal |
| **VS Code Extension**                 | VS Code                              | Dedicated extension communicates cursor position directly                                        |

### 2.5 Known Issues

- **Cursor misalignment** — overlay frequently appeared at wrong position
  ([#939](https://github.com/withfig/fig/issues/939),
  [#2414](https://github.com/withfig/fig/issues/2414))
- **tmux incompatibility** — figterm conflicted with terminal multiplexers
- **Memory leaks** — WKWebView-based rendering caused severe memory issues (up
  to [160GB](https://github.com/withfig/fig/issues/2753))
- **Accessibility permission friction** — required explicit user grant, didn't
  stick across macOS updates
- **IME conflicts** — interfered with CJK input methods
- **macOS-only** — the entire approach was fundamentally macOS-specific; Linux
  and Windows were never shipped during Fig's lifetime

**Sources:**

- [How Fig Knows What You Typed](https://fig.io/blog/post/how-fig-knows-what-you-typed)
- [Launch HN: Fig (YC S20)](https://news.ycombinator.com/item?id=27277819)
- [Fig: JetBrains Support](https://fig.io/blog/post/jetbrains-support)
- [devtools.fm Episode #8](https://www.devtools.fm/episode/8)
- [aws/amazon-q-developer-cli](https://github.com/aws/amazon-q-developer-cli)

---

## 3. Kiro CLI (Current Successor)

Kiro CLI (Nov 2025) is the latest incarnation: Fig → CodeWhisperer for CLI →
Amazon Q Developer CLI → Kiro CLI. **The rendering architecture is unchanged**
through all rebrands.

### Current State

- **Closed source** (distributed under AWS IP License)
- The open-source snapshot at
  [aws/amazon-q-developer-cli-autocomplete](https://github.com/aws/amazon-q-developer-cli-autocomplete)
  preserves the full architecture as it existed before the Kiro rebrand
- Still uses `tao`/`wry` (Rust) for native windowing + WebView
- Still renders a React autocomplete app inside the WebView
- Still uses `figterm` for edit buffer extraction
- Still uses Accessibility API + IME for cursor tracking on macOS
- Added Linux support: IBus D-Bus monitoring (X11), GNOME Shell Extension
  (Wayland), Sway partial support
- Windows: WSL only (no native support), UI Automation code exists but
  incomplete

### Platform-Specific Cursor Tracking

| Platform                 | Primary Method                     | Code Location                                   |
| ------------------------ | ---------------------------------- | ----------------------------------------------- |
| macOS (native terminals) | Accessibility API (`AXUIElement`)  | `crates/macos-utils/src/caret_position.rs`      |
| macOS (GPU terminals)    | InputMethodKit IME                 | `crates/fig_input_method/src/imk.rs`            |
| Linux (X11)              | IBus D-Bus + x11rb window geometry | `crates/fig_desktop/src/platform/linux/ibus.rs` |
| Linux (GNOME/Wayland)    | GNOME Shell Extension              | `extensions/gnome-extension/src/extension.ts`   |
| Linux (Sway)             | i3/Sway IPC                        | `crates/fig_desktop/src/platform/linux/sway.rs` |
| Windows                  | UI Automation API                  | `crates/fig_desktop/src/platform/windows.rs`    |

### Two Rendering Modes

Kiro CLI now supports both:

1. **Autocomplete Dropdown** — the native GUI overlay (Fig's original UX)
2. **Inline Suggestions** — gray "ghost text" rendered in-band via the terminal
   (like zsh-autosuggestions)

**Sources:**

- [Kiro CLI Docs](https://kiro.dev/cli/)
- [Kiro CLI Autocomplete](https://kiro.dev/docs/cli/autocomplete/)
- [aws/amazon-q-developer-cli-autocomplete](https://github.com/aws/amazon-q-developer-cli-autocomplete)

---

## 4. Other Tools Comparison

### 4.1 Warp

**Approach: Owns the entire terminal renderer.** Warp replaced the terminal
emulator itself with a custom Rust application using GPU rendering (Metal on
macOS, wgpu elsewhere). Completions are part of the same render pipeline.

- Custom UI framework inspired by Flutter, co-developed with Nathan Sobo (Atom/Zed co-founder)
- Average screen redraw: 1.9ms, >144 FPS
- Per-command grid isolation (not a single VT100 grid)
- **Not applicable to our use case** — requires users to switch terminal emulators

**Source:** [How Warp Works](https://www.warp.dev/blog/how-warp-works)

### 4.2 Microsoft Inshellisense

**Approach: In-band ANSI terminal text** (same category as our current plan).

- TypeScript/Node.js, uses `node-pty` + `@xterm/headless`
- PTY wrapper intercepts shell output, renders completions as styled terminal text
- Uses OSC 633 shell integration sequences (same as VS Code terminal)
- Known rendering flicker issues ([#278](https://github.com/microsoft/inshellisense/issues/278))
- Cross-platform (macOS, Linux, Windows)

**Source:** [github.com/microsoft/inshellisense](https://github.com/microsoft/inshellisense)

### 4.3 Atuin

**Approach: In-band ANSI via ratatui inline viewport.**

- Rust, uses ratatui with `Viewport::Inline` mode
- Occupies a fixed number of terminal rows below the cursor
- Double-buffered rendering (only writes diffs)
- As of v18.4.0, defaults to compact UI + inline rendering

**Source:** [github.com/atuinsh/atuin](https://github.com/atuinsh/atuin)

### 4.4 Nushell / Reedline

**Approach: In-band ANSI via Reedline line editor menus.**

- Rust line editor library with built-in completion menu rendering
- Three menu types: ColumnarMenu, ListMenu, DescriptionMenu
- Renders using ANSI escape codes for cursor positioning and styling
- Tightly integrated with the shell

**Source:** [github.com/nushell/reedline](https://github.com/nushell/reedline)

### 4.5 Carapace

**Approach: Delegates to the shell's native completion system.**

- Multi-shell completion framework (bash, zsh, fish, nushell, etc.)
- Generates completions that feed into each shell's existing UI
- Does not render its own dropdown at all

**Source:** [github.com/carapace-sh/carapace](https://github.com/carapace-sh/carapace)

### Comparison Matrix

| Tool                 | Rendering               | Overlay Type               | Cross-Platform               | Open Source    |
| -------------------- | ----------------------- | -------------------------- | ---------------------------- | -------------- |
| **Fig/Kiro CLI**     | Native window + WebView | True GUI overlay           | macOS primary, Linux partial | Archived (MIT) |
| **Warp**             | GPU (Metal/wgpu)        | Built-in (IS the terminal) | macOS, Linux, Win            | No             |
| **Inshellisense**    | ANSI text (node-pty)    | In-band terminal text      | All                          | Yes (MIT)      |
| **Atuin**            | ANSI text (ratatui)     | In-band terminal text      | All                          | Yes (MIT)      |
| **Nushell/Reedline** | ANSI text (reedline)    | In-band terminal text      | All                          | Yes (MIT)      |
| **VS Code terminal** | DOM overlay on xterm.js | IDE-internal               | All (Electron)               | Yes (MIT)      |

---

## 5. Cursor Position Tracking

This is the **hardest problem** in the native overlay approach. Different
platforms and terminals expose cursor position differently (or not at all).

### 5.1 macOS: Accessibility API

```swift
// Pseudocode
let systemWide = AXUIElementCreateSystemWide()
var focusedElement: AnyObject?
AXUIElementCopyAttributeValue(systemWide, kAXFocusedUIElementAttribute, &focusedElement)

// Get cursor range → screen-space rect
let cursorRange = CFRange(location: range.location, length: 1)
AXUIElementCopyParameterizedAttributeValue(
    focusedElement,
    kAXBoundsForRangeParameterizedAttribute,
    cursorRangeValue,
    &boundsValue
)
// boundsValue is a CGRect with screen-space pixel coordinates
```

**Terminal compatibility:**

| Terminal     | AXSelectedTextRange | AXBoundsForRange | Works?             |
| ------------ | ------------------- | ---------------- | ------------------ |
| Terminal.app | Yes                 | Yes              | Full support       |
| iTerm2       | Yes                 | Yes              | Full support       |
| WezTerm      | Partial             | Unknown          | Needs IME fallback |
| Kitty        | No                  | No               | Needs IME fallback |
| Alacritty    | No                  | No               | Needs IME fallback |
| Ghostty      | Partial             | Unknown          | Evolving           |

**Rust crates:** `accessibility`, `accessibility-sys`, `macos-accessibility-client`

### 5.2 macOS: Input Method Editor (IME)

For GPU-rendered terminals that don't support Accessibility, register a custom
Input Method that intercepts `firstRectForCharacterRange:actualRange:` to get
cursor coordinates.

- Must be a native macOS `.app` bundle (Swift/ObjC)
- Requires user to enable third-party input method
- Can interfere with CJK input methods
- Fragile across macOS updates

### 5.3 Linux: X11

- **IBus D-Bus monitoring:** Listen for `SetCursorLocation` signals from
  `ibus-daemon` for cursor position
- **X11 window geometry:** `XGetInputFocus` + `XGetWindowAttributes` for window
  position, combine with shell cursor offset
- **AT-SPI2:** `atspi_text_get_character_extents` for GTK-based terminals only

### 5.4 Linux: Wayland

**Wayland is architecturally hostile to the overlay pattern.** By design:

- No global window positioning (compositor decides placement)
- No window inspection (can't query other windows)
- No `override_redirect` equivalent
- No input injection

**`wlr-layer-shell`** exists but cannot position at arbitrary pixel coordinates
relative to another window. GNOME's Mutter doesn't implement it.

**Fig/Kiro's solution:** GNOME Shell Extension that hooks into Mutter's internal
`MetaWindow` API.

### 5.5 Windows

`Win32::UI::Accessibility` — `CUIAutomation`, `OBJID_CARET` for caret tracking.
Kiro CLI does not yet have native Windows support.

---

## 6. Platform Feasibility

| Component              | macOS           | Linux X11                  | Linux Wayland   | Windows                | SSH/Remote  |
| ---------------------- | --------------- | -------------------------- | --------------- | ---------------------- | ----------- |
| Native overlay window  | Easy (NSWindow) | Medium (override_redirect) | Very Hard       | Easy (HWND)            | Impossible  |
| Cursor pixel tracking  | Medium (AX API) | Hard (IBus/AT-SPI2)        | Very Hard       | Medium (UI Automation) | Impossible  |
| In-band ANSI rendering | Works           | Works                      | Works           | Works                  | Works       |
| tmux/screen compat     | Overlay: broken | Overlay: broken            | Overlay: broken | Overlay: broken        | ANSI: works |

**Critical insight:** The native overlay approach breaks in SSH sessions, tmux,
screen, and remote environments — contexts where many developers spend
significant time. The in-band ANSI approach works transparently in all of these.

---

## 7. Terminal Extension APIs

**No major terminal emulator currently provides an API for external programs to
create floating popup overlays positioned at the cursor.**

| Terminal    | Overlay Capability                            | Notes                                                                                          |
| ----------- | --------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| **Kitty**   | Full-pane overlay only                        | [FR #7450](https://github.com/kovidgoyal/kitty/issues/7450) for positioned overlays was closed |
| **WezTerm** | Full-pane overlay only                        | Lua API has `PromptInputLine`/`InputSelector` but no positioned popups                         |
| **iTerm2**  | None                                          | Python API for status bar/toolbelt, not overlays                                               |
| **Ghostty** | None                                          | libghostty-vt planned but no extension API                                                     |
| **VS Code** | Proposed `registerTerminalCompletionProvider` | Only works inside VS Code                                                                      |

This is the fundamental reason Fig had to use Accessibility API + IME — the
terminals don't cooperate with overlay rendering.

---

## 8. Rust Implementation Options

### 8.1 For Native GUI Overlay

| Framework                 | Description                                      | Tradeoff                                 |
| ------------------------- | ------------------------------------------------ | ---------------------------------------- |
| **tao + wry**             | Same as Fig/Kiro. Tauri's windowing + WebView    | Proven approach, heavy (WebView runtime) |
| **winit + egui**          | Pure Rust GUI with immediate-mode rendering      | Lighter than webview, custom rendering   |
| **egui_overlay**          | Transparent overlay windows via egui + GLFW      | Purpose-built for overlays               |
| **iced**                  | Rust GUI framework, supports transparent windows | Elm architecture, reactive               |
| **objc2-app-kit** (macOS) | Raw FFI to NSWindow/NSView                       | Maximum native control, macOS only       |

### 8.2 For Cursor Tracking (macOS)

| Crate                        | Description                           |
| ---------------------------- | ------------------------------------- |
| `accessibility` (eiz)        | Wraps `AXUIElement` and related types |
| `accessibility-sys`          | Raw FFI bindings                      |
| `macos-accessibility-client` | Higher-level wrapper                  |
| `objc2-accessibility`        | Part of objc2 ecosystem               |

### 8.3 For In-Band ANSI Rendering

| Crate                       | Description                                                     |
| --------------------------- | --------------------------------------------------------------- |
| `crossterm`                 | Already in our dependency tree. Direct ANSI escape code writing |
| `ratatui` (inline viewport) | Double-buffered rendering with `Viewport::Inline`               |

---

## 9. Architecture Paths for autocomplete-rs

### Path A: In-Band ANSI (Current Plan — ADR-0006)

Render completions as terminal text below the cursor using crossterm and ANSI
escape codes. This matches fish shell, inshellisense, Atuin, Reedline.

**Pros:**

- Cross-platform from day one (macOS, Linux, Windows)
- Works in SSH, tmux, screen, Docker
- No OS permissions required
- Minimal binary size
- Direct terminal writes = lowest latency
- Simple architecture (daemon → terminal, no overlay process)

**Cons:**

- Limited to terminal cell grid (no shadows, transparency, rich styling)
- Must handle terminal edge cases (scrolling, resize, line wrapping)
- Can cause flicker (mitigate with synchronized output DEC 2026)

**Complexity:** Medium
**Time to ship:** Weeks

### Path B: Native GUI Overlay (Fig Pattern)

Create a separate native overlay window positioned at the cursor. Use
Accessibility API / IME for cursor tracking. Render with WebView or egui.

**Pros:**

- Rich visual UX (HTML/CSS styling, animations, transparency, shadows)
- Pixel-perfect positioning
- Can display icons, markdown descriptions, syntax highlighting

**Cons:**

- macOS-only initially (Linux Wayland is extremely hard)
- Requires Accessibility + IME permissions
- Broken in SSH, tmux, screen
- Fragile cursor tracking (per-terminal quirks)
- Large binary (webview runtime or GUI framework)
- Complex multi-process architecture
- Fig abandoned this approach (maintenance burden too high)

**Complexity:** Very high
**Time to ship:** Months

### Path C: Hybrid (ANSI Baseline + Optional Native Overlay)

Start with Path A as the universal renderer. Architect the completion engine
and rendering as separate concerns (trait-based). Later add a native overlay
renderer for macOS as an optional feature.

**Pros:**

- Ship fast with ANSI rendering
- Progressive enhancement: users who want the "Fig experience" can opt into
  the native overlay on macOS
- Clean architectural separation (CompletionEngine doesn't know about rendering)
- ANSI renderer serves as universal fallback

**Cons:**

- Maintaining two rendering paths
- Must design the abstraction boundary carefully upfront

**Complexity:** Medium initially, high if overlay is added later

---

## 10. Recommendation

**Start with Path A (in-band ANSI), architect for Path C.**

### Reasoning

1. **Fig's overlay approach failed commercially** — not because the rendering
   was wrong, but because the operational complexity (permissions, cursor
   tracking, per-terminal quirks, memory leaks) created friction that drove
   users away. The [160GB memory leak](https://github.com/withfig/fig/issues/2753),
   persistent cursor misalignment, and macOS-only limitation were fundamental
   to the architecture, not bugs to be fixed.

2. **Every active tool in this space uses in-band rendering** — inshellisense,
   Atuin, Reedline, carapace. The only tool using native overlays (Kiro CLI)
   inherited it from Fig and is now closed source.

3. **Developers work in SSH, tmux, and containers** — a rendering approach that
   breaks in these environments excludes a large portion of the target audience.

4. **ADR-0006 is still correct** — the decision to use raw ANSI via crossterm
   for inline rendering is sound. What was wrong was the assumption that this
   replicates "Fig's UX." It doesn't — and that's a feature, not a bug. Fig's
   UX came with costs that outweighed the visual benefits.

### What This Means for ADR-0006

ADR-0006 should be **updated with an addendum** noting:

- The research gap has been addressed
- Fig's actual approach (native overlay) was considered and rejected based on
  complexity, portability, and maintenance burden
- The in-band ANSI approach is a deliberate choice, not an oversight
- A native overlay can be added as a progressive enhancement later if desired

### What This Means for the CompletionEngine

Design the rendering interface as a trait:

```rust
trait CompletionRenderer {
    fn show(&mut self, suggestions: &[Suggestion], cursor_col: u16) -> Result<()>;
    fn hide(&mut self) -> Result<()>;
    fn navigate(&mut self, direction: Direction) -> Result<()>;
    fn selected(&self) -> Option<&Suggestion>;
}
```

The ANSI renderer implements this trait by writing escape codes to stdout. A
future native overlay renderer would implement the same trait by sending
suggestions to a GUI process over IPC.

---

## Appendix A: Fig/Kiro CLI Source Code References

Key files in the open-source
[aws/amazon-q-developer-cli-autocomplete](https://github.com/aws/amazon-q-developer-cli-autocomplete)
repo:

| Component         | Path                                            | Description                       |
| ----------------- | ----------------------------------------------- | --------------------------------- |
| Desktop app       | `crates/fig_desktop/`                           | tao/wry overlay window            |
| Window management | `crates/fig_desktop/src/webview/window.rs`      | `WindowState`, positioning        |
| Pseudoterminal    | `crates/figterm/`                               | PTY interceptor + screen model    |
| macOS IME         | `crates/fig_input_method/src/imk.rs`            | InputMethodKit integration        |
| macOS cursor      | `crates/macos-utils/src/caret_position.rs`      | Accessibility API cursor tracking |
| Linux IBus        | `crates/fig_desktop/src/platform/linux/ibus.rs` | D-Bus cursor tracking             |
| Linux X11         | `crates/fig_desktop/src/platform/linux/x11.rs`  | Window geometry                   |
| GNOME extension   | `extensions/gnome-extension/src/extension.ts`   | Wayland cursor tracking           |
| Sway support      | `crates/fig_desktop/src/platform/linux/sway.rs` | i3/Sway IPC                       |
| React app         | `packages/autocomplete/`                        | Autocomplete UI                   |
| Proto definitions | `proto/local.proto`                             | IPC message types                 |
| Spec parser       | `packages/autocomplete-parser/`                 | Shell input → completion matching |

## Appendix B: Protocol Buffer IPC (Fig/Kiro)

Key message types from `proto/local.proto`:

```protobuf
message EditBufferHook {
  ShellContext context = 1;
  string text = 2;
  int64 cursor = 3;
  int64 histno = 4;
  optional TerminalCursorCoordinates terminal_cursor_coordinates = 5;
}

message TerminalCursorCoordinates {
  int32 x = 1;       // column
  int32 y = 2;       // row
  int32 xpixel = 3;  // pixel x
  int32 ypixel = 4;  // pixel y
}

message CaretPositionHook {
  enum Origin { ORIGIN_TOP_LEFT = 0; ORIGIN_BOTTOM_LEFT = 1; }
  double x = 1;
  double y = 2;
  double width = 3;
  double height = 4;
}

message FocusedWindowDataHook {
  string source = 1;
  string id = 2;
  BoundingBox inner = 3;
  BoundingBox outer = 4;
  optional bool hide = 5;
  float scale = 6;
}
```

## Appendix C: External References

### Fig/Kiro

- [How Fig Knows What You Typed](https://fig.io/blog/post/how-fig-knows-what-you-typed)
- [Launch HN: Fig (YC S20)](https://news.ycombinator.com/item?id=27277819)
- [Fig: JetBrains Support (IME)](https://fig.io/blog/post/jetbrains-support)
- [Fig: SSH & Docker](https://fig.io/blog/post/autocomplete-in-ssh-and-docker)
- [devtools.fm Episode #8](https://www.devtools.fm/episode/8)
- [Fig is Sunsetting](https://fig.io/blog/post/fig-is-sunsetting)
- [aws/amazon-q-developer-cli-autocomplete](https://github.com/aws/amazon-q-developer-cli-autocomplete)
- [Kiro CLI Docs](https://kiro.dev/cli/)

### Other Tools

- [How Warp Works](https://www.warp.dev/blog/how-warp-works)
- [microsoft/inshellisense](https://github.com/microsoft/inshellisense)
- [atuinsh/atuin](https://github.com/atuinsh/atuin)
- [nushell/reedline](https://github.com/nushell/reedline)

### Platform APIs

- [Apple Accessibility API](https://developer.apple.com/documentation/applicationservices)
- [Apple InputMethodKit](https://developer.apple.com/documentation/inputmethodkit)
- [wlr-layer-shell protocol](https://wayland.app/protocols/wlr-layer-shell-unstable-v1)
- [AT-SPI2 Text Interface](https://gnome.pages.gitlab.gnome.org/at-spi2-core/libatspi/iface.Text.html)

### Rust Crates

- [egui_overlay](https://github.com/coderedart/egui_overlay)
- [tao](https://github.com/tauri-apps/tao)
- [wry](https://github.com/tauri-apps/wry)
- [accessibility (eiz)](https://github.com/eiz/accessibility)
