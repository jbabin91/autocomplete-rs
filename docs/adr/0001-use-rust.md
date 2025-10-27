# ADR-0001: Use Rust

**Status:** Accepted **Date:** 2025-10-25 **Decision Makers:** Project Team
**Technical Story:** Need to choose implementation language for autocomplete-rs

## Context

We need a programming language for building a fast, reliable terminal
autocomplete system that will:

- Replace Fig/Amazon Q with better performance
- Work across all terminals (iTerm2, Alacritty, Kitty, Wezterm, Ghostty, etc.)
- Support multiple shells (zsh, bash, fish)
- Process completion requests in <20ms total latency
- Run as a persistent daemon with minimal memory footprint (<50MB)
- Parse and embed 600+ Fig completion specs

### Requirements

- **Performance:** Sub-millisecond latency critical for good UX
- **Reliability:** Must not crash or corrupt shell state
- **Binary Size:** Single binary deployment, reasonable size
- **Cross-platform:** macOS and Linux (Windows future)
- **Developer Experience:** Primary language is TypeScript, but willing to learn

## Decision

We will use **Rust** as the implementation language.

## Consequences

### Positive

**Performance:**

- Native compilation → no runtime overhead
- Zero-cost abstractions → predictable performance
- Daemon startup: <5ms (vs Node.js ~50-100ms)
- Memory usage: ~10-30MB (vs Node.js ~50-100MB minimum)

**Reliability:**

- Compile-time safety prevents common bugs (null pointers, data races)
- No garbage collector → no GC pauses disrupting completions
- Strong type system catches errors before runtime
- Ownership system prevents memory leaks

**Deployment:**

- Single static binary (~5-15MB with all deps)
- No runtime to install (vs Node.js requiring npm/node)
- Works on systems without package managers

**Ecosystem:**

- Excellent TUI libraries (Ratatui, Crossterm)
- Strong async runtime (Tokio) for socket handling
- TypeScript parser available (deno_ast)
- Active community and growing adoption

### Negative

**Learning Curve:**

- Ownership and borrowing concepts are new
- Longer compile times than TypeScript (but still reasonable)
- Harder to prototype quickly compared to dynamic languages

**Development Speed:**

- More upfront design needed
- Compile-check-run cycle vs just run
- Estimated 2-4 weeks for MVP vs 1-2 weeks in TypeScript

**Ecosystem Maturity:**

- Some areas less mature than Node.js ecosystem
- Fewer terminal completion examples than bash/TypeScript

## Alternatives Considered

### Option 1: Bun (TypeScript with Zig runtime)

**Pros:**

- Use existing TypeScript knowledge
- Can reuse Fig specs directly (no parsing needed)
- Faster than Node.js (~3-4x)
- Single binary with `bun build --compile`

**Cons:**

- Still ~40MB binary size (vs Rust ~5-10MB)
- Still has runtime overhead (~10-20ms startup vs <5ms)
- Less predictable performance (JIT compiler)
- Memory usage higher than native (~40MB+ vs ~10-30MB)

**Why Not Chosen:** Performance and memory overhead still significant. While
faster than Node.js, doesn't meet our <20ms total latency goal reliably.

### Option 2: Go

**Pros:**

- Fast compilation
- Good concurrency support
- Simple language, easy to learn
- Single binary deployment

**Cons:**

- Garbage collector can cause latency spikes
- Larger binary sizes than Rust
- No Fig spec parser readily available (would need to write from scratch)
- Less established TUI ecosystem

**Why Not Chosen:** GC pauses are dealbreaker for sub-millisecond latency
requirements. Carapace (Go) exists but has different goals.

### Option 3: Zig

**Pros:**

- No runtime, truly native like Rust
- Simpler than Rust (no borrow checker)
- Very fast compilation
- Small binaries

**Cons:**

- Immature ecosystem (pre-1.0)
- Fewer libraries for our needs
- No TypeScript parser available
- Less community support
- Higher risk of ecosystem churn

**Why Not Chosen:** Too immature for production use. Would spend more time
building infrastructure than features.

### Option 4: TypeScript/Node.js

**Pros:**

- Existing knowledge (primary language)
- Fast development
- Can use Fig specs directly
- Huge ecosystem

**Cons:**

- Runtime overhead (~50-100ms startup)
- Memory usage (~50-100MB minimum)
- Unpredictable GC pauses
- Large deployment size
- Amazon Q already does this (and has positioning bugs)

**Why Not Chosen:** This is exactly what Fig/Amazon Q uses, and we're trying to
be better. Can't achieve <20ms latency reliably.

## Comparison Matrix

| Criterion      | Rust      | Bun        | Go        | Zig       | Node.js   |
| -------------- | --------- | ---------- | --------- | --------- | --------- |
| Startup Time   | <5ms ✅   | ~15ms ⚠️   | ~10ms ⚠️  | <5ms ✅   | ~100ms ❌ |
| Memory         | ~20MB ✅  | ~40MB ⚠️   | ~30MB ⚠️  | ~20MB ✅  | ~80MB ❌  |
| Binary Size    | ~8MB ✅   | ~40MB ⚠️   | ~15MB ⚠️  | ~5MB ✅   | N/A ❌    |
| Predictability | High ✅   | Medium ⚠️  | Medium ⚠️ | High ✅   | Low ❌    |
| Dev Speed      | Medium ⚠️ | Fast ✅    | Fast ✅   | Slow ❌   | Fast ✅   |
| Ecosystem      | Mature ✅ | Growing ⚠️ | Mature ✅ | Young ❌  | Mature ✅ |
| Learning Curve | Steep ❌  | Easy ✅    | Easy ✅   | Medium ⚠️ | Easy ✅   |

## Implementation Notes

- Use Rust 2024 Edition for latest features
- Leverage async/await with Tokio for socket handling
- Use Ratatui for TUI (mature, actively maintained)
- Parse TypeScript specs at build time with deno_ast
- Follow Rust conventions (rustfmt, clippy)

## References

- [Rust Performance Comparison](https://benchmarksgame-team.pages.debian.net/benchmarksgame/)
- [Bun Performance](https://bun.sh/blog/bun-v1.0)
- [Why Discord Switched to Rust](https://discord.com/blog/why-discord-is-switching-from-go-to-rust)
- Original discussion in project planning (2025-10-25)

## Review Notes

Decision made after thorough discussion weighing:

1. Performance requirements (hard constraint)
2. Developer familiarity (soft constraint)
3. Long-term maintainability
4. Deployment simplicity

The performance requirements ultimately drove the decision. While Bun was close,
Rust provides the most predictable, lowest-latency solution.
