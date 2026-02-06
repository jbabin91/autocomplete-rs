---
title: Agent Integration
description: bd CLI commands, robot mode for agents, Claude Code agent teams for parallel execution, and test coverage bead creation
tags: [bd, bv, cli, robot-mode, agent-teams, test-coverage, swarm]
---

# Agent Integration

## bd CLI Basics

```bash
bd init

bd create "Implement user authentication" -t feature -p 1

bd depend BD-123 BD-100  # BD-123 depends on BD-100

bd update BD-123 --status in_progress

bd close BD-123 --reason "Completed and tested"

bd ready --json
```

Key commands:

| Command                                 | Purpose                                  |
| --------------------------------------- | ---------------------------------------- |
| `bd init`                               | Initialize beads in a project            |
| `bd create "title" -t type -p priority` | Create a new bead                        |
| `bd depend <issue> <depends-on>`        | Declare that issue depends on depends-on |
| `bd update <id> --status <status>`      | Change bead status                       |
| `bd close <id> --reason "reason"`       | Close a completed bead                   |
| `bd ready --json`                       | List beads with no unresolved blockers   |

## Robot Mode for Agents

**Never run bare `bv`** -- it launches an interactive TUI that blocks agents. Always use `--robot-*` flags.

```bash
bv --robot-triage

bv --robot-next

bv --robot-plan

bv --robot-insights
```

| Flag               | Output                                        |
| ------------------ | --------------------------------------------- |
| `--robot-triage`   | Triage recommendations for all open beads     |
| `--robot-next`     | The single highest-priority unblocked bead    |
| `--robot-plan`     | Parallel execution tracks for swarm agents    |
| `--robot-insights` | Graph analysis: PageRank, bottlenecks, cycles |

Check for cycles before implementation:

```bash
bv --robot-insights | jq '.Cycles'
```

An empty result means the dependency graph is clean.

## Claude Code Agent Teams

When multiple beads can be worked in parallel, use Claude Code agent teams to coordinate execution. Agent teams let teammates work independently in separate context windows while sharing a task list for coordination.

### When to Use Teams vs Subagents

| Scenario                                   | Use            | Why                                                 |
| ------------------------------------------ | -------------- | --------------------------------------------------- |
| Independent beads touching different files | Agent team     | Teammates work in parallel without conflicts        |
| Research spike with competing hypotheses   | Agent team     | Teammates debate and challenge each other           |
| Bulk work across many similar beads        | Agent team     | Embarrassingly parallel; each teammate owns a batch |
| Quick focused task (lint fix, single bead) | Subagent       | Lower overhead, result returns to caller            |
| Sequential dependency chain                | Single session | No parallelism to exploit                           |
| Multiple beads touching the same files     | Single session | Avoids file conflict overwrites                     |

### Execution Workflow

The lead uses beads as the source of truth and team tasks as the dispatch mechanism.

**Step 1 -- Discover parallel tracks:**

```bash
bd ready --json
# or
bv --robot-plan
```

Identify beads that have no mutual dependencies and touch different files.

**Step 2 -- Create team and session tasks:**

The lead creates a team and maps each parallelizable bead to a session task:

```text
TeamCreate → creates shared task list
TaskCreate for each bead being dispatched (include bead ID in task description)
```

**Step 3 -- Spawn teammates with context:**

Each teammate gets a spawn prompt that includes:

- The bead ID and its full description
- File ownership boundaries (which files they can touch)
- The project's AGENTS.md conventions
- Verification criteria from the bead

Example spawn prompt:

```text
You are working on bead BD-123: "Implement command buffer parser".
Your file scope: src/parser/**
Read AGENTS.md for project conventions.
When done, run the verification commands from the bead's acceptance criteria.
Mark your team task complete when finished.
```

**Step 4 -- Teammates work and track progress:**

Each teammate:

1. Claims bead: `bd update BD-123 --status in_progress`
2. Implements the work within their file scope
3. Updates bead notes with progress: `bd update BD-123 --notes "..."`
4. Runs verification from the bead's acceptance criteria
5. Marks team task complete: `TaskUpdate` with status `completed`

**Step 5 -- Lead verifies and closes beads:**

After teammates complete their tasks:

1. Review each teammate's work
2. Run `mise run ci` to verify everything integrates
3. Close verified beads: `bd close BD-123 --reason "Completed and verified"`
4. Shut down teammates and clean up the team

### File Ownership Convention

To prevent conflicts, assign each teammate exclusive file ownership:

```text
Teammate A (parser):     src/parser/**
Teammate B (daemon):     src/daemon/**
Teammate C (shell):      shell-integration/**
```

If two beads need to touch the same file, work them sequentially or have one teammate own both.

### Beads vs Team Tasks

| Layer      | Tool                      | Lifetime                | Purpose                   |
| ---------- | ------------------------- | ----------------------- | ------------------------- |
| Persistent | Beads (`bd`)              | Across sessions         | What needs to be done     |
| Ephemeral  | Team tasks (`TaskCreate`) | Within a single session | Who is doing it right now |

Beads are the source of truth. Team tasks are the dispatch mechanism. A bead may span multiple sessions; a team task lives and dies within one.

## Test Coverage Beads

Use the following prompt to audit test coverage and create beads for any gaps.

```text
Do we have full unit test coverage without using mocks/fake stuff? What about complete e2e integration test scripts with great, detailed logging? If not, then create a comprehensive and granular set of beads for all this with tasks, subtasks, and dependency structure overlaid with detailed comments.
```

This prompt:

- Audits existing test coverage across the project
- Identifies gaps in both unit tests and e2e tests
- Creates granular beads for missing test coverage with full dependency structure
- Emphasizes real implementations over mocks and detailed logging for debugging
