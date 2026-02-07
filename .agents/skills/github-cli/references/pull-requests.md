---
title: Pull Requests
description: GitHub CLI pull request creation, review workflows, merge strategies, checkout, checks, diff, auto-merge, and revert
tags: [gh-cli, pull-requests, review, merge, checks, auto-merge, revert, draft]
---

# Pull Requests

## List and Status

```sh
gh pr list
gh pr list --state merged --limit 20
gh pr list --label "needs-review" --assignee @me
gh pr list --base main --head feature/auth
gh pr list --draft
gh pr list --search "fix in:title"
gh pr list --json number,title,headRefName,mergeable
gh pr status
```

## View

```sh
gh pr view 45
gh pr view 45 --web
gh pr view 45 --json title,body,reviews,statusCheckRollup
gh pr diff 45
gh pr checks 45
gh pr checks 45 --json name,state,conclusion
```

## Create

```sh
gh pr create --title "Add feature" --body "Description"
gh pr create --fill
gh pr create --fill-first
gh pr create --fill-verbose
gh pr create --draft
gh pr create --base main --head feature/auth
gh pr create --reviewer user1,user2 --assignee @me
gh pr create --label "enhancement" --milestone "v2.0"
gh pr create --project "Sprint Board"
gh pr create --body-file pr-body.md
gh pr create --template pull_request_template.md
gh pr create --web
gh pr create --dry-run
```

Key flags:

- `--fill` -- auto-populate title and body from commits
- `--fill-first` -- use first commit for title, rest for body
- `--fill-verbose` -- include full commit messages in body
- `--draft` -- create as draft PR
- `--dry-run` -- preview without creating
- `--no-maintainer-edit` -- prevent maintainer edits on fork PRs

## Checkout

```sh
gh pr checkout 45
gh pr checkout 45 --force
gh pr checkout 45 --detach
```

## Edit

```sh
gh pr edit 45 --title "Updated title"
gh pr edit 45 --body "Updated description"
gh pr edit 45 --add-reviewer user1
gh pr edit 45 --remove-reviewer user2
gh pr edit 45 --add-label "ready" --remove-label "wip"
gh pr edit 45 --add-assignee @me
gh pr edit 45 --base develop
gh pr edit 45 --add-project "Sprint Board"
```

## Review

```sh
gh pr review 45 --approve
gh pr review 45 --approve --body "LGTM"
gh pr review 45 --request-changes --body "Please fix X"
gh pr review 45 --comment --body "Consider using Y instead"
```

## Review Threads (GraphQL)

Resolving review threads requires the GraphQL API -- there is no REST endpoint or `gh pr` subcommand for this.

**Important:** GraphQL queries containing `!` (e.g., `String!`) must be passed via file or `$(cat ...)` to avoid shell interpretation. Use `-f` for string variables and `-F` for typed variables (Int, Boolean).

### List review threads

```sh
# Save query to a file to avoid shell escaping issues with '!'
cat > /tmp/threads.graphql << 'QUERY'
query($owner: String!, $repo: String!, $pr: Int!) {
  repository(owner: $owner, name: $repo) {
    pullRequest(number: $pr) {
      reviewThreads(first: 100) {
        nodes {
          id
          isResolved
          isOutdated
          comments(first: 1) {
            nodes { body path line }
          }
        }
      }
    }
  }
}
QUERY

gh api graphql -F owner="owner" -F repo="repo" -F pr=45 \
  -f query="$(cat /tmp/threads.graphql)"
```

### Filter to outdated unresolved threads

```sh
gh api graphql -F owner="owner" -F repo="repo" -F pr=45 \
  -f query="$(cat /tmp/threads.graphql)" \
  --jq '.data.repository.pullRequest.reviewThreads.nodes[]
    | select(.isOutdated == true and .isResolved == false)
    | {id, path: .comments.nodes[0].path, snippet: .comments.nodes[0].body[:80]}'
```

### Reply to a thread

Use `addPullRequestReviewThreadReply` to reply inline within an existing thread. This is the correct way to respond to review comments — do NOT use `addComment` or create a new review, as those add standalone comments instead of threaded replies.

```sh
cat > /tmp/reply.graphql << 'QUERY'
mutation($threadId: ID!, $body: String!) {
  addPullRequestReviewThreadReply(
    input: { pullRequestReviewThreadId: $threadId, body: $body }
  ) {
    comment { id }
  }
}
QUERY

gh api graphql -f query="$(cat /tmp/reply.graphql)" \
  -f threadId="PRRT_kwDO..." \
  -f body="Fixed in abc1234 — description of what changed."
```

### Reply and resolve a thread

The most common workflow: reply with context, then resolve. Use both mutations in sequence.

```sh
# Reply first
gh api graphql -f query="$(cat /tmp/reply.graphql)" \
  -f threadId="PRRT_kwDO..." \
  -f body="Fixed — extracted helper to avoid duplication."

# Then resolve
gh api graphql -f query="$(cat /tmp/resolve.graphql)" \
  -f threadId="PRRT_kwDO..."
```

### Batch reply and resolve

```sh
# Reply to and resolve multiple threads
REPLY=$(cat /tmp/reply.graphql)
RESOLVE=$(cat /tmp/resolve.graphql)

for tid in PRRT_abc PRRT_def PRRT_ghi; do
  gh api graphql -f query="$REPLY" -f threadId="$tid" \
    -f body="Fixed — see commit abc1234."
  gh api graphql -f query="$RESOLVE" -f threadId="$tid"
done
```

### Resolve a single thread

```sh
cat > /tmp/resolve.graphql << 'QUERY'
mutation($threadId: ID!) {
  resolveReviewThread(input: { threadId: $threadId }) {
    thread { isResolved }
  }
}
QUERY

gh api graphql -f query="$(cat /tmp/resolve.graphql)" \
  -f threadId="PRRT_kwDO..."
```

### Batch resolve outdated threads

```sh
# Resolve all outdated unresolved threads in a loop
RESOLVE=$(cat /tmp/resolve.graphql)
gh api graphql -F owner="owner" -F repo="repo" -F pr=45 \
  -f query="$(cat /tmp/threads.graphql)" \
  --jq '.data.repository.pullRequest.reviewThreads.nodes[]
    | select(.isOutdated == true and .isResolved == false) | .id' \
| while read -r tid; do
    gh api graphql -f query="$RESOLVE" -f threadId="$tid"
  done
```

### Unresolve a thread

```sh
cat > /tmp/unresolve.graphql << 'QUERY'
mutation($threadId: ID!) {
  unresolveReviewThread(input: { threadId: $threadId }) {
    thread { isResolved }
  }
}
QUERY

gh api graphql -f query="$(cat /tmp/unresolve.graphql)" \
  -f threadId="PRRT_kwDO..."
```

### When to reply vs. resolve vs. new comment

- **Reply and resolve:** You fixed the issue or are deferring with an explanation — the reviewer deserves context
- **Resolve only:** The comment is outdated (code no longer exists) or already addressed by a prior commit
- **Reply only (don't resolve):** You disagree or need discussion before resolving
- **New PR comment** (`gh pr comment 45 --body "..."`): General communication that isn't a response to a specific review thread — status updates, summaries, questions for the reviewer

Thread fields reference:

- `id` -- node ID (starts with `PRRT_`), used in resolve/unresolve/reply mutations
- `isResolved` -- whether the thread is marked resolved
- `isOutdated` -- `true` when subsequent commits changed the lines the comment was on
- `comments.nodes[].path` -- file path the comment is on
- `comments.nodes[].line` -- line number (null if outdated/moved)

## Merge

```sh
gh pr merge 45 --squash
gh pr merge 45 --merge
gh pr merge 45 --rebase
gh pr merge --squash --delete-branch
gh pr merge --squash --subject "feat: add auth (#45)"
gh pr merge --squash --body "Detailed merge description"
```

### Auto-Merge

Enable auto-merge to merge automatically when all checks pass:

```sh
gh pr merge --auto --squash
gh pr merge --auto --squash --delete-branch
gh pr merge 45 --auto --rebase
gh pr merge --disable-auto
```

Auto-merge requires branch protection rules with required status checks enabled on the repository.

## Mark Ready for Review

```sh
gh pr ready 45
```

## Update Branch

Bring the PR branch up to date with the base branch:

```sh
gh pr update-branch 45
gh pr update-branch 45 --rebase
```

## Close and Reopen

```sh
gh pr close 45
gh pr close 45 --delete-branch
gh pr reopen 45
```

## Lock and Unlock

```sh
gh pr lock 45 --reason "resolved"
gh pr unlock 45
```

## Revert

Create a revert PR for a merged pull request:

```sh
gh pr revert 45
gh pr revert 45 --body "Reverting due to regression"
```

## Comment

```sh
gh pr comment 45 --body "Updated the implementation"
gh pr comment 45 --body-file review-notes.md
```

## Quick PR Workflow

```sh
git checkout -b feature/my-feature
git add -A && git commit -m "feat: add feature"
git push -u origin feature/my-feature
gh pr create --fill
```

## Review and Merge Workflow

```sh
gh pr checkout 45
gh pr diff
gh pr review --approve
gh pr merge --squash --delete-branch
```
