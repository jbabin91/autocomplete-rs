# Terminal Autocomplete Landscape Research

> Compiled February 2026. Research conducted to inform architectural decisions for autocomplete-rs.

## Table of Contents

- [Terminal Autocomplete Landscape Research](#terminal-autocomplete-landscape-research)
  - [Table of Contents](#table-of-contents)
  - [Executive Summary](#executive-summary)
  - [1. Fig.io Deep Dive](#1-figio-deep-dive)
    - [1.1 History and Timeline](#11-history-and-timeline)
    - [1.2 Architecture](#12-architecture)
    - [1.3 Terminal Integration Evolution](#13-terminal-integration-evolution)
    - [1.4 Performance Failures](#14-performance-failures)
    - [1.5 Source Code Availability](#15-source-code-availability)
    - [1.6 The Completion Specs](#16-the-completion-specs)
  - [2. Microsoft Inshellisense Deep Dive](#2-microsoft-inshellisense-deep-dive)
    - [2.1 Architecture](#21-architecture)
    - [2.2 Rendering System](#22-rendering-system)
    - [2.3 Performance Profile](#23-performance-profile)
    - [2.4 Limitations](#24-limitations)
  - [3. Amazon Q / Kiro CLI Current State](#3-amazon-q--kiro-cli-current-state)
  - [4. Other Alternatives](#4-other-alternatives)
    - [zsh-autosuggestions](#zsh-autosuggestions)
    - [Carapace](#carapace)
    - [fzf-tab (Aloxaf/fzf-tab)](#fzf-tab-aloxaffzf-tab)
    - [Warp Terminal](#warp-terminal)
  - [5. Comparative Analysis](#5-comparative-analysis)
  - [6. Fig Completion Specs — The Key Asset](#6-fig-completion-specs--the-key-asset)
    - [6.1 Static Declarations](#61-static-declarations)
    - [6.2 Generators (Dynamic Completions)](#62-generators-dynamic-completions)
    - [6.3 Consumption Strategies](#63-consumption-strategies)
  - [7. Language and Runtime Evaluation](#7-language-and-runtime-evaluation)
    - [Why Rust (confirmed)](#why-rust-confirmed)
  - [8. Architectural Recommendations for autocomplete-rs](#8-architectural-recommendations-for-autocomplete-rs)
    - [8.1 Inline Rendering vs. Alternate Screen](#81-inline-rendering-vs-alternate-screen)
    - [8.2 Daemon vs. Single Process](#82-daemon-vs-single-process)
    - [8.3 Spec Loading Strategy](#83-spec-loading-strategy)
    - [8.4 QuickJS for Generator Execution](#84-quickjs-for-generator-execution)
    - [8.5 Proposed Architecture](#85-proposed-architecture)
  - [9. Sources](#9-sources)
    - [Fig.io](#figio)
    - [Fig GitHub Issues (Performance)](#fig-github-issues-performance)
    - [Microsoft Inshellisense](#microsoft-inshellisense)
    - [Amazon Q / Kiro](#amazon-q--kiro)

---

## Executive Summary

The terminal autocomplete space has a clear gap. Fig.io proved the UX — an inline dropdown with rich completions — but failed on performance and was swallowed by Amazon's ecosystem. Inshellisense proved the single-process model works but is held back by Node.js/node-pty overhead and installation friction. Kiro CLI inherited Fig's bloat and added its own (70+ orphaned DMG mounts observed on a single macOS system).

The opportunity for autocomplete-rs:

- **Rust single binary** — no runtime dependencies, ~10-15MB, installs via brew/cargo/curl
- **Inline ANSI rendering** — Fig-style dropdown without Accessibility API hacks or full-screen TUI
- **Full Fig spec compatibility** — 600+ CLI tools via the MIT-licensed withfig/autocomplete specs
- **QuickJS for generators** — tiny embedded JS engine for dynamic completions (git branches, docker containers, etc.)
- **Shell-native integration** — ZLE widgets for zsh, readline for bash, no PTY wrapper needed for MVP

---

## 1. Fig.io Deep Dive

### 1.1 History and Timeline

| Date      | Event                                                         |
| --------- | ------------------------------------------------------------- |
| 2020      | Fig founded, accepted into Y Combinator (S20)                 |
| 2021      | Public launch on Hacker News, macOS-only native app           |
| 2022      | Open sourced completion specs (withfig/autocomplete)          |
| Aug 2023  | Acquired by Amazon                                            |
| Mar 2024  | Fig officially sunsets, transitions to Amazon Q Developer CLI |
| Nov 2024  | Re-launched as Amazon Q Developer CLI                         |
| 2025      | Amazon Q Developer CLI receives only critical security fixes  |
| 2025-2026 | Closed-source Kiro CLI becomes the active product             |
| Mar 2025  | withfig/fig issue tracker archived                            |

Fig was a Y Combinator-backed startup (S20) that built the best terminal autocomplete UX to date. The core insight was simple: developers want IDE-style completions in their terminal. The execution was a native macOS app that rendered a floating dropdown near the cursor, powered by community-contributed completion specs for 600+ CLI tools.

Amazon acquired Fig for its terminal expertise and the community-built spec library. The original Fig codebase was effectively abandoned — Amazon rebuilt from scratch in Rust (aws/amazon-q-developer-cli), then moved to the closed-source Kiro CLI.

### 1.2 Architecture

Fig ran as three separate processes communicating via IPC:

```sh
┌───────────────────────────────────────────────────┐
│  Fig Desktop App (Swift/Objective-C)              │
│  - Native macOS UI overlay                        │
│  - Accessibility API for window positioning       │
│  - Rendered dropdown near cursor using screen     │
│    coordinates from the terminal window           │
└──────────────────────┬────────────────────────────┘
                       │ IPC (protobuf)
┌──────────────────────▼────────────────────────────┐
│  Node.js Backend Server                           │
│  - Loaded and processed completion specs          │
│  - Parsed command buffer into AST                 │
│  - Generated ranked suggestions                   │
│  - Ran generator functions for dynamic completions│
└──────────────────────┬────────────────────────────┘
                       │ IPC (protobuf)
┌──────────────────────▼────────────────────────────┐
│  figterm (Rust)                                   │
│  - Pseudoterminal passthrough layer               │
│  - Sat between terminal emulator and shell        │
│  - Monitored all ANSI escape codes                │
│  - Reconstructed terminal screen state            │
│  - Extracted edit buffer content                  │
│  - Injected invisible ANSI codes to mark regions  │
│    (prompt, edit buffer, output, suggestion)      │
└───────────────────────────────────────────────────┘
```

**Why three processes?** Historical evolution. The native macOS app was the original product. The Node.js backend handled spec processing because specs were JavaScript/TypeScript. figterm was added later as a more reliable way to track terminal state than the earlier approaches.

**The IPC overhead was a core performance problem.** Every keystroke had to flow through: terminal → figterm (Rust) → Node.js backend → Swift UI app, and suggestions back. Three processes, two IPC hops, serialization/deserialization at each boundary.

### 1.3 Terminal Integration Evolution

Fig went through four distinct approaches to solve the fundamental problem: "How do we know what the user has typed in the terminal?"

**Method 1: CGEventTap Keylogger (Early MVP)**

- Used macOS `CGEventTap` API to capture keyboard events when the terminal was focused
- Maintained an internal model of what the user typed
- Failed on: history navigation (up/down arrows), unfamiliar keybindings, terminal-specific shortcuts, copy/paste
- Abandoned due to fragility and accuracy issues

**Method 2: ZSH Line Editor (ZLE) Integration**

- Hooked into zsh's built-in ZLE API to read the edit buffer directly
- Similar approach to zsh-autosuggestions
- Accurate for zsh but shell-specific — wouldn't work for bash/fish
- This is the approach autocomplete-rs currently uses

**Method 3: Pseudoterminal Passthrough (figterm)**

- Inserted a transparent PTY layer between the terminal emulator and the shell
- Shell startup hooks launched `figterm` which spawned a child shell
- figterm monitored all I/O including ANSI escape codes
- Key innovation: injected invisible ANSI sequences into the prompt to semantically mark terminal regions (prompt vs. edit buffer vs. output)
- Could reconstruct the full terminal screen state
- Shell-agnostic — worked with any shell
- This became Fig's primary integration method

**Method 4: Rust Rewrite of figterm**

- Reimplemented figterm in Rust (originally was in another language)
- Used code from Alacritty and WezTerm for terminal emulation
- Tokio for async I/O
- Better performance, cross-platform potential, improved memory safety

**Takeaway for autocomplete-rs:** The ZLE approach (Method 2) is the simplest and most reliable for a zsh-first tool. It avoids the complexity of PTY interception. For multi-shell support, a lightweight PTY wrapper (Method 3) or shell-specific integrations (ZLE for zsh, readline for bash, fish's built-in completion system) are the options.

### 1.4 Performance Failures

Fig's performance issues were severe and well-documented in their GitHub issue tracker:

**Memory:**

- Issue #2753: "Fig Graphics and Media" consuming **162 GB** of RAM
- Issue #2577: Fig eating 2GB of memory during normal use
- Memory consumption worsened over time (memory leaks in the native macOS layer)

**CPU:**

- Issue #2769: High CPU usage on Intel Macs while idle
- Issue #1794: Sustained high CPU with no user activity
- Background processes consuming cycles even when not providing completions

**Terminal Responsiveness:**

- Issue #1369: "Slows down macOS significantly"
- Issue #1855: iTerm and Terminal.app very slow with Fig running
- Issue #1268: Typing feels sluggish, missed characters
- Issue #1556: 30-second wait for terminal to reach full speed after opening

**Root Causes:**

1. **Three-process IPC overhead** — Every keystroke traversed two process boundaries with protobuf serialization
2. **Node.js backend** — Garbage collection pauses, V8 memory overhead, spec processing inefficiency
3. **macOS Accessibility API** — Polling for window position, screen coordinate calculations, multi-monitor edge cases
4. **Memory leaks** — The native macOS "Fig Graphics and Media" process leaked memory continuously
5. **No cleanup** — Update mechanisms that mounted DMGs without ejecting them (observed: 70 orphaned Kiro CLI DMGs on a single system, consuming all available /dev/disk slots)

### 1.5 Source Code Availability

| Component                                           | Open Source?                           | License        |
| --------------------------------------------------- | -------------------------------------- | -------------- |
| Completion specs (withfig/autocomplete)             | Yes                                    | MIT            |
| Shell integrations (withfig/config)                 | Yes (archived)                         | —              |
| Desktop app (Swift/ObjC)                            | **No** — never released                | Proprietary    |
| figterm (Rust PTY layer)                            | **No** — never released                | Proprietary    |
| Node.js backend                                     | **No** — never released                | Proprietary    |
| Amazon Q Developer CLI (aws/amazon-q-developer-cli) | Yes (new Rust rewrite, not Fig's code) | MIT/Apache-2.0 |
| Kiro CLI                                            | **No**                                 | Proprietary    |

The only reusable asset from Fig is the completion specs repository.

### 1.6 The Completion Specs

The `withfig/autocomplete` repository is the crown jewel — 25k+ stars, 5.5k forks, 468+ contributors, MIT licensed, covering 600+ CLI tools.

Detailed analysis in [Section 6](#6-fig-completion-specs--the-key-asset).

---

## 2. Microsoft Inshellisense Deep Dive

### 2.1 Architecture

Inshellisense (`@microsoft/inshellisense`) is a single Node.js process that wraps the user's shell in a pseudo-terminal:

```sh
┌──────────────────────────────────────────────────┐
│  Single Node.js Process                          │
│                                                  │
│  ┌────────────────────────────────────────────┐  │
│  │  node-pty (native module)                  │  │
│  │  - Spawns shell as child process           │  │
│  │  - Bidirectional I/O interception          │  │
│  └────────────────────┬───────────────────────┘  │
│                       │                          │
│  ┌────────────────────▼───────────────────────┐  │
│  │  @xterm/headless                           │  │
│  │  - Internal terminal buffer                │  │
│  │  - Cursor position tracking                │  │
│  │  - ANSI sequence parsing                   │  │
│  │  - OSC sequence detection for shell hooks  │  │
│  └────────────────────┬───────────────────────┘  │
│                       │                          │
│  ┌────────────────────▼───────────────────────┐  │
│  │  Runtime Engine                            │  │
│  │  - Tokenizes command buffer                │  │
│  │  - Loads specs from @withfig/autocomplete  │  │
│  │  - Recursive subcommand/option/arg parser  │  │
│  │  - Context-aware suggestion generation     │  │
│  └────────────────────┬───────────────────────┘  │
│                       │                          │
│  ┌────────────────────▼───────────────────────┐  │
│  │  ANSI Renderer                             │  │
│  │  - Raw escape codes (no TUI framework)     │  │
│  │  - Cursor save/restore for positioning     │  │
│  │  - Max 5 suggestions visible               │  │
│  │  - Incremental patch-based updates         │  │
│  │  - UUID-based render IDs (race prevention) │  │
│  └────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────┘
```

**Key design choices:**

- **PTY wrapper** — `node-pty` (Microsoft's own library) spawns the shell as a child process, intercepting all I/O
- **Headless xterm** — Uses xterm.js without a display to track terminal state internally
- **Direct Fig spec consumption** — Imports `@withfig/autocomplete` as an npm dependency, no conversion needed
- **ANSI rendering** — No TUI framework, just raw escape codes via `ansi-escapes` library

**Tech stack:**

- TypeScript 85.9%, Node.js runtime
- 13 production dependencies, 24 dev dependencies
- Commander.js for CLI, chalk for colors, toml for config
- Jest for testing, esbuild for bundling
- Package size: 151 kB (plus node_modules)

### 2.2 Rendering System

Inshellisense renders the completion dropdown using raw ANSI escape sequences — no Ratatui, no ncurses, no TUI framework:

- **Cursor management** via `ansi-escapes` (hide/show/save/restore)
- **Directional rendering** — calculates available space, renders above or below cursor based on room
- **Patch-based updates** — generates incremental diffs instead of full redraws
- **Race condition prevention** — UUID-based render IDs prevent stale renders from overwriting newer ones
- **Styling** via `chalk` and `ansi-styles`

**Dropdown constraints:**

- Max 5 suggestions per page
- Suggestion text: 40 characters wide
- Description: 30 characters wide, 5 lines max
- Smart positioning to avoid screen edge overflow

**This is the right rendering approach.** Inline ANSI rendering is how a terminal autocomplete tool should work — it doesn't take over the screen, it augments the existing prompt context.

### 2.3 Performance Profile

**What's fast:**

- Lightweight suggestion rendering (max 5 items)
- Spec caching
- Incremental patch rendering

**What's slow:**

- **node-pty native module** — requires compilation at install time, fails on many platforms
- **Node.js startup** — ~30-50ms baseline, relevant when invoked per-keystroke
- **PowerShell** — 10-20 second load time (Issue #321)
- **Large specs** — `az`, `gcloud`, `aws` CLI specs explicitly unsupported due to size
- **Render flickering** — visible during rapid updates (Issue #278, being addressed with synchronized outputs)

### 2.4 Limitations

**Platform/compatibility issues:**

- Doesn't work with Starship prompt (Issue #262)
- Node.js version constraints: >=18, <23 (node-pty prebuilt binary availability)
- npm installation failures on various platforms (Issues #307, #313, #314)
- WSL2 bash autocomplete broken (Issue #172)
- PowerShell plugin integration issues (Issue #287)

**Functional gaps:**

- Generators don't work (Issue #48) — the dynamic completion functions from Fig specs
- Zsh alias expansion issues (Issue #54)
- No command name completion (Issue #111)
- Still pre-1.0 (version 0.0.1-rc.31 as of Jan 2026)

**Architectural constraints:**

- node-pty native module is the single biggest source of installation issues
- Node.js runtime overhead for what is fundamentally a text processing + rendering task
- PTY wrapper approach means it must run as the user's "shell" — can't be invoked on-demand like a ZLE widget

---

## 3. Amazon Q / Kiro CLI Current State

After acquiring Fig, Amazon went through several iterations:

1. **Amazon CodeWhisperer for CLI** — First rebrand, added AI features
2. **Amazon Q Developer CLI** — Second rebrand, open sourced as `aws/amazon-q-developer-cli` (Rust)
3. **Kiro CLI** — Current product, closed source, part of the Kiro IDE

**Kiro CLI observed issues (Feb 2026):**

- Auto-updater mounts DMG disk images and never ejects them
- 70 orphaned "Kiro CLI" DMG mounts observed on a single macOS system (`/dev/disk5` through `/dev/disk74`)
- Each mount consumes a `/dev/disk` slot (macOS has ~128 soft limit)
- Temp directory filled with 70 copies of the extracted app bundle (~38 GB of temp files)
- Bundles a `CodeWhispererInputMethod.app` helper (input method approach from the Fig lineage)
- Background processes: `kiro_cli_desktop`, `fig_input_method`, multiple `kiro-cli-term` shells
- Launch agent: `com.amazon.codewhisperer.launcher` running persistently
- Shell hooks injected into `.zshrc` (pre and post blocks, re-added automatically if removed)

**The Kiro CLI is the embodiment of what autocomplete-rs should NOT be** — bloated, leaky, invasive, and built for Amazon's ecosystem rather than developer productivity.

---

## 4. Other Alternatives

### zsh-autosuggestions

- Pure zsh plugin, fish-style inline ghost text suggestions
- Based on command history, not spec-driven
- No dropdown, no structured completions
- Lightweight, zero overhead
- Missing: subcommand awareness, option descriptions, dynamic generators

### Carapace

- Go-based multi-shell completion engine
- Supports 600+ commands with its own spec format
- Generates shell-native completions (compdef for zsh, complete for bash)
- No visual dropdown — uses the shell's built-in completion UI
- Solid but different product category (completion generation vs. interactive dropdown)

### fzf-tab (Aloxaf/fzf-tab)

- Replaces zsh's default completion menu with an fzf-powered dropdown
- Fuzzy searchable, colored, preview support
- Requires fzf installed
- Good but: depends on zsh's completion system (compdef), not spec-driven, no description panels, no generator support

### Warp Terminal

- Has Fig-style autocomplete built into the terminal itself
- Electron-based terminal app
- Proprietary, closed source
- Heavy — full terminal replacement rather than a shell plugin

---

## 5. Comparative Analysis

| Feature       | Fig                      | Inshellisense            | Kiro CLI       | autocomplete-rs (target)     |
| ------------- | ------------------------ | ------------------------ | -------------- | ---------------------------- |
| Language      | Swift + Node + Rust      | TypeScript/Node.js       | Rust (closed)  | Rust                         |
| Binary size   | ~200MB installed         | ~150KB + node_modules    | ~560MB DMG     | ~10-15MB                     |
| Memory        | 100MB+ (leaked to 162GB) | Moderate                 | ~230MB         | <50MB target                 |
| Startup       | 30 seconds reported      | 30-50ms (Node)           | Unknown        | <5ms target                  |
| Processes     | 3                        | 1                        | 2+             | 1                            |
| Rendering     | Native macOS overlay     | Inline ANSI              | Native overlay | Inline ANSI                  |
| Shell support | zsh, bash, fish          | 8+ shells                | zsh            | zsh (MVP), bash/fish planned |
| Spec format   | Fig specs (TypeScript)   | Fig specs (direct)       | Proprietary    | Fig specs (compiled)         |
| Spec count    | 600+                     | 600+                     | Unknown        | 600+ (via Fig specs)         |
| Generators    | Full JS execution        | Broken (Issue #48)       | Unknown        | QuickJS embedded             |
| Distribution  | DMG / brew               | npm install              | DMG / brew     | cargo / brew / curl          |
| Dependencies  | macOS Accessibility API  | node-pty (native module) | macOS APIs     | None                         |
| Open source   | Specs only               | Yes (MIT)                | No             | Yes (MIT)                    |
| Platform      | macOS only               | Win/Linux/macOS          | macOS          | macOS/Linux (MVP)            |

---

## 6. Fig Completion Specs — The Key Asset

Repository: [withfig/autocomplete](https://github.com/withfig/autocomplete)
Stars: 25,100+ | Forks: 5,500+ | Contributors: 468+ | License: MIT

The specs define completions for 600+ CLI tools in a declarative TypeScript format. This is the single most valuable piece of the Fig ecosystem and the foundation for any successor tool.

### 6.1 Static Declarations

The majority of each spec is static data describing the command tree:

```typescript
const completionSpec: Fig.Spec = {
  name: 'git',
  subcommands: [
    {
      name: 'checkout',
      description: 'Switch branches or restore working tree files',
      args: {
        name: 'branch',
        description: 'Branch to checkout',
        // generator would go here for dynamic branch listing
      },
      options: [
        {
          name: ['-b', '--branch'],
          description: 'Create and checkout a new branch',
          args: { name: 'new-branch-name' },
        },
        {
          name: ['-f', '--force'],
          description: 'Force checkout (throw away local modifications)',
        },
      ],
    },
    // ... hundreds more subcommands
  ],
};
```

This data — command names, subcommand trees, option flags, argument descriptions — is pure structure. It can be:

- Parsed at build time from TypeScript source
- Serialized to MessagePack, FlatBuffers, or a custom binary format
- Embedded in the binary or stored as memory-mapped files
- Queried at runtime with zero JavaScript execution

### 6.2 Generators (Dynamic Completions)

Generators are JavaScript functions that execute at completion time to produce context-dependent suggestions:

```typescript
const completionSpec: Fig.Spec = {
  name: 'git',
  subcommands: [
    {
      name: 'checkout',
      args: {
        name: 'branch',
        generators: {
          // This runs: git branch --no-color
          // Then parses the output into suggestions
          script: ['git', 'branch', '--no-color'],
          postProcess: function (output) {
            return output
              .split('\n')
              .filter((branch) => !branch.includes('*'))
              .map((branch) => ({
                name: branch.trim(),
                description: 'Branch',
                icon: 'fig://icon?type=git',
              }));
          },
        },
      },
    },
  ],
};
```

Common generator patterns:

- **Shell command execution** — Run a command and parse its output (git branches, docker containers, npm scripts)
- **File system** — List files/directories matching patterns
- **Custom logic** — JSON parsing, filtering, sorting of command output
- **Caching** — Some generators specify TTLs to avoid re-running expensive commands

**Generators are the killer feature** that separates spec-driven completions from static lists. Without them, you can't suggest the actual git branches in your repo or the running docker containers.

### 6.3 Consumption Strategies

**Option A: Build-time compilation + embedded QuickJS**

- Parse TypeScript specs at build time using `deno_ast` or `swc`
- Extract static declarations → compile to binary format (MessagePack)
- Extract generator functions → preserve as JavaScript strings
- At runtime: static lookups are pure Rust, generators execute via embedded QuickJS
- Tradeoff: binary size grows with specs, updates require rebuild

**Option B: External spec cache + embedded QuickJS**

- Ship a small binary with QuickJS embedded
- `autocomplete-rs update` command fetches and compiles specs to a cache directory (`~/.cache/autocomplete-rs/specs/`)
- Specs stored as memory-mapped files
- Tradeoff: requires initial setup step, but binary stays small and specs update independently

**Option C: Hybrid**

- Embed the most common specs (git, docker, npm, cargo, brew — top 20-30) in the binary
- Additional specs loaded from cache
- Best of both: works out of the box for common tools, extensible for everything else

**Recommended: Option C (Hybrid)**

---

## 7. Language and Runtime Evaluation

### Why Rust (confirmed)

| Factor               | Rust                           | Bun/TypeScript         | Go                  |
| -------------------- | ------------------------------ | ---------------------- | ------------------- |
| Binary size          | 5-15MB                         | 50-90MB (compiled)     | 10-20MB             |
| Startup time         | <1ms                           | ~5ms                   | ~5ms                |
| Runtime deps         | None                           | None (compiled) or Bun | None                |
| Memory control       | Manual (zero-cost)             | GC                     | GC                  |
| Fig spec consumption | Needs build-time parsing       | Native (direct import) | Needs parsing       |
| Generator execution  | QuickJS embed (~200KB)         | Native JS              | QuickJS or V8 embed |
| Terminal libraries   | crossterm, ratatui (excellent) | Limited                | bubbletea (good)    |
| Distribution         | cargo install, brew, curl      | npm or compiled binary | go install, brew    |
| Dev velocity         | Slower                         | Fastest                | Moderate            |

**Bun was seriously considered** for its native Fig spec consumption and fast development. However:

- `bun build --compile` produces 50-90MB binaries (entire Bun runtime bundled)
- For a tool that should feel invisible, a 10MB Rust binary is more appropriate
- The user's goal is open source distribution with minimal friction — Rust's `cargo install` and static binary distribution are ideal
- Performance on the hot path (keystroke → suggestion) is language-agnostic for this workload, but binary size and startup time are not

**QuickJS as the JS bridge:**

- Embeds in ~200KB of additional binary size
- Cold starts in ~1ms
- Full ES2023 support
- No dependencies
- Available via `rquickjs` crate (well-maintained Rust bindings)
- Only invoked for generator execution — static spec lookups are pure Rust

---

## 8. Architectural Recommendations for autocomplete-rs

### 8.1 Inline Rendering vs. Alternate Screen

**Current state:** Ratatui with `EnterAlternateScreen` — blanks the terminal, shows full-screen list, returns on selection.

**Problem:** This is the fzf UX, not the Fig UX. The defining feature of Fig was that completions appeared inline below the cursor without disrupting terminal context. You could see your command, see the suggestions, and the rest of your terminal history remained visible.

**Recommendation:** Drop Ratatui for the completion dropdown. Render inline using crossterm directly:

```sh
$ git checkout ma|
┌──────────────────────────────────┐
│ main              default branch │
│ master            old default    │
│ feature/markdown  docs update    │
└──────────────────────────────────┘
```

Implementation approach:

1. Save cursor position
2. Move cursor below current line
3. Write dropdown box using box-drawing characters and ANSI colors
4. Handle keyboard input (arrows, enter, esc, typing to filter)
5. On dismiss: restore cursor, erase dropdown lines
6. Use synchronized output (`\x1b[?2026h` / `\x1b[?2026l`) to prevent flicker

Ratatui can still be used for other UI needs (config TUI, dashboard, etc.) but not for the inline completion dropdown.

### 8.2 Daemon vs. Single Process

**Current state:** Persistent daemon on Unix socket, separate client binary connects to request completions.

**Arguments for daemon:**

- Specs stay loaded in memory — zero loading cost after first request
- Shared state across terminal sessions
- Can pre-warm spec cache

**Arguments against daemon:**

- IPC serialization overhead on every keystroke
- Process lifecycle management (startup, shutdown, crash recovery, stale sockets)
- More failure modes (socket permissions, daemon not running, version mismatch)
- Complexity for what is fundamentally: parse a short string, look up a spec, return suggestions

**Recommendation:** Start without a daemon. Profile the single-process approach first.

If spec loading via memory-mapped files is fast enough (likely <5ms for MessagePack deserialization of a single spec), the daemon provides no benefit. The ZLE widget invokes the binary, the binary loads the relevant spec, generates suggestions, renders inline, and exits.

If profiling shows spec loading is a bottleneck, add a daemon later. The architecture should be designed so the completion engine is a library that can be called from either a CLI binary or a long-running daemon.

### 8.3 Spec Loading Strategy

**Recommended approach (Hybrid):**

1. **Build-time:** Compile the top 20-30 most common specs (git, docker, npm, cargo, brew, kubectl, etc.) into the binary as embedded data
2. **Runtime cache:** Additional specs stored in `~/.cache/autocomplete-rs/specs/` as individual MessagePack files
3. **Update command:** `autocomplete-rs update` fetches the latest withfig/autocomplete, compiles all specs, writes to cache
4. **Lazy loading:** Only load the spec for the command being completed, not all 600+
5. **Memory mapping:** Use `mmap` for cached spec files — OS handles memory management, no explicit loading needed

**Spec compilation pipeline:**

```txt
withfig/autocomplete (TypeScript)
    ↓ deno_ast or swc (build-time)
Parsed AST
    ↓ Extract static data
MessagePack binary (subcommands, options, args, descriptions)
    ↓ Extract generators
JavaScript source strings (preserved for QuickJS execution)
    ↓ Package together
.spec file (MessagePack envelope containing both static data and JS strings)
```

### 8.4 QuickJS for Generator Execution

**The problem:** Fig generators are JavaScript functions. Static spec data can be compiled to any format, but generators must execute at completion time.

**The solution:** Embed QuickJS via the `rquickjs` crate.

**How it works:**

1. User types `git checkout <space>` and triggers completion
2. autocomplete-rs loads the `git` spec (MessagePack, <1ms)
3. Static lookup finds the `checkout` subcommand's `args` field has a generator
4. Generator's JavaScript source is passed to QuickJS
5. If the generator has a `script` field, autocomplete-rs runs the shell command (e.g., `git branch --no-color`) and passes the output to the JS `postProcess` function
6. QuickJS executes `postProcess`, returns structured suggestions
7. Suggestions rendered in the inline dropdown

**Performance budget:**

- QuickJS cold start: ~1ms
- Shell command execution: variable (but cached)
- JS postProcess execution: <1ms for typical functions
- Total generator overhead: ~5-10ms (dominated by shell command execution)

**Why not just implement generators natively in Rust?**

- There are hundreds of unique generator functions across 600+ specs
- They're maintained by the community in JavaScript
- Reimplementing them means forking away from the upstream spec format
- QuickJS adds ~200KB to the binary — negligible cost for full compatibility

### 8.5 Proposed Architecture

```sh
┌──────────────────────────────────────────────────┐
│  Single Rust Binary (~10-15MB)                   │
│                                                  │
│  ┌────────────────────────────────────────────┐  │
│  │  Shell Integration (ZLE widget / readline) │  │
│  │  - Captures buffer + cursor position       │  │
│  │  - Invokes binary with args                │  │
│  │  - Receives selected completion            │  │
│  └────────────────────┬───────────────────────┘  │
│                       │                          │
│  ┌────────────────────▼───────────────────────┐  │
│  │  Command Parser                            │  │
│  │  - Tokenizes buffer                        │  │
│  │  - Determines completion context           │  │
│  │  - Identifies: command, subcommand,        │  │
│  │    option, argument                        │  │
│  └────────────────────┬───────────────────────┘  │
│                       │                          │
│  ┌────────────────────▼───────────────────────┐  │
│  │  Spec Engine                               │  │
│  │  - Loads spec (embedded or cached mmap)    │  │
│  │  - Static lookup for commands/options      │  │
│  │  - Delegates to QuickJS for generators     │  │
│  └────────────┬───────────────┬───────────────┘  │
│               │               │                  │
│  ┌────────────▼────────┐  ┌──▼─────────────────┐ │
│  │  Static Specs       │  │  QuickJS Runtime   │ │
│  │  (MessagePack)      │  │  (~200KB embedded) │ │
│  │  - Embedded top 30  │  │  - Runs generators │ │
│  │  - Cached via mmap  │  │  - ES2023 support  │ │
│  └─────────────────────┘  └────────────────────┘ │
│                       │                          │
│  ┌────────────────────▼───────────────────────┐  │
│  │  Inline ANSI Renderer                      │  │
│  │  - Crossterm for terminal control          │  │
│  │  - Box-drawing dropdown below cursor       │  │
│  │  - Keyboard navigation (arrows/enter/esc)  │  │
│  │  - Synchronized output (no flicker)        │  │
│  │  - Themed styling (Catppuccin planned)     │  │
│  └────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────┘
```

**Performance targets:**

| Operation                  | Target    | Notes                      |
| -------------------------- | --------- | -------------------------- |
| Binary startup             | <2ms      | No runtime initialization  |
| Spec loading (embedded)    | <1ms      | Already in memory          |
| Spec loading (cached)      | <5ms      | Memory-mapped file         |
| Command parsing            | <1ms      | Simple string tokenization |
| Static suggestion lookup   | <1ms      | Hash map / trie lookup     |
| Generator execution        | <10ms     | Dominated by shell command |
| Dropdown render            | <2ms      | ANSI escape codes          |
| **Total (static)**         | **<5ms**  | No generators needed       |
| **Total (with generator)** | **<15ms** | Including shell command    |

---

## 9. Sources

### Fig.io

- [withfig/autocomplete](https://github.com/withfig/autocomplete) — Completion specs (MIT, 25k stars)
- [withfig/fig](https://github.com/withfig/fig) — Issue tracker (archived Mar 2025)
- [withfig/config](https://github.com/withfig/config) — Shell integrations (archived Jul 2023)
- [How does Fig know what you've typed in the terminal?](https://fig.io/blog/post/how-fig-knows-what-you-typed) — Technical blog post on integration methods
- [Launch HN: Fig (YC S20)](https://news.ycombinator.com/item?id=27277819) — Original HN launch
- [Amazon acquires Fig](https://techcrunch.com/2023/08/29/amazon-fig-command-line-terminal-generative-ai/) — TechCrunch acquisition article
- [Fig is sunsetting](https://fig.io/blog/post/fig-is-sunsetting) — Official sunset announcement

### Fig GitHub Issues (Performance)

- [#2753](https://github.com/withfig/fig/issues/2753) — 162 GB RAM consumption
- [#2577](https://github.com/withfig/fig/issues/2577) — 2GB memory usage
- [#2769](https://github.com/withfig/fig/issues/2769) — High CPU on Intel Macs
- [#1794](https://github.com/withfig/fig/issues/1794) — CPU while idle
- [#1369](https://github.com/withfig/fig/issues/1369) — Slows down macOS
- [#1855](https://github.com/withfig/fig/issues/1855) — iTerm/Terminal slow
- [#1268](https://github.com/withfig/fig/issues/1268) — Typing sluggishness
- [#1556](https://github.com/withfig/fig/issues/1556) — 30-second startup delay

### Microsoft Inshellisense

- [microsoft/inshellisense](https://github.com/microsoft/inshellisense) — Source code
- [npm: @microsoft/inshellisense](https://www.npmjs.com/package/@microsoft/inshellisense) — Package
- [Show HN: Inshellisense](https://news.ycombinator.com/item?id=38167363) — HN discussion
- [Issue #321](https://github.com/microsoft/inshellisense/issues/321) — PowerShell slowness
- [Issue #278](https://github.com/microsoft/inshellisense/issues/278) — Render flickering
- [Issue #262](https://github.com/microsoft/inshellisense/issues/262) — Starship incompatibility
- [Issue #48](https://github.com/microsoft/inshellisense/issues/48) — Generators don't work

### Amazon Q / Kiro

- [aws/amazon-q-developer-cli](https://github.com/aws/amazon-q-developer-cli) — Open source Rust CLI
- [Understanding AWS Q Developer, Q CLI, CodeWhisperer, and Kiro](https://medium.com/@rongalinaidu/understanding-aws-q-developer-q-cli-codewhisperer-and-kiro-fc8d6f7e6075)
