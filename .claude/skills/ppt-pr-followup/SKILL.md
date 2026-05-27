---
name: ppt-pr-followup
description: Close out a PR's post-push loop. Two modes — (a) `dispatcher` — read the dispatcher's reviewer verdict in assignments.json and respawn the original implementer on `verdict=changes` (called by ppt-research-dispatcher after Phase 5 returns changes); (b) `manual` — implementer-driven CI + review-comment loop on the current branch (called right after ppt-pr-create or by a human). Triggers on "follow up on the PR", "fix the CI", "address review comments", "respawn implementer".
when_to_use: Dispatcher mode — a `status=review` row in `.research/management/assignments.json` has a `reviewer_summary.verdict=changes` and you want to respawn the original specialist to address the blocking findings. Manual mode — a PR you authored is open and you want to drive CI failures + reviewer comments to green. Sister skills: `ppt-pr-create` (opens the PR), `ppt-pr-merge` (lands a green, approved PR).
mode: both
capabilities: [C6]
tags: [workflow, ci, review, dispatcher]
---

# PPT PR Follow-up

End-to-end choreography for the post-push portion of the PR lifecycle. Two
distinct entry points share this skill:

- **`mode=dispatcher`** — the `ppt-research-dispatcher` routine calls this
  after Phase 5 when a reviewer subagent returns `verdict=changes`. The skill
  reads `.research/management/assignments.json`, picks up the original task
  context, and re-spawns the same specialist with the reviewer's blocking
  notes as the new brief. Hard-capped at 3 fix rounds per task.
- **`mode=manual`** — a human or `ppt-pr-create` invokes this on a fresh
  PR to drive CI failures + reviewer comments to green via in-place fixes
  on the PR branch.

Default is `manual` for backward compatibility; the dispatcher passes
`mode=dispatcher`.

## When to invoke

- **Dispatcher mode** — `assignments.json` row has
  `status=review` AND `reviewer_summary` starts with `verdict=changes`; the
  dispatcher calls this after Phase 5 to drive the fix loop.
- Right after `gh pr create` returns a URL (manual mode)
- A reviewer left comments and you want them addressed (manual mode)
- CI has gone red and you want the loop to diagnose + fix + push (manual mode)
- The user says: *"check on the PR"*, *"fix the CI"*, *"address the review"*,
  *"work the PR queue"*, *"respawn implementer"*

**Do not** invoke for one-off questions ("did CI pass?"). For a single status
check use `gh pr checks` directly.

## Inputs

| Input        | Default     | Notes                                                  |
| ---          | ---         | ---                                                    |
| `mode`       | `manual`    | `dispatcher` \| `manual`                               |
| `pr_number`  | (required for `dispatcher`; derived from branch in `manual`) | The PR to follow up on |
| `repo`       | `martin-janci/property-management` | Full GH slug                         |
| `MAX_ITERS`  | `10` (manual only) | Safety cap on manual loop                       |
| `SCOPE`      | `both`      | `ci-only` \| `reviews-only` \| `both` (manual mode)    |

## Dispatcher mode — state machine

The dispatcher hands this skill a `pr_number` (or branch) and expects a
one-line return contract. The skill MUST NOT loop or interact — it reads
state, decides one action, mutates `assignments.json` once, optionally
respawns the implementer once, then returns.

### State source

`.research/management/assignments.json` — find the row whose `pr_number`
equals the input (fall back to matching `branch`). Read:

- `task_id`, `specialist`, `branch`, `pr_number`
- `reviewer_summary` (string, parsed for `verdict=...` and `note=...`)
- `fix_rounds` (int; default 0 if absent — backfill on first read)
- the task's original `action` text from `.research/management/action-list.json`
  matched by `task_id` (the dispatcher's Phase 4 implementer prompt uses
  this; we reuse it for re-spawn).

### Decision table

| Reviewer state                                  | Action                                                                 | Return line                                                |
| ---                                             | ---                                                                    | ---                                                        |
| `reviewer_summary` is `null`                    | No-op. Wait for next reviewer pass.                                    | `followup=skip pr=<n> note=no-verdict-yet`                 |
| `verdict=approve`                               | No-op (the next Phase 5.5 picks it up; `ppt-pr-merge` handles drafts). | `followup=skip pr=<n> note=approved-call-pr-merge`         |
| `verdict=changes` AND `fix_rounds >= 3`         | Mark row `status=failed`, `reason=fix-rounds-exhausted`.               | `followup=failed pr=<n> note=fix-rounds-exhausted`         |
| `verdict=changes` AND `fix_rounds < 3` AND row already `status=in-progress` | No-op — guard against double-spawn (idempotency).         | `followup=skip pr=<n> note=in-progress-already`            |
| `verdict=changes` AND `fix_rounds < 3`          | Increment `fix_rounds`, set `status=in-progress`, spawn original specialist via the same `Task` channel `ppt-research-dispatcher` uses in Phase 4. After the spawn returns, set `status=review`, clear `reviewer_summary` so the next reviewer pass picks it up fresh. | `followup=respawned pr=<n> specialist=<sp> round=<k>` |
| `verdict=block` or `verdict=reject`             | Mark row `status=failed`, `reason=reviewer-rejected`.                  | `followup=failed pr=<n> note=reviewer-rejected`            |

### Respawn brief

The respawn does NOT re-do the whole task. Pass the spawned implementer:

- `task_id`, `branch`, `pr_number` — unchanged from the original row.
- `specialist` — same as the original row (deterministic; do not re-pick).
- `brief` — the **reviewer's blocking findings** (`reviewer_summary.note`),
  NOT the original `action` text. The implementer pushes commits onto the
  existing PR branch and does **not** open a new PR.
- `base_branch` — `dev`.

Spawn template (mirrors the dispatcher's Phase 4 prompt, with `brief`
swapped for the reviewer note):

> You are a `<specialist>` implementer addressing reviewer fix-round
> `<k>/3` on PR #`<n>` (branch `<branch>`). Do NOT open a new PR.
> Check out `<branch>` and push commits on top.
>
> Reviewer's blocking findings (must be addressed):
>
>     <reviewer_summary.note>
>
> Original task context for reference only: `<task_id>: <action>`.
>
> Follow `.claude/skills/ppt-implement/agents/<specialist>.md`. Verify per
> the specialist's verify command before pushing. Return EXACTLY:
> `pr=<n> status=<done|blocked> specialist=<sp> note=<short>`.

### Idempotency

`fix_rounds` + `status=in-progress` together prevent double-spawn:

- Running the skill once on a `verdict=changes` row: bumps `fix_rounds`
  to N+1, flips `status` to `in-progress`, spawns.
- Running it again before the spawn returns: row is now `in-progress`,
  the table's penultimate row matches → `followup=skip`.
- Running it after the spawn returned and bumped `status=review` but
  before the next reviewer pass: `reviewer_summary` is cleared
  (`null`) → first row matches → `followup=skip note=no-verdict-yet`.

### Round cap

Hard cap of **3 fix rounds**. After 3 unsuccessful respawns the row is
terminal `failed`. This is the same shape as `Phase 5.6`'s
`rebase_attempts < 3` guard.

### Return contract (one line — dispatcher parses this)

```
followup=<respawned|skip|failed> pr=<n> [specialist=<sp>] [round=<k>] note=<short>
```

The dispatcher reads this verbatim into a Phase 5.7 log row.

### Reference script

`scripts/dispatcher-followup.sh` implements the decision table above. It is
the canonical source; the SKILL.md describes the semantics for agents that
need to reason about edge cases, but the script is what runs.

```bash
bash .claude/skills/ppt-pr-followup/scripts/dispatcher-followup.sh \
  --pr <n> [--dry-run]
```

The script:
- reads `.research/management/assignments.json`,
- decides the action per the table above,
- writes the mutation back (atomic temp-file + rename),
- prints the spawn brief to stdout when action is `respawned` (the
  dispatcher then feeds that brief into the `Task` tool — the script
  itself does not call any Claude tools),
- exits non-zero on `failed` so the dispatcher logs it.

## What it does (manual mode)

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

## Manual-mode inputs (legacy detail; see top-level Inputs table for canonical names)

- `PR` — PR number (optional). If omitted, derive from the current branch via
  `gh pr view --json number,headRefName`.
- `MAX_ITERS` — safety cap (default 10). Stops runaway loops.
- `SCOPE` — `ci-only` | `reviews-only` | `both` (default `both`).

## Iteration recipe (manual mode)

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
