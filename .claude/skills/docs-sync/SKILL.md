---
name: docs-sync
description: Documentation sync after code review. Use when resolving PR review comments, fixing code from review feedback, adding new files or components, or changing architectural patterns.
---

# Docs Sync — Propagate Review Learnings

After resolving code review feedback (Copilot reviews, `/pr-review-toolkit:review-pr`,
manual review comments), check whether the fix reveals something that should be written
down. Not every fix needs a docs update — only propagate when you learned something new.

## Decision Framework

Ask: **"Did we discover a pattern, constraint, or architectural fact that someone would
need to know next time?"** If yes, update the appropriate layer below. If the fix was
just a typo or obvious bug, skip this entirely.

## Layer 1: Rules (`.claude/rules/*.md`)

**When to update:** A review comment revealed a coding pattern, convention, or constraint
that should apply to future work in the same area.

Examples:

- Review caught a missing `drop()` before socket rebind → add to `daemon.md`
- Review flagged incorrect error handling pattern → codify in `rust.md`
- New file or module added that should trigger a rule → update `paths:` frontmatter

**How:**

- Add the pattern as a concise rule in the relevant rule file
- If a new file path was added, check whether any rule's `paths:` glob should include it
- Keep rules actionable ("Do X" / "Don't do Y"), not descriptive

## Layer 2: Architecture (`AGENTS.md`)

**When to update:** The codebase structure changed in a way that affects how someone
navigates or understands the project.

Examples:

- New file or module added (add to the Architecture section listing)
- Module responsibilities shifted during review-driven refactor
- Spike results changed understanding of a component's role
- New public API surface or trait added

**How:**

- Update the relevant bullet in the Architecture section
- Keep descriptions factual and current-state (not aspirational)
- Match the existing style: `**Name** (\`path\`) — description`

## Layer 3: Project Docs (`docs/`)

**When to update:** Review revealed information that belongs in long-lived documentation
rather than ephemeral rule files.

Examples:

- Spike results that should update or create an ADR
- Design decisions made during review that affect `docs/design/` specs
- New setup steps or gotchas for `docs/development/`
- Findings that contradict or extend existing ADR decisions

**How:**

- ADRs are immutable once accepted — create a new ADR to supersede
- Design docs should reflect current reality (see `documentation.md` rule)
- Don't update docs speculatively — only when review produced concrete evidence

## What NOT to Do

- Don't update docs for every minor fix — only when there's a genuine learning
- Don't create new rule files for one-off issues — add to existing rules
- Don't duplicate information across layers — pick the most appropriate one
- Don't update aspirationally — only document what is true now
