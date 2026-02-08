# Completion History — Design Spec

> **This is a design specification, not documentation.** It describes the
> intended completion history design. Actual documentation will be written
> after implementation.

## Overview

Track which completions users select and use that data to rank future
suggestions by frequency. This makes the autocomplete engine learn user
habits — frequently used subcommands, flags, and arguments float to the
top.

## Problem

Without history-based ranking, suggestions are ordered by spec definition
(alphabetical or author-defined). This means:

- `git checkout` always shows branches in the same order
- Rarely-used subcommands rank equally with daily-use ones
- The user repeatedly scrolls past irrelevant suggestions
- No personalization — every user sees identical ordering

Fig.io never solved this well. It's a differentiator.

## Design

### Storage Schema (v2 migration)

```sql
CREATE TABLE completion_selections (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    command     TEXT NOT NULL,     -- root command, e.g. "git"
    subcommand  TEXT,              -- e.g. "checkout", "commit"
    selected    TEXT NOT NULL,     -- the suggestion the user picked
    context     TEXT,              -- optional: cwd, prior args, etc.
    selected_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_selections_frequency
    ON completion_selections(command, subcommand, selected);
CREATE INDEX idx_selections_recency
    ON completion_selections(selected_at);
```

### Recording Selections

When the shell widget inserts a completion, it sends a signal back to the
daemon indicating which suggestion was selected:

```rust
StorageEvent::CompletionSelected {
    command: String,
    subcommand: Option<String>,
    selected: String,
    context: Option<String>,
}
```

This goes through the existing actor channel — fire-and-forget via
`try_send()` on the hot path (same pattern as diagnostics).

### Ranking Query

When generating suggestions, the engine queries frequency data to reorder
results:

```sql
SELECT selected, COUNT(*) as frequency
FROM completion_selections
WHERE command = ?1
  AND (subcommand = ?2 OR subcommand IS NULL)
GROUP BY selected
ORDER BY frequency DESC
```

The result is a `HashMap<String, u32>` used as a sort key. Suggestions
matching high-frequency entries sort first; unmatched suggestions retain
their spec-defined order.

### Ranking Algorithm

```text
score(suggestion) = spec_priority * 1.0
                  + frequency_count * frequency_weight
                  + recency_bonus(last_used)

where:
  frequency_weight = 0.5  (tunable)
  recency_bonus    = 1.0 if used in last hour, 0.5 if today, 0.0 otherwise
```

The weights are configurable (Phase 3, configuration system). Initial
implementation can use a simpler approach: just sort by frequency count,
with spec order as tiebreaker.

### Read Path

Same approach as generator cache — a separate read-only connection held
by the completion engine. The frequency query runs once per completion
request (not per suggestion), so latency is bounded.

Expected performance: <1ms for the GROUP BY query on typical data volumes
(thousands of rows, indexed).

### Data Lifecycle

- **Retention:** Keep 90 days of history by default
- **Periodic cleanup:** delete old rows monthly

  ```sql
  DELETE FROM completion_selections
  WHERE selected_at < datetime('now', '-90 days')
  ```

- **Data volume:** ~10-50 selections per day for an active user =
  ~1,500-4,500 rows per 90-day window. Trivial for SQLite.

### Privacy

Completion history records **command names and selected suggestions**, not
full command lines or arguments with values. For example:

- Recorded: `command=git, subcommand=checkout, selected=main`
- NOT recorded: `git checkout main` (full command with context)

Sensitive arguments (passwords, tokens, file contents) never appear in
suggestions in the first place — they come from generators, not from
history. The history table only stores the suggestion text that was
already displayed to the user.

Users can clear history via:

```sh
autocomplete-rs history clear           # clear all
autocomplete-rs history clear --command git  # clear for one command
```

### Shell Integration

The ZLE widget needs modification to report selections back:

```zsh
# After inserting the completion:
autocomplete-rs record-selection \
    --command "$_autocomplete_command" \
    --selected "$_autocomplete_selected"
```

This is a fire-and-forget CLI call (or a message on the Unix socket).
Latency doesn't matter — it runs after the completion is already inserted.

## Integration Points

- **Storage schema** (`src/storage/schema.rs`) — v2 migration adds table
- **Storage events** (`src/storage/events.rs`) — new
  `CompletionSelected` variant
- **Storage actor** (`src/storage/actor.rs`) — handle new event
- **Completion engine** — read frequency data to reorder suggestions
- **Shell integration** (`shell-integration/zsh.zsh`) — report selections
- **CLI** — `history` subcommand for management

## Dependencies

- Requires working completion flow (parser + dropdown) to have selections
  to record
- Schema migration infrastructure already exists
- Shell integration widget already exists (needs selection reporting)

## Future Extensions

- **Contextual frequency** — rank by (command + cwd), so `git checkout`
  in repo A ranks different branches than in repo B
- **Temporal patterns** — "I always run `docker-compose up` in the
  morning" → time-of-day weighting
- **Command sequences** — "after `git add .` I usually run `git commit`"
  → suggest the next command
- **Export/import** — share history across machines

## Related Documents

- [Parser Architecture](parser.md) — section 9.2 "Learning from Usage"
- [Daemon Architecture](daemon.md) — storage integration
- [Architecture Overview](overview.md) — system context
