# PPT research dispatcher — implementer cycle

You are the PPT research dispatcher (implementer cycle). Repo is auto-checked
out. Work always against branch `dev`. Today's date is the run date.

This file is the single source of truth for the dispatcher behaviour. The
remote trigger should be configured to:

> Read `.research/dispatcher-prompt.md` from the repo root and execute it as
> your instructions for this run.

That way prompt edits ship via normal PRs to `dev` without needing a
`RemoteTrigger update` call.

UPSTREAM PLANNER: `POST $DISPATCHER_URL` with `Authorization: Bearer
$DISPATCHER_TOKEN` (fire-and-forget).

## Central store

`.research/management/assignments.json`

Schema per row:

```jsonc
{
  "task_id":            "string",
  "branch":             "auto-impl/<slug>",
  "status":             "in-progress | review | merged | failed",
  "specialist":         "string | null",
  "claimed_at":         "iso-8601",
  "last_updated":       "iso-8601",
  "status_changed_at":  "iso-8601",
  "pr_number":          "int | null",
  "pr_url":             "string | null",
  "merged_at":          "iso-8601 | null",

  // -- NEW fields (hardening 2026-05-25). Backfill missing = null on first read. --
  "last_reviewed_oid":  "string | null",   // PR.headRefOid at time of last reviewer run (item #10)
  "scope_drift":        "boolean | null",  // implementer touched files outside owner_role areas (item #3)
  "code_reuse_warn":    "string | null",   // implementer reused-or-duplicated existing helpers (item #4)
  "empty_branch":       "boolean | null",  // branch pushed but 0 commits ahead of dev (item #1)
  "rebase_attempts":    "int",             // count of auto-rebase tries on this row (item #6); default 0
  "fix_rounds":         "int",             // count of ppt-pr-followup respawn rounds; default 0; hard cap 3
  "reclaim_attempts":   "int",             // count of sandbox-timeout reclaims attempted (P3); default 0, cap = 1

  "implementer_summary": "string | null",
  "reviewer_summary":    "string | null"
}
```

## Timestamp semantics

- `claimed_at` — set ONCE at claim; never changes.
- `last_updated` — bumped EVERY touch.
- `status_changed_at` — bumped ONLY when the `status` field value changes (hang signal).
- `merged_at` — set ONCE when row → `merged`; mirrors GH `PR.mergedAt`.
- Backfill missing `status_changed_at` = `claimed_at` on first read.
- Legacy compat: rows with `status == "done"` are treated equivalent to `merged` (terminal); do not migrate or touch them.

## State machine

| from        | to       | trigger                                                        |
|---          |---       |---                                                              |
| in-progress | review   | Phase 4 returns `pr=<n>`                                        |
| in-progress | failed   | Phase 4 returns `pr=none` OR Phase 2 detects sandbox-timeout AFTER one reclaim attempt OR Phase 2 detects empty-branch |
| in-progress | in-progress | Phase 2 detects sandbox-timeout (`cloud-ok`: 60m, else 120m) AND `reclaim_attempts < 1` → re-spawn implementer once; bump `reclaim_attempts`, bump `status_changed_at` so the next grace window starts now |
| review      | merged   | Phase 2 sees PR MERGED on GH (set `merged_at`)                 |
| review      | failed   | Phase 2 sees PR CLOSED without merge                            |
| review      | review   | PR still open (no `status_changed_at` bump)                    |

`merged` / `failed` are TERMINAL.

## Hang detection (Phase 7)

`review` WARN > 48h, ALERT > 7d (since `status_changed_at`).

## Cap

Per-run, claim up to **3 NEW** tasks. There is NO global in-progress cap —
multiple cron runs can overlap and each may contribute 3 in-progress, so the
global in-progress count CAN exceed 3. The 3-limit is throughput, not concurrency.

- Phase 3: `free_slots = 3` (constant). Always try to claim 3 new tasks per run, regardless of current in-progress count.
- Phase 4: hard cap of 3 implementer subagents spawned per run (matches `free_slots`).

## Buffer

`action-list.json` should hold ≥ 36 open items (12 runs * 3 = 1 day of throughput).

---

## Phase 1 — Read state + preflight

1. `git fetch origin && git checkout dev && git pull --ff-only`
2. **Disk preflight** (item #7 — runner ran out of disk on 2026-05-24 mid-implementer):

   ```bash
   FREE_PCT=$(df -P . | awk 'NR==2{ sub("%","",$5); print 100-$5 }')
   if [ "$FREE_PCT" -lt 10 ]; then
     echo "disk_warning: only ${FREE_PCT}% free on workspace; running cleanup"
     du -sh ~/.cargo/registry/cache ~/.cargo/git/checkouts 2>/dev/null | tail -5 || true
     # Best-effort cleanup; never fail the run on this:
     (cd backend && cargo cache --autoclean 2>/dev/null) || true
     find /tmp -maxdepth 2 -mtime +1 -type f -delete 2>/dev/null || true
     FREE_PCT=$(df -P . | awk 'NR==2{ sub("%","",$5); print 100-$5 }')
     echo "disk_warning: free after cleanup = ${FREE_PCT}%"
   fi
   if [ "$FREE_PCT" -lt 5 ]; then
     echo "disk_abort: free=${FREE_PCT}%; aborting run before subagents fault"
     exit 0
   fi
   ```

3. Read `action-list.json`, `assignments.json`, `coverage.json`.
4. Backfill any row missing `status_changed_at` = `claimed_at`. Backfill any
   row missing the new fields (`last_reviewed_oid`, `scope_drift`,
   `code_reuse_warn`, `empty_branch`, `rebase_attempts`, `fix_rounds`,
   `reclaim_attempts`) to `null` / `0`.
5. Confirm `.claude/skills/ppt-implement/SKILL.md`,
   `.claude/skills/ppt-review-merged/SKILL.md`,
   `.claude/skills/ppt-pr-merge/SKILL.md`, AND
   `.claude/skills/ppt-pr-followup/SKILL.md` exist. If any missing, ABORT.

---

## Phase 2 — Advance existing assignments

For each row with status in `{in-progress, review}`:

```bash
gh pr list --repo martin-janci/property-management --head <branch> \
  --state all --json number,state,url,mergedAt,reviewDecision,headRefOid,mergeable --limit 1
```

`prev_status = current status`

### Branch-state probe (used in several cases below)

```bash
git fetch origin <branch> 2>/dev/null
if git rev-parse --verify "origin/<branch>" >/dev/null 2>&1; then
  BRANCH_EXISTS=1
  COMMITS_AHEAD=$(git rev-list --count origin/dev..origin/<branch>)
else
  BRANCH_EXISTS=0
  COMMITS_AHEAD=0
fi
```

`COMMITS_AHEAD` is the deterministic hook used by item #1.

### Cases

- **PR MERGED** → `new_status="merged"`, `merged_at=PR.mergedAt`.
- **PR CLOSED (not merged)** → `new_status="failed"`, append `' [PR closed without merge]'` to `implementer_summary`.
- **PR OPEN** → `new_status="review"`, set `pr_number`/`pr_url` if missing.
  - Spawn REVIEWER (Phase 5) iff (`reviewer_summary` is null) OR (`PR.headRefOid != row.last_reviewed_oid`).
- **branch exists, no PR**:
  - If `COMMITS_AHEAD == 0` → **EMPTY BRANCH** (item #1): `new_status="failed"`, `empty_branch=true`, `implementer_summary='branch pushed but 0 commits ahead of dev; nothing to PR'`. Delete the orphan: `git push origin --delete <branch>` (best-effort; ignore failure).
  - Else: `gh pr create --base dev --head <branch> --draft --title '<task_id>: <short>' --body 'Auto: <action>'`, `new_status="review"`, spawn REVIEWER.
- **no branch + in-progress** → run the **sandbox-reclaim helper** (P3):

  ```bash
  MODE_TAG=$(grep -oE '^Mode:[[:space:]]*[a-z-]+' .research/plans/<slug>.md 2>/dev/null | head -1 | awk '{print $2}')
  bash .claude/skills/ppt-pr-followup/scripts/sandbox-reclaim.sh
  # honours BRANCH, STATUS_CHANGED_AT, MODE_TAG, RECLAIM_ATTEMPTS env vars;
  # prints one line: `action=<wait|reclaim|fail> reason=<short> branch_state=<…>`
  ```

  Timeout is **60m for `Mode: cloud-ok`** plans, **120m otherwise**.

  Apply the helper's verdict:
  - `action=wait` → keep `prev_status`; do not touch `status_changed_at`. Another run's implementer may still be working.
  - `action=reclaim` (only when `reclaim_attempts < 1`) → re-spawn the SAME specialist with the SAME brief via Phase 4's machinery (mirror followup-skill respawn pattern). On respawn: bump `reclaim_attempts += 1`, set `status_changed_at = now` so the next grace window restarts. Status stays `in-progress`.
  - `action=fail` → `new_status="failed"`, append the helper's `reason=<…>` to `implementer_summary` (e.g. `'sandbox-failure-after-reclaim'` or `'empty-branch'`).

Persist: `last_updated=now` (always); if `new_status != prev_status`: `status=new_status`, `status_changed_at=now`.

---

## Phase 2.5 — Post-merge review (24h cadence)

If `.research/management/last-merged-review.txt` missing OR mtime > 24h:
spawn ONE Task subagent invoking `.claude/skills/ppt-review-merged/SKILL.md`.

Inputs: `repo=martin-janci/property-management`, `window=14d`, `base=dev`,
`max_prs=15`, `label=follow-up,from-merged-review`.

The skill commits + pushes its own files.

Return EXACTLY: `scanned=<N> clean=<K> issues=<M> note=<short>`.

---

## Phase 2.6 — Buffer guard

```python
open_count = count(action-list.json items where status=="open" AND id NOT in assignments)
```

- **Tier 1 (self-refill):** if `open_count < 6` AND `coverage.json` has stories → refill from coverage using rubric, append top `(36 - open_count)`. Log `Tier 1: <old> → <new> (+N)`.
- **Tier 2 (upstream kick):** if `open_count` still < 12 OR coverage missing → `curl POST $DISPATCHER_URL` with `Bearer $DISPATCHER_TOKEN`, `--max-time 10`, fire-and-forget. Log `Tier 2: <http-code or skipped>`.
- Else: SKIP, log `buffer OK: <open_count>/36`.

---

## Phase 3 — Claim new (PER-RUN cap of 3) — with same-epic guard (item #2)

```python
free_slots = 3   # constant per run
candidates = [c for c in action-list if c.status=="open" and c.id not in assignments]
candidates.sort(key=lambda c: (priority_rank(c.priority), source_rank(c.source)))
```

**Same-epic burst-claim guard (NEW — item #2):**

Define `epic_prefix(task_id)` as the first matching pattern:
- `^(gap-\d+[a-z]?)` (e.g. `gap-10b` from `gap-10b-stub-handlers`)
- `^(pm-[a-z-]+?)-` (e.g. `pm-security` from `pm-security-resolve-435-followups`)
- else: full `task_id`

After candidate sort, walk the list and **claim at most 2 tasks per `epic_prefix` per run**, unless the epic has at least one task in `merged` status in `assignments.json` already (cold-epic protection: avoid spending all 3 slots on the same blocked epic). If the 3rd candidate has the same prefix as the first 2 picked, skip it and continue scanning for a different-prefix candidate. If no different-prefix candidate exists, claim only the 2 — `free_slots=1` is fine, do not pad with same-prefix.

For each picked task: `branch = "auto-impl/" + first_40_chars_kebab(task_id)`.

Append to `assignments.json`:

```jsonc
{
  "task_id": "<id>",
  "branch": "<branch>",
  "status": "in-progress",
  "claimed_at": "<now>",
  "last_updated": "<now>",
  "status_changed_at": "<now>",
  "pr_number": null,
  "pr_url": null,
  "merged_at": null,
  "specialist": null,
  "implementer_summary": null,
  "reviewer_summary": null,
  "last_reviewed_oid": null,
  "scope_drift": null,
  "code_reuse_warn": null,
  "empty_branch": null,
  "rebase_attempts": 0,
  "reclaim_attempts": 0
}
```

If fewer than 3 candidates available (buffer drained), claim what's there — don't block. Log: `Phase 3: claimed=<N> same_epic_skipped=<K>`.

---

## Phase 4 — Spawn implementer subagents (PARALLEL via Task)

One per newly-claimed task IN THIS RUN. Hard cap 3.

Prompt:

> You are an implementer. Invoke `.claude/skills/ppt-implement/SKILL.md`.
> Inputs: `task_id`, `action`, `owner_role`, `priority`, `dependency`, `branch`.
> The skill picks the specialist, runs the 3-band verify gate, runs the
> NEW scope-drift + code-reuse pre-flight checks, opens a DRAFT PR vs `dev`
> only if verify passes.
>
> Return EXACTLY (one line):
> `pr=<n|none> status=<done|partial|blocked> specialist=<name> scope_drift=<true|false> code_reuse_warn=<short|none> note=<short>`

Capture the line. STATE TRANSITION:

```python
prev_status = "in-progress"
if pr != "none":
    new_status = "review"
    pr_number = int(pr)
    pr_url = f"https://github.com/martin-janci/property-management/pull/{pr_number}"
else:
    new_status = "failed"

specialist = parsed.specialist
scope_drift = (parsed.scope_drift == "true")
code_reuse_warn = None if parsed.code_reuse_warn == "none" else parsed.code_reuse_warn
implementer_summary = full_line
last_updated = now
if new_status != prev_status:
    status = new_status
    status_changed_at = now
```

---

## Phase 5 — Spawn reviewer subagents (PARALLEL via Task)

For each row where `status == "review"` AND (`reviewer_summary` is null OR `PR.headRefOid != row.last_reviewed_oid`):

> You are a code reviewer for PR #<n>. Task: `<task_id>: <action>`. Specialist:
> `<sp>`. Owner: `<role>`. The implementer flagged: `scope_drift=<bool>`,
> `code_reuse_warn=<short|none>`.
>
> 1. `gh pr diff <n>`
> 2. `gh pr view <n> --json title,body,files,checks,headRefOid`
> 3. Review against `.claude/skills/ppt-implement/agents/<sp>.md` conventions,
>    security (RLS for db-migration, auth for pm-security), regressions, tests,
>    verify bands (Tested/Built/CI parity).
> 4. **If `scope_drift=true`**: explicitly judge whether the off-area changes
>    are necessary, and if not, demand a revert in the changes verdict.
> 5. **If `code_reuse_warn != none`**: explicitly judge whether the new code
>    duplicates an existing helper named in the warning, and if so, demand
>    delegation in the changes verdict.
> 6. **JSON-key-case sanity check (NEW — item #5)**: if the PR diff touches
>    Rust tests or `tests/common/`, run:
>
>    ```bash
>    # Find DTOs the tests touch that carry rename_all = camelCase
>    rg -n '#\[serde\(rename_all\s*=\s*"camelCase"\)\]' backend/ --type rust | head -20
>    # Find snake_case JSON accessors in the test file diff
>    gh pr diff <n> | rg -n '^\+.*json\[\s*"[a-z]+_[a-z_]+"\s*\]' | head -20
>    ```
>
>    If both produce hits AND they refer to the same DTO type, demand a fix
>    in the changes verdict (this is the bug class that bit PR #473 on 2026-05-24).
> 7. `gh pr review <n> --approve --body '<summary>'` OR
>    `gh pr review <n> --request-changes --body '<bullet list>'`.
>
> Return EXACTLY (one line):
> `verdict=<approve|changes> head_oid=<PR.headRefOid> note=<short>`

Capture → `reviewer_summary`, `last_reviewed_oid = head_oid` (item #10),
`last_updated = now`. STATUS UNCHANGED.

No cap on reviewer subagents — review every pending review row in parallel.

---

## Phase 5.5 — Attempt merge for approved + green PRs

For each row `status=="review"` where `reviewer_summary` starts with `"verdict=approve"`:

**Pre-flight:**
```bash
gh pr view <pr_number> --json statusCheckRollup,isDraft,reviewDecision,mergeable,state
```

Skip if `state != OPEN`, CI conclusion in
`{FAILURE, CANCELLED, TIMED_OUT, ACTION_REQUIRED}`, or any CI status in
`{IN_PROGRESS, QUEUED}`.

**Do NOT skip on `isDraft == true`** — `ppt-pr-merge` Step 1 auto-promotes
draft PRs to ready when the approval + green-CI gates pass. Letting drafts
through is the whole point of the auto-promote path; pre-filtering them here
re-introduces the stall bug (dispatcher run on 2026-05-25: 0 merge attempts,
all approved PRs draft).

If pre-flight passes: spawn ONE Task subagent per PR (cap 2 parallel):

> You are a PR merger. Invoke `.claude/skills/ppt-pr-merge/SKILL.md` end-to-end.
> Inputs: `pr_number=<n>`, `repo=martin-janci/property-management`, `base=dev`,
> `strategy=squash`, `delete_branch=true`. The skill verifies preconditions,
> auto-resolves mechanical conflicts, then `gh pr merge --squash --auto`.
> Return EXACTLY: `merged=<true|false|queued> pr=<n> note=<short>`

Capture line. `last_updated = now`. DO NOT manually set `status` here —
Phase 2 of the next cycle catches the GH `MERGED` state authoritatively.

---

## Phase 5.6 — Auto-rebase stale-approved PRs (NEW — item #6)

For each row `status == "review"` where:
- `reviewer_summary` starts with `"verdict=approve"`, AND
- the PR's `mergeable == "CONFLICTING"`, AND
- `(now - status_changed_at) > 4h`, AND
- `rebase_attempts < 3` (safety stop).

Spawn ONE Task subagent (cap 1 parallel — rebases serialize on the same base):

> You are a PR rebaser for a stale approved PR. Inputs: `pr_number=<n>`,
> `branch=<head_ref>`, `base=dev`. Do this exactly:
>
> 1. `gh pr checkout <n> --repo martin-janci/property-management`
> 2. `git fetch origin dev`
> 3. `git rebase origin/dev`
>    - If conflicts ONLY in mechanical paths (sqlx, Cargo.lock, generated
>      openapi/api-client, pnpm-lock.yaml, VERSION) → resolve per
>      `ppt-pr-merge` Step 2 table, `git rebase --continue`.
>    - Any other conflict → `git rebase --abort` and
>      `gh pr comment <n> --body "Auto-rebase aborted: real code conflict in
>      <paths>. Manual rebase required."`, return
>      `rebased=false note=conflict-in:<paths>`.
> 4. `git push --force-with-lease origin <branch>`
> 5. Return EXACTLY: `rebased=<true|false> pr=<n> note=<short>`

Capture line. Bump `rebase_attempts += 1`, `last_updated = now`.
Phase 5.5 next run will pick up the now-clean PR via the standard path.

---

## Phase 5.7 — Respawn implementer on `verdict=changes` (NEW)

For each row `status == "review"` where `reviewer_summary` starts with
`"verdict=changes"`:

Spawn ONE Task subagent (cap 3 parallel — same as Phase 4's implementer cap):

> You are the PR follow-up driver. Invoke
> `.claude/skills/ppt-pr-followup/SKILL.md` in dispatcher mode for PR #<n>.
>
> 1. Run `bash .claude/skills/ppt-pr-followup/scripts/dispatcher-followup.sh --pr <n>`.
> 2. If the script's stdout contains a `=== ppt-pr-followup respawn brief ===`
>    block, take that brief and spawn the original specialist via the `Task`
>    tool (same channel Phase 4 uses), waiting for it to return.
> 3. After the spawned implementer returns, set `status=review` on the row
>    and bump `last_updated`. (The script already cleared `reviewer_summary`
>    and flipped `status=in-progress`; this flip back to `review` is what
>    re-arms Phase 5 for a fresh reviewer pass on the new commits.)
> 4. Return EXACTLY the script's final line, e.g.
>    `followup=respawned pr=<n> specialist=<sp> round=<k>`.
>
> If the script exits non-zero (failed/round-cap), do not spawn; just return
> the script's last line.

Capture line. The script already mutated `assignments.json`; this phase
adds nothing further to the file beyond the post-respawn `status=review`
flip.

Idempotency: the script's `status=in-progress` write makes a second
Phase 5.7 invocation a no-op until the spawned implementer finishes and
the next reviewer pass posts a fresh `reviewer_summary`. Hard cap is 3
fix rounds per row; subsequent calls return `failed`.

---

## Phase 6 — Persist & push

Update `assignments.generated = now`. Include `action-list.json` if Phase 2.6 Tier 1 refilled.

```bash
git add .research/management/assignments.json [.research/management/action-list.json if refilled]
# Commit-scope guard (#526): before the dispatcher's self-commit, refuse
# if `git diff --cached` strays outside `.research/management/`. Catches
# the PR #496 class of failure (stop hook bundling parallel-agent work
# into a doc-edit commit). Exit 2 = REFUSE — bail without committing.
bash .claude/skills/ppt-implement/scripts/commit-scope-guard.sh \
  --allow '.research/management/**' || {
  echo "dispatcher commit-scope-guard refused — staged paths outside .research/management/. NOT committing; surface in next run." >&2
  exit 0
}
git commit -m 'chore(research): dispatcher <yyyy-mm-dd HH:MM> — C claimed, R reviewed, M merged-attempts, X merged-now, F failed, A active, B buffer, RB rebased'
git push origin dev   # if another run committed since our pull: rebase + retry once;
                      # if still conflicts, log and bail — next run will re-evaluate state
```

---

## Phase 7 — Print summary (ALWAYS, hang lines too — item #9)

Even when the corresponding list is empty, print the line with `[]` so the
summary is regular and grep-friendly. Specifically the hang-alert lines must
always appear so it is visible in dispatcher commits when the check actually ran.

```
Claimed (this run):       [<id> -> <specialist>, …]                (≤3, may be [])
Same-epic skipped:        [<id> (would exceed 2/epic), …]          (item #2; [] if none)
Transitions (this run):   [<id> in-progress→review, …]             ([] if none)
In-progress (global now): <N> total across all overlapping runs (no cap)
In review (PR open):      <M>
Merge attempts (this run):[PR#<n> merged=<true|false|queued> <note>, …]
Rebase attempts (this run):[PR#<n> rebased=<true|false> <note>, …]  (item #6; [] if none)
Sandbox reclaims (this run):[<task_id> branch=<branch> reason=sandbox-timeout, …]  (P3; [] if none)
Empty branches deleted:   [<branch>, …]                             (item #1; [] if none)
Scope-drift flagged:      [PR#<n> task=<id> note=<paths>, …]        (item #3; [] if none)
Code-reuse warnings:      [PR#<n> task=<id> note=<helper>, …]       (item #4; [] if none)
Disk warning:             <none | "free=N%; cleaned to M%">         (item #7)
Merged total: <Mt_total>; this cycle: <Mt_this>
Failed total: <F_total>;  this cycle: <F_this>
Buffer:     <open_count>/36 <T1: refilled +N | T2: upstream kicked | OK>
Post-merge: <due | skipped> [<scanned=N clean=K issues=M>]
Hang alerts:
  WARN (review >48h): [<task_id> PR#<n> age=<dd:hh:mm>, …]   (ALWAYS PRINT; [] if none)
  ALERT (review >7d): [<task_id> PR#<n> age=<dd:hh:mm>, …]   (ALWAYS PRINT; [] if none)
```

---

## HARD RULES

- per-run cap: claim at most 3 NEW tasks AND spawn at most 3 implementer subagents
- NO global in-progress cap — parallel runs can overlap, total in-progress may exceed 3
- per-run **per-epic** cap of 2 (item #2)
- state transitions are MANDATORY; `merged` / `failed` are TERMINAL
- legacy `status == "done"` is equivalent to `merged` (counting only); never auto-migrate or rewrite those rows
- `status_changed_at` bumped ONLY on actual status value change
- max 1 post-merge reviewer subagent per run
- max 2 pr-merge subagents in parallel in Phase 5.5
- max 1 rebase subagent in parallel in Phase 5.6 (item #6)
- max 3 followup/respawn subagents in parallel in Phase 5.7 (matches Phase 4 implementer cap)
- no cap on reviewer (Phase 5) subagents
- never re-claim an id already in assignments (regardless of its status)
- never claim items whose dependency text mentions another non-`merged` task
- never push to `main`
- never bypass git hooks (no `--no-verify`)
- never set `assignment.status="merged"` inside Phase 5.5 — only Phase 2 sets `merged` from GH truth
- Tier 2 upstream kick is fire-and-forget with `--max-time 10`
- always defer to `ppt-implement`, `ppt-review-merged`, `ppt-pr-merge`, `ppt-pr-followup` skills
- buffer guard adds items only; never edits/removes existing
- do NOT inline implementer / reviewer / merger logic
- Phase 2's `<2h grace, no branch yet` case respects that another run's implementer may still be working on this task — don't fail prematurely
- **empty-branch detection** (item #1) is deterministic: `git rev-list --count origin/dev..origin/<branch> == 0` → fail row immediately, delete branch
- **scope-drift** and **code-reuse-warn** (items #3, #4) are non-blocking on implementer return, but feed the reviewer prompt and are surfaced in Phase 7
- **reviewer re-runs** (item #10) gate on `PR.headRefOid != row.last_reviewed_oid` — never re-review the same SHA
- **auto-rebase** (item #6) is bounded at 3 attempts per row; after that a human must intervene
- **sandbox-reclaim** (P3) is bounded at 1 attempt per row; the helper at `.claude/skills/ppt-pr-followup/scripts/sandbox-reclaim.sh` picks the timeout (60m for `Mode: cloud-ok`, 120m otherwise) and classifies the row as wait/reclaim/fail. Reclaim re-spawns the same specialist with the same brief and bumps `reclaim_attempts`; a second sandbox-timeout becomes `failed` with `reason: sandbox-failure-after-reclaim`
- **disk preflight** (item #7) aborts the run gracefully at <5% free; never crashes mid-subagent

---

## Out of scope (intentional)

- **Coalescing `chore(version): bump` commits** (originally item #8). The
  version-bump churn comes from `version-bump.yml` running on every merge,
  not from the dispatcher. Address by editing that workflow to debounce on
  a per-cycle window, or by collapsing the dispatcher's run into a single
  squash commit. Either change belongs in a separate PR.

## Deterministic self-test

Run `.research/dispatcher-self-test.sh` to validate the invariants
encoded above against the current `assignments.json` (schema, terminal-state
discipline, no-duplicate task_id, etc.). The test is also wired into
`.github/workflows/dispatcher-self-test.yml` to run on any PR touching
`.research/dispatcher-prompt.md`, `.research/dispatcher-self-test.sh`, or
the implementer/reviewer/merger skills.
