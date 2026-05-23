---
name: ppt-pr-followup
description: After a PR is pushed, monitor its CI checks and review comments, then fix and address them in a loop until the PR is green and uncontested. Triggers on phrases like "follow up on the PR", "fix the CI", "address review comments", or runs automatically right after `ppt-pr-create`.
when_to_use: A PR has been opened on the current branch (or one you specify) and you want to close out the post-push loop — CI failures resolved, reviewer comments answered, branch ready to merge. Sister skill: `ppt-pr-create` (opens the PR; this skill closes it out).
mode: both
capabilities: [C6]
tags: [workflow, ci, review]
---

# PPT PR Follow-up

End-to-end choreography for the post-push portion of the PR lifecycle. Assumes
the branch is already pushed (typically by `ppt-pr-create`); this skill drives
the iterative loop until the PR is mergeable.

## When to invoke

- Right after `gh pr create` returns a URL
- A reviewer left comments and you want them addressed
- CI has gone red and you want the loop to diagnose + fix + push
- The user says: *"check on the PR"*, *"fix the CI"*, *"address the review"*,
  *"work the PR queue"*

**Do not** invoke for one-off questions ("did CI pass?"). For a single status
check use `gh pr checks` directly.

## What it does

A single iteration:

1. **Identify the PR.** Default to the PR for the current branch.
2. **Snapshot CI + review state.** Pull `gh pr checks` and review threads.
3. **Triage.** Categorize each failing check and each unresolved comment.
4. **Fix.** Edit the worktree to resolve the highest-priority item. One fix
   per loop iteration — easier to review, easier to revert.
5. **Verify locally** with the smallest correct test (`ppt-tests` rules apply).
6. **Commit + push** to the PR branch.
7. **Reply to comments** for items you addressed (or for which you decided
   not to act — every comment gets a reply).
8. **Loop** until either:
   - All checks green AND no open review threads, OR
   - You hit an item that requires user judgment (auth model change,
     architectural call, intentional skip)

Then surface a short status report.

## Inputs

- `PR` — PR number (optional). If omitted, derive from the current branch via
  `gh pr view --json number,headRefName`.
- `MAX_ITERS` — safety cap (default 10). Stops runaway loops.
- `SCOPE` — `ci-only` | `reviews-only` | `both` (default `both`).

## Iteration recipe

### Step 1 — Identify the PR

```bash
PR=$(gh pr view --json number -q .number 2>/dev/null) || {
  echo "No PR for the current branch; nothing to follow up on." >&2
  exit 0
}
BRANCH=$(gh pr view "$PR" --json headRefName -q .headRefName)
git rev-parse --abbrev-ref HEAD | grep -qx "$BRANCH" || {
  echo "FATAL: local branch ($(git rev-parse --abbrev-ref HEAD)) does not match PR head ($BRANCH)." >&2
  exit 1
}
```

If the local branch and the PR's head ref disagree, **stop and ask the user** —
pushing the wrong branch to the PR will surprise reviewers.

### Step 2 — Snapshot CI state

```bash
gh pr checks "$PR" --json name,bucket,state,detailsUrl --jq '
  .[] | select(.bucket=="fail" or .bucket=="cancel") |
  {name, state, url: .detailsUrl}'
```

For each failing check:
- Fetch the last 200 lines of its log:
  ```bash
  # Find the run id from the workflow URL; details URL looks like:
  #   https://github.com/owner/repo/actions/runs/<id>/job/<job_id>
  RUN_ID=$(echo "$URL" | grep -oE 'runs/[0-9]+' | head -1 | cut -d/ -f2)
  JOB_ID=$(echo "$URL" | grep -oE 'job/[0-9]+'  | head -1 | cut -d/ -f2)
  gh run view "$RUN_ID" --log-failed --job "$JOB_ID" | tail -200
  ```
- Identify the *first* hard error in the log — the rest is usually cascade.
- Map error → fix (see *Common CI failure patterns* below).

### Step 3 — Snapshot review state

```bash
# Top-level review verdicts (Approved / Changes requested / Commented)
gh pr view "$PR" --json reviews -q '
  .reviews[] | {state, author: .author.login, body, submittedAt}'

# Inline / file-anchored comments (NOT included in `reviews` JSON)
OWNER=$(gh repo view --json owner -q .owner.login)
REPO=$(gh repo view --json name  -q .name)
gh api "repos/$OWNER/$REPO/pulls/$PR/comments" --jq '
  .[] | select(.in_reply_to_id == null) |
  {id, path, line, body, user: .user.login, created_at}'

# Conversation / general thread comments on the PR
gh api "repos/$OWNER/$REPO/issues/$PR/comments" --jq '
  .[] | {id, body, user: .user.login, created_at}'
```

**Three different REST surfaces** for three different comment types — don't
miss inline comments by only reading the `reviews` array.

To know which threads are *resolved* (newer review API), use GraphQL:

```bash
gh api graphql -f query='query($owner:String!,$repo:String!,$pr:Int!){
  repository(owner:$owner,name:$repo){
    pullRequest(number:$pr){
      reviewThreads(first:100){
        nodes{ id isResolved isOutdated comments(first:10){nodes{body path line author{login}}} }
      }
    }
  }
}' -f owner="$OWNER" -f repo="$REPO" -F pr="$PR"
```

Filter to `isResolved == false`. Outdated threads still need a reply (or an
explicit *resolve*).

### Step 4 — Triage

Build one list, sorted by priority:

| Priority | Source | Items |
|---|---|---|
| P0 | CI | Hard failures (compile, test, lint) |
| P0 | Review | "Request changes" verdicts |
| P1 | CI | Soft failures (deprecation warnings promoted to errors, slow checks) |
| P1 | Review | Inline blocker comments ("this leaks tokens", "this is wrong") |
| P2 | Review | Suggestions ("nit:", "consider:") |
| P3 | Review | Questions ("why this approach?") — reply, don't necessarily edit |

Work P0 → P1 → P2. **P3 gets a reply, not a commit** (unless the question
exposes a real bug).

### Step 5 — Fix one item per iteration

For each item, decide:
- **Act**: edit the worktree to fix
- **Defer**: open a follow-up issue (`gh issue create`) and reply with the
  link
- **Push back**: reply explaining why the comment doesn't apply (gather
  evidence first — quote line numbers, link prior decisions in CLAUDE.md)

**One fix per commit.** Don't bundle. Bundled fix commits make later
bisecting / revert painful.

### Step 6 — Verify locally

Use the smallest correct check from `ppt-tests`:
- Rust file edited → `cargo check -p <crate>` then `cargo test -p <crate> --lib`
- Frontend file edited → `pnpm -F <pkg> test:run -- <file>`
- Mobile native → `./gradlew :shared:build` (local-only)
- Schema edited → `sqlx migrate run` against ephemeral DB

**Never push without a local green** — round-trip latency on CI is too high
to debug there.

### Step 7 — Commit + push

Follow root `CLAUDE.md` commit conventions:

```bash
git add <specific files>
git commit -m "fix(<scope>): address review comment <#nnn> — <terse summary>"
git push
```

Force-push only if you're rewriting your own un-reviewed commits **and**
you're certain no one else has based work on this branch. Otherwise prefer
a new commit on top.

### Step 8 — Reply to comments

Every comment you addressed (or chose not to) gets a reply. Two channels:

```bash
# Reply to a specific inline comment (creates a threaded reply)
gh api -X POST "repos/$OWNER/$REPO/pulls/$PR/comments/$COMMENT_ID/replies" \
  -f body="Fixed in $(git rev-parse --short HEAD). $REASON"

# Reply to a top-level review or general PR conversation
gh pr comment "$PR" --body "Addressed:
- <thread A>: fixed in <sha>
- <thread B>: deferred to #<issue>
- <thread C>: see <link> — keeping as-is because <reason>"

# Resolve a thread via GraphQL once the fix lands
gh api graphql -f query='mutation($id:ID!){
  resolveReviewThread(input:{threadId:$id}){thread{isResolved}}
}' -f id="$THREAD_ID"
```

Re-request review when you've cleared everything:

```bash
gh pr ready "$PR"      # if it was draft and is now ready
gh api -X POST "repos/$OWNER/$REPO/pulls/$PR/requested_reviewers" \
  -f reviewers='["<reviewer-login>"]'
```

### Step 9 — Loop

Re-snapshot. CI runs again after every push, so wait for it to settle
before declaring done.

```bash
gh pr checks "$PR" --watch --interval 30
```

Bail out at `MAX_ITERS` to avoid runaway loops. On bail-out, surface a
status report and ask the user how to proceed.

## Common CI failure patterns

| Symptom in log | Likely cause | Fix |
|---|---|---|
| `error[E0...]` from cargo | type/borrow error in the diff | read the diff, fix the call site |
| `error: linking with cc failed` | linker pinned to clang/lld but absent on runner | shim with `CARGO_TARGET_*_LINKER=gcc` or fix `.cargo/config.toml` |
| `error: could not compile ... due to previous error` | look ABOVE — the first error is the real one | fix that one, cascade clears |
| `test ... FAILED` panic with `assertion failed` | logic regression OR test asserting old behavior | check git blame on the test; if it asserts removed behavior, update the test |
| `error: process completed with exit code 1` (no detail) | re-fetch with `--log` not `--log-failed` | sometimes the error went to stdout, not stderr |
| `Resource not accessible by integration` | workflow permissions missing | check the workflow's `permissions:` block |
| pnpm `ERR_PNPM_OUTDATED_LOCKFILE` | added/changed a dep without updating lockfile | `pnpm install --no-frozen-lockfile`, commit lockfile |
| `flaky` / passes on rerun | retry once with `gh run rerun --failed`; if still flaky, file a follow-up |

## Common review-comment patterns

| Comment shape | How to respond |
|---|---|
| *"This unwrap can panic"* → fix in place with `?` or `.ok_or(...)` |
| *"This leaks the token in logs"* → redact: `%user.email` → `%user_id` or hash the value |
| *"Why this approach over X?"* → reply with the design constraint; only edit if X is genuinely better |
| *"nit: rename `foo` to `bar`"* → rename in next commit; cheap; don't push back |
| *"Add a test for the error path"* → write it; commit as `test:` then `fix:` (IG3 pattern) |
| *"This conflicts with #NNN"* → check the linked PR / issue; resolve or coordinate with author |
| *"Did you mean to leave this `console.log`?"* → remove it; never argue with a left-in debug print |

## Smoke check

Sub-30s, exit 0 when the skill's tooling is wired up:

```bash
gh auth status >/dev/null 2>&1 && \
  command -v jq >/dev/null 2>&1 && \
  test -f .github/workflows/backend.yml
```

(Requires `gh` authed against `martin-janci/property-management`, `jq`
on PATH, and the workflow files to exist.)

## Limits and non-goals

- This skill does **not** open the PR. Use `ppt-pr-create` for that.
- It does **not** re-trigger flaky CI infinitely. Two reruns max, then bail.
- It does **not** auto-merge. Even after green + replies, a human merges.
- It does **not** rebase onto `main` automatically. If `main` has moved and
  conflicts exist, surface the conflict list and stop.
- It does **not** push to anyone else's branch. If the PR's head ref isn't
  yours, stop.

## Related skills

- [`ppt-pr-create`](../ppt-pr-create/SKILL.md) — opens the PR this skill follows up on
- [`ppt-tests`](../ppt-tests/SKILL.md) — pick the smallest correct verify command
- [`ppt-research-flow`](../ppt-research-flow/SKILL.md) — the end-to-end plan→PR→archive flow this slots into
