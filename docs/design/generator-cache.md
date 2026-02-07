# Generator Cache — Design Spec

> **This is a design specification, not documentation.** It describes the
> intended generator cache design. Actual documentation will be written
> after implementation.

## Overview

Fig completion specs define **generators** — shell commands that produce
dynamic completions (git branches, docker containers, npm scripts, etc.).
These commands are expensive (10-100ms each) and their output changes
infrequently. Caching generator results in SQLite lets them survive daemon
restarts and avoids redundant shell invocations.

## Problem

Without caching:

- `git branch --no-color` runs on **every** completion trigger
- Docker/npm generators are even slower (50-200ms)
- Daemon restart loses all cached state
- Multiple completions for the same command re-run identical generators

The parser design doc mentions a 1-second in-memory TTL, but that's too
aggressive for data that changes rarely (branches, containers) and too
volatile (lost on restart).

## Design

### Storage Schema (v2 migration)

```sql
CREATE TABLE generator_cache (
    command       TEXT NOT NULL,   -- e.g. "git"
    generator_key TEXT NOT NULL,   -- e.g. "branches" or hash of generator spec
    context_hash  TEXT,            -- hash of (cwd, env vars, etc.) for context-sensitive generators
    output        BLOB NOT NULL,   -- MessagePack-encoded Vec<Suggestion>
    cached_at     INTEGER NOT NULL, -- Unix timestamp (seconds)
    ttl_seconds   INTEGER NOT NULL, -- per-generator TTL from spec or default
    PRIMARY KEY (command, generator_key, context_hash)
);

CREATE INDEX idx_cache_expiry ON generator_cache(cached_at);
```

### TTL Strategy

Generators have different freshness requirements:

| Generator type | Default TTL | Rationale                                |
| -------------- | ----------- | ---------------------------------------- |
| Git branches   | 30s         | Changes on push/fetch, not per-keystroke |
| Docker images  | 60s         | Rarely changes during a session          |
| npm scripts    | 300s        | Changes only on package.json edit        |
| File listing   | 5s          | Filesystem changes frequently            |
| Default        | 10s         | Safe middle ground                       |

Specs can override via a `cache` field on the generator. If not specified,
use the default.

### Lookup Flow

```text
1. Parser identifies generator needed for current context
2. Compute (command, generator_key, context_hash)
3. Query cache:
   SELECT output FROM generator_cache
   WHERE command = ? AND generator_key = ? AND context_hash = ?
     AND cached_at + ttl_seconds > unixepoch()
4. If hit → deserialize and return (0ms)
5. If miss → execute generator, store result, return
```

### Write Path

Generator cache writes go through the existing storage actor channel:

```rust
StorageEvent::CacheGenerator {
    command: String,
    generator_key: String,
    context_hash: Option<String>,
    output: Vec<u8>,  // MessagePack-encoded suggestions
    ttl_seconds: u32,
}
```

### Read Path

Cache reads need to be **synchronous on the hot path** — the completion
engine can't await a round-trip through the actor channel. Options:

1. **Separate read-only connection** — the engine holds its own
   `Connection` for cache lookups (reads don't conflict with the actor's
   writes in SQLite WAL mode)
2. **In-memory LRU backed by SQLite** — on daemon start, load recent
   cache entries into a `HashMap`; SQLite is the persistence layer, memory
   is the hot path

Option 1 is simpler and sufficient. SQLite reads from a local file are
<1ms. Only move to option 2 if profiling shows cache lookups are a
bottleneck.

### Eviction

- **On read:** expired entries are ignored (TTL check in WHERE clause)
- **Periodic cleanup:** every 10 minutes, delete expired rows:

  ```sql
  DELETE FROM generator_cache
  WHERE cached_at + ttl_seconds < unixepoch()
  ```

- **On daemon start:** prune entries older than 24 hours regardless of TTL

### Context Hashing

Some generators produce different output depending on context:

- **Working directory** — `git branch` in repo A vs repo B
- **Environment variables** — `$DOCKER_HOST` affects container listing
- **Current arguments** — generator output may depend on prior args

The `context_hash` is a SHA-256 of the relevant context fields. The spec
declares which context fields matter for each generator. If none are
declared, `context_hash` is NULL (global cache).

## Integration Points

- **Storage schema** (`src/storage/schema.rs`) — v2 migration adds table
- **Storage events** (`src/storage/events.rs`) — new `CacheGenerator` variant
- **Storage actor** (`src/storage/actor.rs`) — handle new event
- **Completion engine** — read-only connection for cache lookups
- **Generator executor** — write cache after execution (depends on
  `autocomplete-rs-t94`)

## Dependencies

- Requires generator execution (`autocomplete-rs-t94`) to produce data
- Schema migration infrastructure already exists (v1 → v2)
- Storage actor channel pattern already handles batched writes

## Related Documents

- [Parser Architecture](parser.md) — generator execution design
- [Daemon Architecture](daemon.md) — storage integration
- [Architecture Overview](overview.md) — system context
