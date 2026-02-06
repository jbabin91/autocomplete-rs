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
