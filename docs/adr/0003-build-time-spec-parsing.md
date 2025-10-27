# ADR-0003: Build-time Spec Parsing

**Status:** Accepted **Date:** 2025-10-25 **Decision Makers:** Project Team
**Technical Story:** Choose how to parse and embed Fig's 600+ TypeScript
completion specs

## Context

Fig's autocomplete specs are written in TypeScript and define completions for
600+ CLI tools (git, npm, docker, kubectl, etc.). We need to make these specs
available in our Rust application for fast lookup during completion requests.

### Requirements

- **Startup Time:** Daemon must start in <5ms (can't parse specs at startup)
- **Completion Speed:** Need instant spec lookups (<1ms)
- **Binary Size:** Keep reasonable despite embedding 600+ specs
- **Maintainability:** Easy to update specs from upstream
- **Type Safety:** Preserve TypeScript spec semantics in Rust

### The Specs

- Location: `https://github.com/withfig/autocomplete`
- Format: TypeScript with complex object literals
- Count: 600+ files
- Size: ~50MB of TypeScript source
- Structure: Nested subcommands, options, arguments, generators

Example spec:

```typescript
const completionSpec: Fig.Spec = {
  name: "git",
  description: "The stupid content tracker",
  subcommands: [
    {
      name: "checkout",
      description: "Switch branches or restore files",
      options: [{ name: ["-b", "--branch"], description: "Create new branch" }],
    },
  ],
};
```

## Decision

We will **parse TypeScript specs at build time** using **deno_ast**, **compile
to MessagePack**, and **embed in the binary** as static data.

### Build Process

1. Build script (`build.rs`) runs before compilation
2. deno_ast parses TypeScript files to AST
3. Extract completion data structures
4. Serialize to MessagePack format
5. Embed as `include_bytes!()` in Rust binary
6. Lazy-load specs at runtime with LRU cache

## Consequences

### Positive

**Startup Performance:**

- No parsing overhead at daemon startup
- Specs are pre-compiled binary data
- Daemon starts in <5ms even with 600 specs available
- First completion request: ~1-2ms (just deserialize MessagePack)

**Runtime Performance:**

- MessagePack deserialization: <0.1ms per spec
- Binary format faster than JSON (~3-5x)
- LRU cache means hot specs stay in memory
- No AST parsing in hot path

**Binary Size:**

- MessagePack more compact than JSON (~30% smaller)
- ~10-15MB for all 600 specs (vs ~50MB TypeScript source)
- Still single-binary deployment
- Acceptable tradeoff for zero startup cost

**Reliability:**

- Parse errors caught at build time, not runtime
- Type checking happens once during build
- No runtime parsing failures
- Guaranteed valid specs in production

**Maintainability:**

- Update specs: just update submodule and rebuild
- No runtime dependency on TypeScript/Node.js
- Build-time validation ensures quality
- Easy CI integration

### Negative

**Build Complexity:**

- Need deno_ast build dependency (~100MB download first time)
- Longer build times (~30-60s for full rebuild)
- Build script complexity (TypeScript AST traversal)
- Must maintain TypeScript → Rust mapping

**Update Workflow:**

- Spec updates require rebuild (can't hot-reload)
- Development iteration slower than runtime parsing
- Must rebuild to test spec changes
- Not suitable for user-defined specs (future consideration)

**Debugging:**

- Embedded specs harder to inspect than files
- Need tooling to extract/view embedded specs
- Build errors less intuitive than runtime errors
- Can't easily patch specs in production

**Flexibility:**

- Locked to spec format at build time
- Can't load custom specs at runtime (Phase 1)
- Version mismatches require rebuild
- Users can't add their own completions easily (future feature)

## Alternatives Considered

### Option 1: Runtime TypeScript Parsing

**How It Works:**

- Ship Fig specs as TypeScript files
- Parse with deno_core at runtime when needed
- Cache parsed specs in memory

**Pros:**

- No build-time complexity
- Easy to update specs (just replace files)
- Users could add custom specs
- Simpler debugging

**Cons:**

- Requires shipping deno_core runtime (~50MB)
- First parse: ~100-200ms per spec (unacceptable)
- Startup cost: ~5-10s to parse all 600 specs
- Memory overhead of JavaScript runtime
- Unpredictable performance

**Why Not Chosen:** Cannot meet <5ms startup requirement. Would need to parse
specs on-demand, causing latency spikes on first use of each command.

### Option 2: JSON Spec Format

**How It Works:**

- Convert TypeScript specs to JSON at build time
- Embed JSON in binary
- Parse with serde_json at runtime

**Pros:**

- Simpler than MessagePack (human-readable)
- No deno_ast dependency
- Standard format
- Easy debugging

**Cons:**

- JSON slower to parse than MessagePack (~3-5x)
- Larger binary size (~30% bigger)
- Still need TypeScript → JSON conversion step
- Less efficient for nested structures

**Why Not Chosen:** MessagePack provides same benefits with better performance
and smaller size. Human-readability not critical for embedded data.

### Option 3: Runtime Dynamic Loading

**How It Works:**

- Ship specs as separate files
- Load from `~/.config/autocomplete-rs/specs/`
- Parse on demand, cache in memory

**Pros:**

- Users can add/modify specs
- No binary size impact
- Easy updates (just download new files)
- Flexible and extensible

**Cons:**

- File I/O on first load (~5-10ms per spec)
- Dependency on filesystem state
- Startup time variable (network shares, slow disks)
- Need spec versioning/validation
- Installation complexity

**Why Not Chosen:** Too many variables for predictable performance. File I/O
adds latency. Better suited for Phase 3+ when allowing custom specs.

### Option 4: Code Generation (proc macros)

**How It Works:**

- Generate Rust code from TypeScript specs at build time
- Each spec becomes native Rust struct
- Compile directly into binary

**Pros:**

- Fastest possible (native Rust structs)
- Type-safe at compile time
- Zero runtime overhead
- No parsing needed

**Cons:**

- Massive code generation (~500k+ LOC for 600 specs)
- Compile times extremely long (>5 minutes)
- Binary size huge (~50-100MB)
- Hard to maintain generated code
- Debugging nightmare

**Why Not Chosen:** Code bloat unacceptable. Compile times too long for
development. MessagePack provides 95% of the performance with 10% of the
complexity.

### Option 5: SQLite Embedded Database

**How It Works:**

- Convert specs to SQLite database at build time
- Embed database in binary
- Query at runtime

**Pros:**

- Efficient queries
- Structured data model
- Good for complex lookups
- Mature ecosystem

**Cons:**

- Overkill for hierarchical data
- SQLite overhead (~1-2MB binary size)
- Slower than MessagePack for simple lookups
- More complex than needed
- Schema migrations complex

**Why Not Chosen:** Over-engineered for our use case. Specs are hierarchical
trees, not relational data. MessagePack simpler and faster.

## Comparison Matrix

| Criterion    | Build-time MP | Runtime TS | JSON Embed | Dynamic Load | Codegen    | SQLite    |
| ------------ | ------------- | ---------- | ---------- | ------------ | ---------- | --------- |
| Startup      | <5ms ✅       | ~10s ❌    | <10ms ⚠️   | ~100ms ❌    | <5ms ✅    | <10ms ⚠️  |
| Lookup Speed | <0.1ms ✅     | ~1ms ⚠️    | ~0.5ms ⚠️  | ~2ms ⚠️      | <0.01ms ✅ | ~0.5ms ⚠️ |
| Binary Size  | ~15MB ⚠️      | ~60MB ❌   | ~20MB ⚠️   | ~5MB ✅      | ~80MB ❌   | ~20MB ⚠️  |
| Build Time   | ~60s ⚠️       | <5s ✅     | ~30s ⚠️    | <5s ✅       | ~300s ❌   | ~45s ⚠️   |
| Flexibility  | Low ❌        | High ✅    | Low ❌     | High ✅      | Low ❌     | Medium ⚠️ |
| Complexity   | Medium ⚠️     | High ❌    | Low ✅     | Medium ⚠️    | High ❌    | High ❌   |
| Reliability  | High ✅       | Medium ⚠️  | High ✅    | Low ❌       | High ✅    | High ✅   |

## Implementation Details

### Build Script (`build.rs`)

```rust
use deno_ast::{parse_module, MediaType, ParseParams};
use std::fs;

fn main() {
    let specs_dir = "vendor/autocomplete/src";

    for entry in fs::read_dir(specs_dir)? {
        let path = entry?.path();
        if path.extension() == Some("ts") {
            let source = fs::read_to_string(&path)?;
            let parsed = parse_module(ParseParams {
                specifier: path.to_string(),
                text_info: source.into(),
                media_type: MediaType::TypeScript,
                capture_tokens: false,
                scope_analysis: false,
                maybe_syntax: None,
            })?;

            let spec = extract_completion_spec(&parsed.module());
            let msgpack = rmp_serde::to_vec(&spec)?;

            fs::write(
                format!("specs/{}.msgpack", path.stem()?),
                msgpack
            )?;
        }
    }
}
```

### Runtime Loading

```rust
pub struct SpecLoader {
    cache: LruCache<String, CompletionSpec>,
}

impl SpecLoader {
    pub fn load(&mut self, name: &str) -> Result<&CompletionSpec> {
        if !self.cache.contains(name) {
            let data = match name {
                "git" => include_bytes!("../specs/git.msgpack"),
                "npm" => include_bytes!("../specs/npm.msgpack"),
                // ... 600+ specs
            };

            let spec = rmp_serde::from_slice(data)?;
            self.cache.put(name.to_string(), spec);
        }

        Ok(self.cache.get(name).unwrap())
    }
}
```

### MessagePack Format

```rust
#[derive(Serialize, Deserialize)]
pub struct CompletionSpec {
    pub name: String,
    pub description: Option<String>,
    pub subcommands: Vec<Subcommand>,
    pub options: Vec<Option>,
    pub args: Vec<Argument>,
}
```

### Future Considerations

**Phase 3: Custom User Specs**

- Support `~/.config/autocomplete-rs/specs/` for user-defined specs
- Load dynamically at runtime (hybrid approach)
- User specs override embedded specs

**Spec Updates:**

- Check for spec updates on daemon start
- Download deltas from CDN
- Cache in `~/.cache/autocomplete-rs/`

**Hot Reloading (Development):**

- Watch mode that rebuilds on spec changes
- Only for development, not production

## References

- [deno_ast Documentation](https://docs.rs/deno_ast/)
- [MessagePack Specification](https://msgpack.org/)
- [Fig Autocomplete Specs](https://github.com/withfig/autocomplete)
- [Rust include_bytes! macro](https://doc.rust-lang.org/std/macro.include_bytes.html)

## Review Notes

Decision prioritizes:

1. **Performance** - Build-time parsing eliminates startup cost
2. **Reliability** - Build-time errors prevent runtime failures
3. **Simplicity** - Single binary deployment, no external dependencies

Trade-offs:

- Accept longer build times (~60s) for zero startup cost
- Accept ~15MB binary size for instant spec availability
- Defer user-defined specs to Phase 3+ (not needed for MVP)

The build-time approach ensures we meet our <20ms total latency requirement
while maintaining the simplicity of single-binary deployment.
