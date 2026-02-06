# Beads Command Reference for Retrospective Analysis

## Data Gathering Commands

### Get all issues from a specific epic

```sh
bd epic show <epic-id>
bd list --epic <epic-id> --json
```

### Get all issues with a specific label

```sh
bd list --labels <label-name> --json
```

### Get discovered issues (gaps found during execution)

```sh
bd list --labels discovered --json
```

### Get tech debt accumulation

```sh
bd list --labels tech-debt --json
```

### Get blocked issues (friction points)

```sh
bd blocked
```

### Get closed issues for analysis

```sh
bd list --status closed --json
```

### Get issues by priority

```sh
bd list --priority 0 --json  # Critical issues
bd list --priority 1 --json  # High priority
bd list --priority 2 --json  # Medium priority
```

### Get dependency information

```sh
bd dep tree <issue-id>  # Show full dependency tree
bd dep list <issue-id>  # List direct dependencies
```

## Project Health Commands

### Overall project statistics

```sh
bd stats
```

### Find stale issues

```sh
bd stale
```

### Check for blocked work

```sh
bd blocked
```

### Recent git activity (correlate with issue work)

```sh
git log --oneline --since="2 weeks ago"
```
