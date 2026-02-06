---
title: Workflows
description: Agent workflow patterns, TUI views, bd CLI integration, agent team coordination, and time travel
tags: [workflow, tui, bd, agent-teams, time-travel, integration, export]
---

# Workflows

## Agent Workflow Pattern

The standard agent workflow for using BV to find and claim work:

```bash
# 1. Start with triage
TRIAGE=$(bv --robot-triage)
NEXT_TASK=$(echo "$TRIAGE" | jq -r '.recommendations[0].id')

# 2. Check for cycles first (structural errors)
CYCLES=$(bv --robot-insights | jq '.cycles')
if [ "$CYCLES" != "[]" ]; then
  echo "Fix cycles first: $CYCLES"
fi

# 3. Claim the task
bd claim "$NEXT_TASK"

# 4. Work on it...

# 5. Close when done
bd close "$NEXT_TASK"
```

### Focused Workflow with Recipes

```bash
# Only show actionable (unblocked) work
bv --recipe actionable --robot-triage | jq '.recommendations[:3]'

# Find quick wins for momentum
bv --recipe quick-wins --robot-triage | jq '.quick_wins'

# Find and resolve bottlenecks first
bv --recipe bottlenecks --robot-triage | jq '.blockers_to_clear'
```

### Parallel Execution Planning

```bash
# Get parallel tracks for team coordination
bv --robot-plan | jq '.plan.tracks'

# Find the single highest-impact unblock
bv --robot-plan | jq '.plan.summary.highest_impact'
```

## Integration with bd CLI

BV reads from `.beads/beads.jsonl` created by the `bd` CLI:

```bash
bd init                    # Initialize beads in project
bd create "Task title"     # Create a bead
bd list                    # List beads
bd ready                   # Show actionable beads
bd claim bd-123            # Claim a bead
bd close bd-123            # Close a bead
```

BV provides the analytical layer on top of the data `bd` manages. Use `bd` for CRUD operations and `bv` for graph-aware analysis and triage.

## Integration with Agent Teams

When using Claude Code agent teams for parallel bead execution, BV provides the analytical layer for work distribution:

```bash
# Discover parallel tracks for team assignment
bv --robot-plan | jq '.plan.tracks'

# Find highest-impact unblock to assign first
bv --robot-plan | jq '.plan.summary.highest_impact'
```

The lead uses `bv --robot-plan` to identify independent tracks, then spawns teammates with bead-scoped prompts. See the beads-workflow skill's agent integration reference for the full execution workflow.

## Time Travel

Compare current project state against historical snapshots:

```bash
bv --as-of HEAD~10                    # 10 commits ago
bv --as-of v1.0.0                     # At tag
bv --as-of "2024-01-15"               # At date
bv --robot-diff --diff-since HEAD~30  # Changes in last 30 commits
```

Time travel applies to all robot commands. Combine with `--robot-insights` to track metric trends over time.

## TUI Views (for Humans)

When running `bv` interactively (not for agents):

| Key | View                                      |
| --- | ----------------------------------------- |
| `l` | List view (default)                       |
| `b` | Kanban board                              |
| `g` | Graph view (dependency DAG)               |
| `E` | Tree view (parent-child hierarchy)        |
| `i` | Insights dashboard (6-panel metrics)      |
| `h` | History view (bead-to-commit correlation) |
| `a` | Actionable plan (parallel tracks)         |
| `f` | Flow matrix (cross-label dependencies)    |
| `]` | Attention view (label priority ranking)   |

Agents must never launch the TUI. These views are documented here for completeness when human users reference them.
