# Fig.io & Inshellisense Research Findings (Feb 2026)

Deep research on Fig.io and Microsoft's Inshellisense to inform autocomplete-rs architectural decisions. For the full detailed analysis with architecture diagrams, code examples, and source links, see [terminal-autocomplete-landscape.md](./terminal-autocomplete-landscape.md).

## Fig.io Post-Mortem

- **Architecture:** Three-process design — Swift/ObjC native macOS app (UI overlay via Accessibility API) + Node.js backend (suggestion generation) + figterm in Rust (PTY passthrough)
- **What worked:** The inline dropdown UX was best-in-class. The completion specs (withfig/autocomplete, 25k stars, MIT, 600+ CLI tools) are the real community asset.
- **What killed it:** Memory leaks (162GB RAM reported), high CPU while idle, 30-second startup delays, missed keystrokes. Root causes: IPC overhead between 3 processes, Node.js backend inefficiency, macOS Accessibility API fragility.
- **Source code:** The desktop app was NEVER open sourced. Only the completion specs are public. Amazon rebuilt from scratch as aws/amazon-q-developer-cli (Rust), then moved to closed-source Kiro CLI.
- **Fig went through 4 terminal integration approaches:** (1) CGEventTap keylogger — abandoned, (2) ZSH Line Editor integration — replaced, (3) figterm PTY passthrough with ANSI injection — primary, (4) Rust rewrite of figterm — final. The ZLE approach (what this project currently uses) was actually Fig's Method 2.

## Inshellisense Analysis

- **Architecture:** Single Node.js process, TypeScript + node-pty (PTY wrapper), @xterm/headless for terminal state tracking, raw ANSI escape codes for dropdown rendering (no TUI framework).
- **Uses Fig's specs directly** via @withfig/autocomplete npm package.
- **Problems:** node-pty native module causes installation nightmares (compilation failures, Node version constraints). 10-20s startup on PowerShell. Render flickering. Doesn't work with Starship prompt. Still pre-1.0 (rc.31). Can't load large specs (aws, gcloud, az).
- **What's right:** Single-process design, ANSI-based inline rendering, Fig spec reuse.

## Fig Completion Specs — The Key Asset

The specs have two distinct parts:

1. **Static declarations** — subcommands, options, args. Pure data. Can be compiled to any format (MessagePack, binary, etc.) at build time.
2. **Generators** — JavaScript functions that execute shell commands at completion time to produce dynamic suggestions (git branches, docker containers, npm scripts, etc.). These are the killer feature and require a JS runtime.

**Recommendation:** Use QuickJS (tiny embeddable JS engine, ~200KB, ~1ms cold start) for generator execution at runtime. Static specs get compiled to binary format at build time. This gives full Fig spec compatibility in a ~10-15MB binary with zero runtime dependencies.

## Architectural Concerns with Current Implementation

### 1. Inline Rendering vs. Alternate Screen (HIGH PRIORITY)

The current TUI uses Ratatui with `EnterAlternateScreen` — this blanks the terminal and shows a full-screen list (like fzf). Fig's UX was an inline dropdown that appeared below the cursor without disrupting terminal context. This is the single biggest UX differentiator.

**Recommendation:** Drop Ratatui for the completion dropdown. Render inline using raw ANSI escape codes — save cursor, write dropdown below prompt, erase on dismiss. Ratatui is great for full-screen apps but wrong for an inline overlay. Use crossterm directly or a thin wrapper.

### 2. Daemon Architecture (RECONSIDER)

The daemon adds complexity (socket management, IPC serialization, process lifecycle) for a benefit that may not be needed. The IPC round-trip (spawn client -> connect socket -> serialize -> deserialize -> respond) adds latency.

**Alternative:** Single binary invoked by the ZLE widget. Specs stay memory-mapped or cached on disk. No daemon, no socket, no IPC. Simpler, fewer failure modes. If spec loading is fast enough (memory-mapped MessagePack), startup cost is negligible.

**If keeping the daemon:** Consider whether the daemon should handle rendering too (eliminating the separate client process), making it more like a long-running service that the shell widget communicates with and that renders directly to the terminal.

### 3. QuickJS for Fig Generators (MISSING PIECE)

The roadmap plans `deno_ast` for build-time TypeScript parsing. That handles static spec compilation but not runtime generator execution. Generators are JS functions that need to run when the user requests completions. QuickJS is the right fit — tiny, fast startup, no dependencies, embeds cleanly in Rust via the `rquickjs` crate.

### 4. Spec Loading Strategy

Current plan: MessagePack embedding in binary. This works but has tradeoffs:

- Binary size grows with every spec (~10-30MB for all 600+ specs)
- Updates require rebuilding
- Alternative: Memory-mapped spec files in a cache directory, with a separate `autocomplete-rs update` command to fetch/compile specs. Keeps binary small, specs updatable independently.

## Cleanup Notes

### README.md

The README has AI-generated template feel — emoji bullet points, "blazing fast" marketing language, exhaustive doc links to pages that don't have real content yet, badges, acknowledgments section, "Built with Rust | Powered by Performance | Designed for Everyone" footer. Strip it down to what actually exists and what the project actually does today. Honest > polished.

### Documentation

The docs/ directory is extensive but much of it describes features that don't exist yet as if they do. Consider trimming to what's real and marking everything else as planned/aspirational. ADRs may need revisiting based on the architectural concerns above.

### Planning

The original proposals were written before the Fig/Inshellisense research. They should be reviewed against the new findings — particularly the rendering approach and daemon architecture. Planning is now tracked in beads (`bd list`).
