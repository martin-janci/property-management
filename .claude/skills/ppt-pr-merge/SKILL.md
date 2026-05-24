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
- PRs still in `draft` state
- PRs with CI failures (run `ppt-pr-followup` first)
- PRs with unresolved review threads (run `ppt-pr-followup` first)
- PRs blocked by branch protection rules a human must override

## Inputs

| Input          | Default                            | Notes                                              |
| ---            | ---                                | ---                                                |
| `pr_number`    | (required)                         | The PR to merge                                    |
| `repo`         | `martin-janci/property-management` | Full GH slug                                       |
| `base`         | `dev`                              | Used for conflict-resolution merge                 |
| `strategy`     | `squash`                           | `squash` / `rebase` / `merge`                      |
| `delete_branch`| `true`                             | Pass `--delete-branch` to `gh pr merge`            |
| `dry_run`      | `false`                            | Run preconditions + conflict resolve, skip the merge call |

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
| `isDraft` | `true` |
| `reviewDecision` | != `"APPROVED"` (allow `null` ONLY when repo has no required reviews) |
| `statusCheckRollup[].conclusion` | any of `"FAILURE"`, `"CANCELLED"`, `"TIMED_OUT"`, `"ACTION_REQUIRED"` |
| `statusCheckRollup[].status` | any `"IN_PROGRESS"`/`"QUEUED"` → return `merged=false note=ci-pending` (caller can retry later) |

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

- Never merge a draft PR.
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
