---
name: handoff
description: Hand off to a fresh Claude session. Use when context is full, you've finished a logical chunk of work, or need a fresh perspective. New session recovers context via `bd prime`.
---

# Handoff - Session Cycling

Hand off your current session to a fresh Claude instance while preserving work context through beads and git.

## When to Use

- Context getting full (approaching token limit)
- Finished a logical chunk of work
- Need a fresh perspective on a problem
- Human requests session cycling

## Usage

```sh
/handoff [optional message]
```

## How It Works

1. Update in-progress bead notes with current state
2. File issues for any remaining work
3. Run quality gates (`mise run ci`)
4. Close completed issues
5. Commit and push all changes
6. Generate a handoff summary for the user
7. User starts a new session manually; `bd prime` recovers context

## Handoff Protocol

When invoked, execute the following steps:

### 1. Capture Current State

Update the active bead's notes with current progress:

```sh
bd update <active-id> --notes "$(cat <<'EOF'
## Status
- Stage: <current stage>
- Now: <what was being worked on>
- Next: <what should happen next>
- Blockers: <any blockers>
EOF
)"
```

If a handoff message was provided, add it as a comment:

```sh
bd comments add <active-id> "Handoff: <message>"
```

### 2. File Issues for Remaining Work

Create beads issues for any discovered or unfinished work:

```sh
bd create --title="<remaining work>" --type=task --priority=2
```

### 3. Run Quality Gates (if code changed)

```sh
mise run ci
```

Fix any issues before proceeding. Do not hand off broken code.

### 4. Close Completed Issues

```sh
bd close <id1> <id2> ... --reason="Completed during session"
```

### 5. Commit and Push

```sh
git add <changed-files>
git commit -m "<conventional commit message>"
git pull --rebase
git push
```

### 6. Generate Handoff Summary

Present a summary to the user:

```text
Handoff Summary
- Completed: <list of closed issues>
- In Progress: <list of open issues with current state>
- Created: <list of new issues filed>
- Next Steps: <what the next session should start with>

Start a new session — `bd prime` will recover full context.
```

## What Persists

- **Beads state**: All issues, dependencies, notes, progress
- **Git state**: Commits, branches, all pushed to remote
- **Handoff notes**: Captured in bead comments

## What Resets

- **Conversation context**: Fresh Claude instance
- **Session-scoped tasks**: TaskCreate items are ephemeral
- **In-memory state**: Any uncommitted analysis
