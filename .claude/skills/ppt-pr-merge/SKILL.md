---
name: ppt-pr-merge
description: Land a green, approved PR. Verify preconditions (CI green, approved, no unresolved threads, not draft), auto-resolve mechanical merge conflicts against the base branch (sqlx offline data, Cargo.lock, generated openapi/api-client, lockfiles), then `gh pr merge --squash --auto`. Stops and surfaces if real code conflicts or stale CI.
when_to_use: A PR is in `review` state, has reviewer approval, CI is green, and you're ready to merge. Typically called by the ppt-research-dispatcher Phase 5.5 after the per-PR reviewer in Phase 5 returns `verdict=approve`, or hand-invoked as "merge PR #N".
mode: both
capabilities: [C6]
tags: [workflow, merge, ci]
---

# PPT PR Merge

The terminal step of the implementer pipeline. Sister skill of `ppt-pr-create`
(opens) and `ppt-pr-followup` (drives review/CI loop). This skill closes out
the loop by getting a green, approved PR actually merged.

## When to invoke

- A PR has `reviewDecision == APPROVED`, all checks green, no unresolved threads.
- The dispatcher's Phase 5 reviewer agent just returned `verdict=approve`.
- You manually want to "land PR #N".

**Do NOT invoke for:**
- PRs with CI failures (run `ppt-pr-followup` first)
- PRs with unresolved review threads (run `ppt-pr-followup` first)
- PRs blocked by branch protection rules a human must override

**Drafts:** this skill **auto-promotes** a draft PR to ready (`gh pr ready <pr>`)
**iff** the PR has reviewer approval (either `reviewDecision == APPROVED` on
GitHub, or `reviewer_summary.verdict == "approve"` in
`.research/management/assignments.json`) AND all required CI checks are green.
Otherwise the skill refuses to touch a draft and points the caller at
`ppt-pr-followup` (dispatcher mode) for the next step.

## Inputs

| Input          | Default                            | Notes                                              |
| ---            | ---                                | ---                                                |
| `pr_number`    | (required)                         | The PR to merge                                    |
| `repo`         | `martin-janci/property-management` | Full GH slug                                       |
| `base`         | `dev`                              | Used for conflict-resolution merge                 |
| `strategy`     | `squash`                           | `squash` / `rebase` / `merge`                      |
| `delete_branch`| `true`                             | Pass `--delete-branch` to `gh pr merge`            |
| `dry_run`      | `false`                            | Run preconditions + conflict resolve, skip the merge call |
| `mode`         | `merge`                            | `merge` (default — full flow) or `rebase-only` (run Step 2 conflict-resolver + push, then return without merging — used by dispatcher Phase 5.6 to unstick stale-approved PRs) |

## Step 1 — Preconditions (HARD GATES — abort if any fail)

```bash
PR=<pr_number>
REPO=<repo>

# Read full PR state
gh pr view "$PR" --repo "$REPO" --json \
  number,state,isDraft,mergeable,reviewDecision,statusCheckRollup,headRefName,headRefOid,baseRefName
```

Abort with `merged=false note=<reason>` if ANY of:

| Check | Abort if |
|---|---|
| `state` | != `"OPEN"` (already merged or closed) |
| `reviewDecision` | != `"APPROVED"` (see "approval source" below; allow `null` ONLY when repo has no required reviews) |
| `statusCheckRollup[].conclusion` | any of `"FAILURE"`, `"CANCELLED"`, `"TIMED_OUT"`, `"ACTION_REQUIRED"` |
| `statusCheckRollup[].status` | any `"IN_PROGRESS"`/`"QUEUED"` → return `merged=false note=ci-pending` (caller can retry later) |

### Approval source (GH `reviewDecision` OR dispatcher verdict)

`reviewDecision` is the primary signal. If it is missing/`null` (the dispatcher's
reviewer agents review via `gh pr review --approve` so this is usually populated,
but some repos / draft PRs return `null`), fall back to the dispatcher's
`reviewer_summary` in `.research/management/assignments.json`:

```bash
# Read dispatcher verdict for the row whose pr_number == <PR>
VERDICT=$(jq -r --argjson n "$PR" '
  .assignments[]
  | select(.pr_number == $n)
  | .reviewer_summary
  | tostring
  | capture("^verdict=(?<v>approve|changes|block|reject)").v // empty
' .research/management/assignments.json 2>/dev/null)
```

Treat `VERDICT == "approve"` as approval-equivalent. Any other value (or
missing assignments.json row) is **not** approval.

### Draft handling — auto-promote on approve, otherwise refuse

If `isDraft == true`:

1. Compute the approval state using both sources above:
   `APPROVED = (reviewDecision == "APPROVED") OR (VERDICT == "approve")`.
2. If `APPROVED` AND CI is green per the table above: promote the PR.
   ```bash
   gh pr ready "$PR" --repo "$REPO"
   # Re-read PR state once; isDraft should now be false.
   ```
   Then continue with the rest of Step 1 (unresolved-threads check) and the
   normal merge flow.
3. If NOT approved (no verdict, or verdict ∈ `{changes, block, reject}`):
   abort with
   `merged=false note=draft-unapproved (call ppt-pr-followup to drive the review loop)`.
4. If CI is not green: abort via the standard CI path
   (`merged=false note=ci-pending|ci-failed`). **Never promote a draft whose
   CI isn't green.**

Also check unresolved review threads via GraphQL (the REST `reviewDecision` doesn't catch comments-only threads):

```bash
gh api graphql -f query='
  query($owner:String!, $repo:String!, $pr:Int!){
    repository(owner:$owner, name:$repo){
      pullRequest(number:$pr){
        reviewThreads(first:50){nodes{id isResolved}}
      }
    }
  }' -f owner=martin-janci -f repo=property-management -F pr=$PR \
  | jq '[.data.repository.pullRequest.reviewThreads.nodes[] | select(.isResolved==false)] | length'
```

Abort with `merged=false note=unresolved-threads=<count>` if count > 0.

## Step 2 — Mergeable check + conflict resolution

If `mergeable == "MERGEABLE"`: jump to Step 3.

If `mergeable == "CONFLICTING"`: try the auto-resolver. Clone the PR branch to a temp worktree, merge base, and handle known mechanical conflicts.

```bash
HEAD_REF=$(gh pr view "$PR" --repo "$REPO" --json headRefName -q .headRefName)
TMP=$(mktemp -d)/ppt-pr-merge
git worktree add "$TMP" "$HEAD_REF"
cd "$TMP"
git fetch origin "$base"
git merge --no-ff "origin/$base" -m "Merge branch '$base' into $HEAD_REF (auto: ppt-pr-merge)"
```

If merge fails, inspect `git diff --name-only --diff-filter=U`. **Auto-resolve only these patterns:**

| Conflict in path | Resolution strategy |
|---|---|
| `backend/.sqlx/*.json` | Take base, then regenerate: `cd backend && cargo sqlx prepare --workspace` |
| `backend/Cargo.lock` | Take base, then regenerate: `cd backend && cargo update -p <root-crate> --precise <head-version>` (or simpler: `cargo build` and let it relock) |
| `docs/api/openapi.yaml` | Take base, then regenerate: `cd docs/api/typespec && npx tsp compile .` |
| `frontend/packages/api-client/src/**/*.ts` (generated) | Take base, then regenerate: `pnpm -F @ppt/api-client generate && pnpm -F @ppt/api-client build` |
| `frontend/pnpm-lock.yaml` | Take base, then regenerate: `pnpm install --no-frozen-lockfile` |
| `VERSION` | Always take base (CI manages this) |
| Any `*.lock` / `*.lockfile` we know how to regenerate | Take base, regenerate |
| `mobile-native/gradle.properties` (version line only) | Take base |

**Anything else (real code conflicts)** → abort:

```bash
git merge --abort
git worktree remove --force "$TMP"
```

Return `merged=false note=conflict-in:<comma-separated-paths>`. Do NOT push partial resolutions.

If auto-resolve succeeded:
1. Verify the resolution: per-stack quick check (`cargo check -p <crate>`, `pnpm -F <pkg> typecheck`, etc. — pick smallest applicable from `ppt-tests`).
2. Stage + commit: `git add -A && git commit --no-edit` (uses the merge message).
3. Push: `git push origin "$HEAD_REF"`.
4. `git worktree remove --force "$TMP"`.
5. Re-check `mergeable` (GH needs ~10s to recompute):
   ```bash
   for i in 1 2 3 4 5; do
     sleep 5
     M=$(gh pr view "$PR" --repo "$REPO" --json mergeable -q .mergeable)
     [ "$M" = "MERGEABLE" ] && break
   done
   ```
   If still not `MERGEABLE` after 25s → return `merged=false note=mergeable-recompute-timeout`.

### `mode=rebase-only` short-circuit

If invoked with `mode=rebase-only`, complete Step 2 (auto-resolve and push)
then **return without invoking `gh pr merge`**. Return contract is the same
shape, but the value semantics shift:

```
merged=<true|false> pr=<n> note=rebase-only:<rebased|conflict-in:<paths>|already-clean>
```

- `merged=true note=rebase-only:rebased` — base merged, mechanical conflicts auto-resolved, push succeeded.
- `merged=true note=rebase-only:already-clean` — branch was already MERGEABLE; no push needed.
- `merged=false note=rebase-only:conflict-in:<paths>` — real code conflict; manual rebase required.

The dispatcher's Phase 5.6 uses this mode. Phase 5.5 (next cycle) will pick
the now-clean PR up and run the full merge flow.

## Step 3 — Merge

```bash
gh pr merge "$PR" --repo "$REPO" --squash --auto --delete-branch \
  --subject "$(gh pr view "$PR" --repo "$REPO" --json title -q .title)"
```

`--auto` queues the merge if branch protection requires up-to-date branch; GH
will merge as soon as conditions are met. Combined with our up-front
preconditions, this usually merges within seconds.

If `--auto` is rejected (repo doesn't allow auto-merge), retry without it:

```bash
gh pr merge "$PR" --repo "$REPO" --squash --delete-branch \
  --subject "$(gh pr view "$PR" --repo "$REPO" --json title -q .title)"
```

## Step 4 — Confirm + return

Wait briefly and confirm the merged state:

```bash
sleep 5
STATE=$(gh pr view "$PR" --repo "$REPO" --json state -q .state)
MERGED_AT=$(gh pr view "$PR" --repo "$REPO" --json mergedAt -q .mergedAt)
```

If `STATE == "MERGED"`: return `merged=true pr=<n> at=<mergedAt> note=squash-merged`.
If `STATE == "OPEN"` but `--auto` accepted: return `merged=queued pr=<n> note=auto-merge-queued (will land when conditions met)`.
Else: return `merged=false pr=<n> note=unexpected-state:<STATE>`.

## Return contract (ONE LINE — dispatcher parses this)

```
merged=<true|false|queued> pr=<n> note=<short text>
```

The dispatcher does NOT manually update assignments.json on merge — Phase 2
of the next cron cycle will catch the GH MERGED state and flip the row to
`merged` (terminal). Legacy rows with `done` are treated as equivalent. This
skill never touches assignments.json directly.

## Hard rules

- Drafts are auto-promoted on approve; otherwise refuse. Never merge a draft
  that has not been promoted to ready via the auto-promote path in Step 1.
- Never merge a PR with failing or in-progress CI.
- Never merge a PR with unresolved review threads.
- Auto-resolve ONLY the mechanical patterns listed in Step 2; real code conflicts always abort.
- Always verify the auto-resolved branch with a quick per-stack check before pushing.
- Never bypass branch protection (no `--admin` flag from this skill — humans only).
- Never modify `assignments.json` here; that's the dispatcher's job.

## Install (local user)

```bash
bash .claude/skills/ppt-pr-merge/install.sh
```

The cloud routine reads this skill directly from the repo checkout.
