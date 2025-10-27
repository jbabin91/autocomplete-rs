# Add Fig Spec Parser (Phase 2)

**Priority:** 2 **Phase:** 2 (Scale) **Dependencies:** `implement-mvp-parser`
**Blocks:** `add-theme-support`

## Why

To provide autocomplete for 600+ CLI tools, we need to parse and use Fig's
open-source TypeScript completion specs. Doing this at build time (not runtime)
ensures zero overhead and maintains our performance goals.

## What Changes

- Clone withfig/autocomplete repository during build
- Parse TypeScript specs using deno_ast at build time
- Convert specs to efficient MessagePack binary format
- Embed compiled specs in binary for runtime use
- Create spec loader for runtime access
- **BREAKING**: Replace hardcoded git spec with parsed version

## Impact

- Affected specs:
  - `build-system` (new capability)
  - `spec-loader` (new capability)
  - `parser` (modified - use loaded specs instead of hardcoded)
  - `git-completion` (modified - remove hardcoded spec)
- Affected code:
  - `build.rs` - TypeScript parsing and MessagePack generation
  - `src/specs/mod.rs` - spec loader from embedded data
  - `src/parser/mod.rs` - lookup specs dynamically
  - `Cargo.toml` - enable deno_ast in build-dependencies
- Dependencies: deno_ast 0.51+ (build-time only)
- Migration: Remove hardcoded git spec, ensure tests use loaded spec

## Design Decisions

- **Build-time vs Runtime**: Parse at build time to avoid runtime overhead
- **MessagePack format**: More efficient than JSON, smaller binary size
- **Embed in binary**: Simpler deployment, no external files needed
- **deno_ast**: Official TypeScript parser from Deno project, well-maintained
