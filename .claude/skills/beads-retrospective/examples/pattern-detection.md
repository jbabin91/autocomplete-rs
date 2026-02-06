# Pattern Detection Examples

## A) Repeated Discoveries Across Work Streams

```markdown
epic:add-auth → bd-50: Add rate limiting (discovered)
epic:add-payments → bd-89: Add rate limiting (discovered)
epic:add-exports → bd-134: Add rate limiting (discovered)

PATTERN: Rate limiting is consistently missing from planning
INSIGHT: Need cross-cutting concern epic for "API Rate Limiting Infrastructure"
ACTION: bd create --title="API rate limiting infrastructure" --type=feature --priority=1 --labels retro
```

## B) Tech Debt Accumulation

```markdown
bd-51: TODO: Refactor auth middleware (closed but marked tech-debt)
bd-52: TODO: Extract validation logic (closed but marked tech-debt)
bd-53: TODO: Add retry logic (closed but marked tech-debt)

PATTERN: 3 refactoring todos created in add-auth epic
INSIGHT: Original epic was too aggressive on timeline, created shortcuts
ACTION: bd create --title="Auth system refactor: address tech debt from add-auth" --type=task --priority=2 --labels retro,tech-debt
```

## C) Scope Creep Patterns

```markdown
epic:add-auth had 8 planned tasks
Final execution: 14 issues (6 discovered during work)

PATTERN: Discovered 75% more work than planned
INSIGHT: Auth epics need better templates
ACTION: bd create --title="Create auth feature checklist template" --type=task --priority=3 --labels retro
```

## D) Recurring Blockers

```markdown
bd-60: Blocked by CI flakiness (2 days)
bd-75: Blocked by CI flakiness (1 day)
bd-90: Blocked by missing test fixtures (3 days)

PATTERN: CI reliability blocking multiple issues across epics
INSIGHT: Infrastructure gap causing repeated delays
ACTION: bd create --title="Fix CI flakiness causing recurring blocks" --type=bug --priority=1 --labels retro,infra
```
