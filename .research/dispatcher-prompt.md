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
  "last_reviewed_oid":  "string | null",   // PR.headRefOid at time of last reviewer run (item #10) — MANDATORY when reviewer_summary != null (gap 1)
  "scope_drift":        "boolean | null",  // implementer touched files outside owner_role areas (item #3)
  "code_reuse_warn":    "string | null",   // implementer reused-or-duplicated existing helpers (item #4)
  "empty_branch":       "boolean | null",  // branch pushed but 0 commits ahead of dev (item #1)
  "rebase_attempts":    "int",             // count of auto-rebase tries on this row (item #6); default 0
  "fix_rounds":         "int",             // count of ppt-pr-followup respawn rounds; default 0; hard cap 3
  "reclaim_attempts":   "int",             // count of sandbox-timeout reclaims attempted (P3); default 0, cap = 1
  "merge_attempted_at": "iso-8601 | null", // last time Phase 5.5 tried to merge this row (gap 4); used for CI-stuck back-off

  "implementer_summary": "string | null",
  "reviewer_summary":    "string | null"
}
```

## action-list.json schema (gap 3 — structured deps)

Each item in `.research/management/action-list.json` `.items[]` carries:

```jsonc
{
  "id":          "string",       // e.g. "gap-7a-2-folder-api"
  "action":      "string",       // free-text description
  "owner_role":  "string",       // e.g. "pm-backend"
  "priority":    "high | medium | low",
  "status":      "open | in-progress | done | dropped",
  "source":      "string",       // provenance
  "deadline":    "iso-8601 | null",

  // -- Dependency wiring --
  "dependency":  "string | null", // LEGACY free-text; informational only
  "depends_on":  ["task_id", …]   // gap 3 — structured; canonical for Phase 3
}
```

**`depends_on` is the canonical, machine-checked dependency field.**
The free-text `dependency` is retained for human readability but the
dispatcher MUST NOT regex-parse it for claim decisions. An empty array
(`"depends_on": []`) means no dependencies.

Refill/refresh flows (`coverage.json` rubric, Tier-1 buffer, manual
edits) MUST populate `depends_on` directly; the dispatcher does not
re-parse `dependency` text on each run.

Migration (one-time, gap 3): for any row missing `depends_on`,
best-effort-parse the legacy `dependency` field by splitting on
`, ; and / AND` and matching kebab-case task-id-shaped tokens
(`gap-…`, `pm-…`, `epic-…`). Unparseable values (owner-role names,
epic descriptions like "Epic 2B WebSocket infrastructure") become
`[]` and the free-text is left in `dependency` for human follow-up.

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
| in-progress | review   | Phase 2 of a SUBSEQUENT run sees a PR exists on `<branch>` (gap 5 — async-spawn model). Fast-path: Phase 4 of THIS run captures `pr=<n>` from a sync return; either path is valid. |
| in-progress | failed   | Phase 2 detects sandbox-timeout AFTER one reclaim attempt OR Phase 2 detects empty-branch OR (fast-path) Phase 4 returns `pr=none` on a synchronous return |
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

## Subagent workspace isolation (NEW — issue #7: branch displacement)

The dispatcher runs from a single working tree on `dev`. Any subagent that
does `git checkout <other-branch>` in that same working tree silently
displaces the dispatcher off `dev` — Phase 6's `git add .research/management/`
then stages files relative to the wrong branch, and Phase 1 of the next run
pulls into the wrong branch. This bit the 2026-05-27 runs (stash-pop
conflicts, assignments.json written against feature branches).

Fix: **every subagent that needs a different branch MUST do its work in a
dedicated `git worktree` under `/tmp/ppt-worktrees/<task_id>/`**, NEVER in
the dispatcher's working tree.

Standard preamble injected into every Phase 4 / 5.6 / 5.7 subagent brief:

```bash
WORKTREE_PATH="/tmp/ppt-worktrees/${TASK_ID:-${PR_NUMBER:-spawn-$$}}"
mkdir -p "$(dirname "$WORKTREE_PATH")"
# Fresh checkout off origin/dev; -B re-creates the branch if a previous
# attempt left it behind.
git worktree add -B "$BRANCH" "$WORKTREE_PATH" origin/dev 2>&1 \
  || git worktree add "$WORKTREE_PATH" "$BRANCH"
cd "$WORKTREE_PATH"
trap 'cd / && git worktree remove --force "$WORKTREE_PATH" 2>/dev/null' EXIT
```

The trap removes the worktree on exit — `.gitignore` already excludes
`.worktrees/` from the repo, and `/tmp/ppt-worktrees/` is outside the repo
entirely, so no in-tree pollution either way.

Phase 5.5 (merger) does NOT need worktree isolation — `gh pr merge` is a
GitHub API call, no local checkout.

Phase 2 (state reconciliation) is read-only against `gh` and read-only
against `git rev-list/fetch`; no checkout needed.

---

## Phase 1 — Read state + preflight

1. `git fetch origin && git checkout dev && git pull --ff-only`
2. **Recent-run skip-gate** (NEW — issue #1, prevents the `assignments.json` rebase-race
   observed on 2026-05-27 when two cron runs ~1h apart both wrote `assignments.generated`):

   ```bash
   # Read assignments.generated from the freshly-pulled file.
   GEN_AT=$(jq -r '.generated // empty' .research/management/assignments.json 2>/dev/null)
   if [ -n "$GEN_AT" ]; then
     GEN_EPOCH=$(date -u -d "$GEN_AT" +%s 2>/dev/null || echo 0)
     NOW_EPOCH=$(date -u +%s)
     AGE_MIN=$(( (NOW_EPOCH - GEN_EPOCH) / 60 ))
     if [ "$AGE_MIN" -lt 30 ] && [ "$GEN_EPOCH" -gt 0 ]; then
       echo "skip-gate: assignments.generated=$GEN_AT (age=${AGE_MIN}m < 30m); another run is in flight"
       echo "  → Phase 2 (GH reconciliation, idempotent) WILL run; Phases 3/4/5/5.5/5.6/5.7 SKIP."
       echo "  → Phase 6 will commit only if Phase 2 produced status transitions."
       export DISPATCHER_SKIP_MUTATING=1
     fi
   fi
   ```

   When `DISPATCHER_SKIP_MUTATING=1`:
   - Phase 2 (state reconciliation from GH) runs normally — it is idempotent and safe.
   - Phase 2.5 / 2.6 / 2.7 / 3 / 4 / 5 / 5.5 / 5.6 / 5.7 SKIP. Each phase header
     MUST check `$DISPATCHER_SKIP_MUTATING` before doing work.
   - Phase 6 still commits + pushes IFF Phase 2 produced at least one transition
     (otherwise nothing to write).
   - Phase 7 prints summary as usual with `skip_reason=recent-run` in the header line.

   This is a soft lock based on a repo-level timestamp, not a filesystem lock —
   it survives cross-host parallel runs. Two runs within the same 30-min window
   will see the same `assignments.generated` and both skip mutating phases on
   their second pass; one of them already did the work.

3. **Disk preflight** (item #7 — runner ran out of disk on 2026-05-24 mid-implementer):

   ```bash
   FREE_PCT=$(df -P . | awk 'NR==2{ sub("%","",$5); print 100-$5 }')
   if [ "$FREE_PCT" -lt 10 ]; then
     echo "disk_warning: only ${FREE_PCT}% free on workspace; running cleanup"
     du -sh ~/.cargo/registry/cache ~/.cargo/git/checkouts 2>/dev/null | tail -5 || true
     # Best-effort cleanup; never fail the run on this:
     (cd backend && cargo cache --autoclean 2>/dev/null) || true
     # Prune stale Cargo incremental artifacts. backend/target/debug/incremental/
     # accumulates per-branch fingerprints that thrash across subagent runs
     # (observed 2026-05-27: 15 GB incremental dir on dev). Drop entries idle
     # for >4h — current-run artifacts stay hot, only multi-branch debris goes.
     find backend/target/debug/incremental -maxdepth 1 -mindepth 1 \
          -mmin +240 -exec rm -rf {} + 2>/dev/null || true
     # Same treatment for sub-agent worktrees from prior runs whose trap missed.
     find /tmp/ppt-worktrees -maxdepth 1 -mindepth 1 -type d \
          -mmin +360 -exec rm -rf {} + 2>/dev/null || true
     find /tmp -maxdepth 2 -mtime +1 -type f -delete 2>/dev/null || true
     FREE_PCT=$(df -P . | awk 'NR==2{ sub("%","",$5); print 100-$5 }')
     echo "disk_warning: free after cleanup = ${FREE_PCT}%"
   fi
   if [ "$FREE_PCT" -lt 5 ]; then
     echo "disk_abort: free=${FREE_PCT}%; aborting run before subagents fault"
     exit 0
   fi
   ```

4. Read **active** state files only. Archive files are loaded on demand
   later (issue #9 — token spending):

   - `.research/management/assignments.json` — **active rows only**
     (`status` ∈ `{in-progress, review}`). Terminal rows (merged/failed/done)
     live in `assignments-archive.json` and are NOT read here. The archive is
     loaded **only** by Phase 7 for `Merged total` / `Failed total` counts,
     via a streaming `jq | length` (file not parsed into the run's narrative
     context).
   - `.research/management/action-list.json` — **non-terminal items only**
     (`status` ∈ `{open, in-progress}`). Done/dropped items live in
     `action-list-archive.json` and are NOT read by the dispatcher in steady
     state — they exist for human audit / post-merge analytics only.
   - **Do NOT read `coverage.json` here.** It is only consumed by Phase 2.6
     Tier 1 when `open_claimable_count < 18`. Most runs have a healthy buffer
     and skip the file entirely (saves ~10k tokens / run when buffer is OK).
     Phase 2.6 reads it lazily.

   Token budget impact (measured 2026-05-28 dev snapshot): active
   `assignments.json` shrinks from ~39k → ~4k tokens, `action-list.json`
   from ~20k → ~14k. Combined with lazy `coverage.json` this is ~40k
   tokens off the per-run baseline.

5. Backfill any row missing `status_changed_at` = `claimed_at`. Backfill any
   row missing the new fields (`last_reviewed_oid`, `scope_drift`,
   `code_reuse_warn`, `empty_branch`, `rebase_attempts`, `fix_rounds`,
   `reclaim_attempts`, `merge_attempted_at`) to `null` / `0`.

   **Backfill applies to active rows only.** The archive is frozen — rows
   moved there in past runs are never rewritten, even if their schema is
   pre-hardening. The self-test exempts archive rows from hardening-field
   checks for the same reason.

   **Gap-1 backfill (re-review forcing):** for any row with
   `reviewer_summary != null AND last_reviewed_oid == null`, LEAVE
   `reviewer_summary` intact (do not clear it; it's still useful narrative)
   but treat that row as oid-drifted on this cycle's Phase 2 / Phase 5 gate.
   Concretely, in Phase 2 the re-review predicate becomes:

   ```
   spawn_reviewer iff
     (reviewer_summary is null)
     OR (last_reviewed_oid is null)             # gap-1: force re-review
     OR (PR.headRefOid != last_reviewed_oid)
   ```

   Once Phase 5 runs and writes the new `last_reviewed_oid`, the forcing
   condition self-clears on subsequent cycles.
6. Confirm `.claude/skills/ppt-implement/SKILL.md`,
   `.claude/skills/ppt-review-merged/SKILL.md`,
   `.claude/skills/ppt-pr-merge/SKILL.md`, AND
   `.claude/skills/ppt-pr-followup/SKILL.md` exist. If any missing, ABORT.
7. **Ensure gating labels exist** (P6) — idempotent:

   ```bash
   bash .claude/skills/ppt-pr-followup/scripts/ensure-labels.sh \
     martin-janci/property-management
   ```

   This creates `needs-human-review` once and is a no-op thereafter.

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
  - Spawn REVIEWER (Phase 5) iff
    (`reviewer_summary` is null) OR
    (`last_reviewed_oid` is null — gap-1 force re-review) OR
    (`PR.headRefOid != row.last_reviewed_oid`).
- **branch exists, no PR**:
  - If `COMMITS_AHEAD == 0` → **EMPTY BRANCH** (item #1): `new_status="failed"`, `empty_branch=true`, `implementer_summary='branch pushed but 0 commits ahead of dev; nothing to PR'`. Delete the orphan: `git push origin --delete <branch>` (best-effort; ignore failure).
  - Else: `gh pr create --base dev --head <branch> --draft --title '<task_id>: <short>' --body 'Auto: <action>'`, `new_status="review"`, spawn REVIEWER.
- **no branch + in-progress** → run the **sandbox-reclaim helper** (P3):

  ```bash
  PLAN_FILE=".research/plans/${TASK_ID}.md"
  MODE_TAG=$(grep -oE '^Mode:[[:space:]]*[a-z-]+' "$PLAN_FILE" 2>/dev/null | head -1 | awk '{print $2}')
  bash .claude/skills/ppt-pr-followup/scripts/sandbox-reclaim.sh
  # honours BRANCH, STATUS_CHANGED_AT, MODE_TAG, RECLAIM_ATTEMPTS env vars;
  # prints one line: `action=<wait|reclaim|fail> reason=<short> branch_state=<…>`
  ```

  Timeout is **60m for `Mode: cloud-ok`** plans, **120m otherwise**.

  Apply the helper's verdict:
  - `action=wait` → keep `prev_status`; do not touch `status_changed_at`. Another run's implementer may still be working.
  - `action=reclaim` → re-spawn the SAME specialist with the SAME brief via Phase 4's machinery (mirror followup-skill respawn pattern). On respawn: bump `reclaim_attempts += 1`, set `status_changed_at = now` so the next grace window restarts. Status stays `in-progress`. (Reclaim cap is enforced by `sandbox-reclaim.sh` — that script returns `action=fail` when `RECLAIM_ATTEMPTS >= 1`.)
  - `action=fail` → `new_status="failed"`, append the helper's `reason=<…>` to `implementer_summary` (e.g. `'sandbox-failure-after-reclaim'` or `'empty-branch'`).

Persist: `last_updated=now` (always); if `new_status != prev_status`: `status=new_status`, `status_changed_at=now`.

---

## Phase 2.5 — Post-merge review (24h cadence)

SKIP if `$DISPATCHER_SKIP_MUTATING == 1` (recent-run gate from Phase 1 step 2).

If `.research/management/last-merged-review.txt` missing OR mtime > 24h:
spawn ONE Task subagent invoking `.claude/skills/ppt-review-merged/SKILL.md`.

Inputs: `repo=martin-janci/property-management`, `window=14d`, `base=dev`,
`max_prs=15`, `label=follow-up,from-merged-review`.

The skill commits + pushes its own files.

Return EXACTLY: `scanned=<N> clean=<K> issues=<M> note=<short>`.

---

## Phase 2.6 — Buffer guard

SKIP if `$DISPATCHER_SKIP_MUTATING == 1` (recent-run gate from Phase 1 step 2).

**Archive lookup pattern (issue #9 — token spending).** With terminal rows
split into `assignments-archive.json`, the dep_blocked check below MUST
consult BOTH files when resolving a `depends_on` entry. Compute a set of
terminal task_ids once via jq and reuse — this keeps the archive at the
filesystem level (small set output, archive never enters the LLM context):

```bash
# Cheap one-shot: build the set of terminal task_ids from active + archive.
TERMINAL_IDS_JSON=$(jq -r '.assignments[]
                          | select(.status=="merged" or .status=="done")
                          | .task_id' \
  .research/management/assignments.json \
  .research/management/assignments-archive.json \
  | sort -u | jq -R . | jq -s .)
```

Use `$TERMINAL_IDS_JSON` as the lookup for "dep is satisfied". The pre-split
predicate (which loaded all of `assignments.json` to scan for status) is
replaced by an `index($dep) != null` test against this small set.

```python
open_count = count(action-list.json items where status=="open" AND id NOT in active_assignments)

# gap 2 + issue #9: a dep is "satisfied" iff its task_id is in TERMINAL_IDS
# (merged or done, sourced from active + archive). An item is dep-blocked
# if ANY depends_on entry is NOT satisfied.
def is_dep_blocked(item, terminal_ids):
    deps = item.get("depends_on") or []
    if not deps:
        return False
    for dep_id in deps:
        if dep_id not in terminal_ids:
            return True
    return False

dep_blocked_count    = count(open items where is_dep_blocked(item, terminal_ids))
open_claimable_count = open_count - dep_blocked_count
```

- **Tier 1 (self-refill):** if `open_claimable_count < 18` (half of the 36 target) → **NOW read `coverage.json`** (was previously loaded in Phase 1; deferred to here in issue #9). If coverage has stories → refill using rubric, append top `(36 - open_claimable_count)`. Log `Tier 1: <old_claimable> → <new_claimable> (+N)`. When `open_claimable_count >= 18` the file is never opened, saving ~10k tokens / run.
- **Tier 2 (upstream kick):** if `open_claimable_count` still `< 12` OR coverage missing → `curl POST $DISPATCHER_URL` with `Bearer $DISPATCHER_TOKEN`, `--max-time 10`. **Capture the response code AND first 200 chars of body** (NEW — issue #5: HTTP 400 from the planner used to vanish into fire-and-forget; now we see it):

  ```bash
  T2_TMP=$(mktemp)
  T2_CODE=$(curl -sS -X POST "$DISPATCHER_URL" \
    -H "Authorization: Bearer $DISPATCHER_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"reason":"buffer-low","claimable":'"$open_claimable_count"'}' \
    -o "$T2_TMP" -w '%{http_code}' --max-time 10 2>/dev/null || echo "curl-error")
  T2_BODY=$(head -c 200 "$T2_TMP" 2>/dev/null | tr '\n' ' ' | sed 's/[[:space:]]\+/ /g')
  rm -f "$T2_TMP"
  echo "Tier 2: http=$T2_CODE body=\"${T2_BODY:-<empty>}\""
  ```

  Still semantically fire-and-forget (we don't retry on non-2xx), but the body
  surfaces in the dispatcher commit log so a stuck/broken planner endpoint is
  visible without grepping the trigger's run history.
- Else: SKIP, log `buffer OK: claimable=<open_claimable_count>/36 (open=<open_count>, dep_blocked=<dep_blocked_count>)`.

The Phase 6 commit message MUST surface both counts:

```
chore(research): dispatcher <date> — C claimed, R reviewed, M merge-attempts,
X merged-now, F failed, A active, B <claimable>/<open> dep-blocked=<n>, RB rebased
```

---

## Phase 2.7 — Failed-dependency cascade (NEW — issue #6)

SKIP if `$DISPATCHER_SKIP_MUTATING == 1` (recent-run gate from Phase 1 step 2).

Problem: an open action-list item whose `depends_on` points at a row that has
gone terminal-`failed` will never become claimable on its own. It sits in the
buffer forever, eating a `dep_blocked_count` slot and pushing Tier-1/Tier-2
into refill-loops for items that should be re-planned, not retried.

For each `action-list.json` item with `status=="open"` and non-empty
`depends_on`:

```python
for dep_id in item.depends_on:
    dep_row = assignments.find(task_id=dep_id)
    if dep_row is not None and dep_row.status == "failed":
        # Terminal dependency. Cascade.
        item.status = "dropped"
        item.action = f"[CASCADED-DROP: depends_on {dep_id} terminal-failed] " + item.action
        cascade_log.append((item.id, dep_id))
        break  # one failed dep is enough to drop
```

Cap: process at most 20 cascades per run (defensive — avoids a runaway sweep if
a single failed item has dozens of dependents). If more than 20 are eligible,
process the first 20 by sort order (priority desc, id asc) and surface the
remainder count in Phase 7.

This phase modifies `action-list.json` (`status=open → status=dropped`). When
any cascade happened, include `action-list.json` in Phase 6's commit.

Re-planning is upstream: the dropped items show up in Phase 7
(`Failed-dep cascades:`) and Tier 2 can be kicked manually
(`curl -X POST $DISPATCHER_URL`) once the operator has decided how to unblock
the failed parent (re-spec it, split it, or accept that the dependent work
is also dead). The dispatcher does NOT auto-respawn failed work.

Idempotency: items already `status=="dropped"` are skipped.

---

## Phase 3 — Claim new (PER-RUN cap of 3) — with same-epic guard (item #2)

SKIP if `$DISPATCHER_SKIP_MUTATING == 1` (recent-run gate from Phase 1 step 2).

```python
free_slots = 3   # constant per run

# gap 3 + issue #9: claimable iff every depends_on entry is in TERMINAL_IDS
# (the set built in Phase 2.6 from active + archive — reuse it here, do not
# rebuild). active_ids is the set of task_ids in the active assignments file.
def claimable(c, terminal_ids):
    for dep_id in (c.get("depends_on") or []):
        if dep_id not in terminal_ids:
            return False
    return True

candidates = [c for c in action-list
              if c.status == "open"
              and c.id not in active_ids
              and c.id not in terminal_ids       # don't re-claim something already shipped
              and claimable(c, terminal_ids)]
candidates.sort(key=lambda c: (priority_rank(c.priority), source_rank(c.source)))
```

The legacy `dependency` free-text field is NOT consulted by the claim
predicate. Only `depends_on` is.

**Cross-file uniqueness (issue #9 — archive split):** Phase 3 MUST refuse to
claim any task_id that appears in either `assignments.json` (active) OR
`assignments-archive.json` (terminal). Reusing a task_id from the archive
would resurrect a merged/failed task with a fresh `claimed_at` — a bug.
The `c.id not in terminal_ids` check above enforces this.

**Same-epic burst-claim guard (NEW — item #2):**

Define `epic_prefix(task_id)` as the first matching pattern:
- `^(gap-\d+[a-z]?)` (e.g. `gap-10b` from `gap-10b-stub-handlers`)
- `^(pm-[a-z-]+?)-` (e.g. `pm-security` from `pm-security-resolve-435-followups`)
- else: full `task_id`

After candidate sort, walk the list and **claim at most 2 tasks per `epic_prefix` per run**, unless the epic has at least one task in `merged` status in `assignments.json` already (cold-epic protection: avoid spending all 3 slots on the same blocked epic). If the 3rd candidate has the same prefix as the first 2 picked, skip it and continue scanning for a different-prefix candidate. If no different-prefix candidate exists, claim only the 2 — `free_slots=1` is fine, do not pad with same-prefix.

For each picked task: `branch = "auto-impl/" + first_40_chars_kebab(task_id)`.

**Branch-prefix guard (ingestion contract — issue #573):**
Before appending any row to `assignments.json`, assert that `branch` starts
with `"auto-impl/"`. If for any reason the computed branch does NOT match this
prefix, log `SKIP ingestion: branch prefix guard failed for <task_id>
(branch=<branch>)` and do NOT append the row -- treat it as unclaimed.
Manual/hotfix branches (e.g. `fix/*`, `feat/*`) must never enter
`assignments.json` via this path; they are tracked outside the dispatcher.
This invariant is verified by the self-test (T16) and T7.

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
  "reclaim_attempts": 0,
  "fix_rounds": 0,
  "merge_attempted_at": null
}
```

If fewer than 3 candidates available (buffer drained), claim what's there — don't block. Log: `Phase 3: claimed=<N> same_epic_skipped=<K>`.

---

## Phase 4 — Spawn implementer subagents (PARALLEL via Task)

SKIP if `$DISPATCHER_SKIP_MUTATING == 1` (recent-run gate from Phase 1 step 2).

One per newly-claimed task IN THIS RUN. Hard cap 3.

### Subagent execution model (gap 5 — IMPORTANT)

**Phase 4 spawn is fire-and-forget.** The Claude Code SDK launches the
implementer subagent asynchronously; the dispatcher does NOT block on
its completion. The dispatcher's job in Phase 4 is to:

1. Record the row as `status=in-progress, claimed_at=now`.
2. Hand off the brief to the subagent via the `Task` tool.
3. Return — the dispatcher run ends shortly after, while implementers
   may still be executing in the background.

**Outcome observation lives in Phase 2 of the NEXT dispatcher run**,
not in Phase 4 of THIS run. Phase 2 of that next run looks at:
- Does a branch named `<row.branch>` exist on origin?
- Does a PR exist for that branch?
- What's the PR's GH state (OPEN / MERGED / CLOSED)?
- What's `COMMITS_AHEAD`?

…and authoritatively transitions the row's status from there.

The state-machine row `in-progress → review (Phase 4 returns pr=<n>)` in
the table below is a HISTORICAL note for runs that happen to complete
before the cron interval expires; in steady-state the same transition
fires in Phase 2 of the next run when it sees the freshly-created PR.

The implementer's `pr=<n>` return-line capture in this phase is still
useful (it lets fast runs short-circuit), but its ABSENCE is not failure
— the next Phase 2 will reconcile from GitHub truth.

Prompt:

> You are an implementer. Invoke `.claude/skills/ppt-implement/SKILL.md`.
> Inputs: `task_id`, `action`, `owner_role`, `priority`, `dependency` (legacy free-text), `depends_on` (gap 3 — structured array of task_ids), `branch`.
>
> **Workspace isolation (MANDATORY — issue #7).** Before invoking the skill,
> run the standard worktree preamble from the "Subagent workspace isolation"
> section above. Do all checkouts, commits, and verify-runs inside
> `/tmp/ppt-worktrees/${task_id}/`. NEVER `git checkout <branch>` in the
> dispatcher's working tree.
>
> The skill picks the specialist, runs the 3-band verify gate, runs the
> NEW scope-drift + code-reuse pre-flight checks, opens a DRAFT PR vs `dev`
> only if verify passes.
>
> Return EXACTLY (one line):
> `pr=<n|none> status=<done|partial|blocked> specialist=<name> scope_drift=<true|false> code_reuse_warn=<short|none> note=<short>`

Capture the line **IFF the subagent returns synchronously within this
run**. If it doesn't return (the SDK backgrounds it past the dispatcher's
own exit — the common case), skip this capture entirely; leave the row
as `status=in-progress` with `claimed_at=now`, and let Phase 2 of the
next dispatcher run observe the outcome from GitHub.

STATE TRANSITION (fast-path only — gap 5):

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

SKIP if `$DISPATCHER_SKIP_MUTATING == 1` (recent-run gate from Phase 1 step 2).

For each row where `status == "review"` AND (`reviewer_summary` is null OR `PR.headRefOid != row.last_reviewed_oid`):

> You are a code reviewer for PR #<n>. Task: `<task_id>: <action>`. Specialist:
> `<sp>`. Owner: `<role>`. The implementer flagged: `scope_drift=<bool>`,
> `code_reuse_warn=<short|none>`.
>
> 0. **Dedup guard (NEW — issue #3: prevents duplicate bot reviews when two
>    dispatcher runs overlap before the Phase 1 skip-gate kicks in).** Before
>    posting anything, check existing reviews on this PR:
>
>    ```bash
>    HEAD_OID=$(gh pr view <n> --repo martin-janci/property-management --json headRefOid --jq .headRefOid)
>    EXISTING=$(gh api repos/martin-janci/property-management/pulls/<n>/reviews \
>      --jq '[.[] | select(.user.type=="Bot" or (.user.login|test("^claude|^github-actions"))) | {sha:.commit_id, state, at:.submitted_at}] | sort_by(.at) | last')
>    EX_SHA=$(echo "$EXISTING" | jq -r '.sha // empty')
>    EX_STATE=$(echo "$EXISTING" | jq -r '.state // empty')
>    EX_AT=$(echo "$EXISTING" | jq -r '.at // empty')
>    ```
>
>    If `$EX_SHA == $HEAD_OID` AND `$EX_STATE` in `{APPROVED, CHANGES_REQUESTED}`
>    AND `(now - $EX_AT) < 2h`: a bot review for this exact SHA already exists.
>    SKIP posting a new review. Map state → verdict (`APPROVED→approve`,
>    `CHANGES_REQUESTED→changes`) and return:
>    `verdict=<v> head_oid=$HEAD_OID note=dedup-existing-review-at-$EX_AT`.
>
> 1. **Smart-triage metadata pull (issue #9 — token spending).** Do NOT
>    `gh pr diff <n>` blind — for big PRs that's 50-100k tokens of unfiltered
>    text into your context, most of it lockfile / generated-client noise.
>    Pull metadata first:
>
>    ```bash
>    gh pr view <n> --repo martin-janci/property-management \
>      --json title,body,checks,headRefOid,files \
>      --jq '{title, body, headRefOid,
>             checks: [.checks[] | {name, conclusion}],
>             files: [.files[] | {path, additions, deletions}]
>                    | sort_by(-(.additions + .deletions))}'
>    ```
>
>    You now have: title/body, CI status, and every changed file ranked by
>    LOC. **Total cost: a few hundred tokens** regardless of PR size.
>
> 1.5. **Triage which files actually need a full diff.** Apply these rules
>      in order and build the path-include / path-exclude lists:
>
>    | Path pattern | Decision |
>    |---|---|
>    | `**/*auth*`, `**/*security*`, `**/middleware/*`, `**/jwt*`, `**/rbac*`, `**/rls*` | **MUST full-diff** (hot path) |
>    | `backend/crates/db/migrations/**` | **MUST full-diff** + check for `DROP`, `NOT NULL`, `DEFAULT` clauses |
>    | `**/Cargo.lock`, `**/pnpm-lock.yaml`, `frontend/packages/api-client/src/**` (generated) | **SKIP** — note in body, do not diff |
>    | Files with `additions + deletions > 800` LOC | **Header + tail only**: `gh pr diff <n> -- <path> \| head -200; echo '...'; gh pr diff <n> -- <path> \| tail -100` |
>    | Test files (`**/tests/**`, `**/*_test.rs`, `**/*.test.ts`) | **Skim**: read assertions but don't deeply audit fixtures |
>    | Everything else | Full diff per file via `gh pr diff <n> -- <path>` |
>
>    Heuristic limit: target ≤ 25k tokens of diff content into your context
>    across all `gh pr diff` calls combined. If the budget is blown by hot-path
>    files alone, that's fine — security review needs the bytes. But never
>    spend the budget on lockfile diffs or generated clients.
>
> 2. After triage, run the targeted diff calls. Example for a typical PR:
>    ```bash
>    # Hot-path mandatory:
>    gh pr diff <n> -- 'backend/**/auth*' 'backend/**/security*' 'backend/**/middleware/*'
>    # Migration check:
>    gh pr diff <n> -- 'backend/crates/db/migrations/*'
>    # Normal-LOC files (under 800 each):
>    gh pr diff <n> -- 'backend/servers/api-server/src/routes/documents/*' \
>                    -- ':!**/generated/**'
>    # Big file headers only:
>    gh pr diff <n> -- 'backend/servers/api-server/src/routes/admin/big.rs' | head -200
>    ```
>
>    For tiny PRs (`additions + deletions < 500` total), skip the triage
>    overhead — the full `gh pr diff <n>` is cheap and clearer. Triage pays
>    off above ~1k LOC of changes.
>
> 3. Review against `.claude/skills/ppt-implement/agents/<sp>.md` conventions,
>    security (RLS for db-migration, auth for pm-security), regressions, tests,
>    verify bands (Tested/Built/CI parity).
> 4. **If `scope_drift=true`**: explicitly judge whether the off-area changes
>    are necessary, and if not, demand a revert in the changes verdict.
> 5. **If `code_reuse_warn != none`**: explicitly judge whether the new code
>    duplicates an existing helper named in the warning, and if so, demand
>    delegation in the changes verdict.
> 6. **JSON-key-case sanity check (NEW — item #5)**: if the triage in step 1.5
>    found any Rust test paths in the changeset, run the check on **just those
>    paths** (issue #9 — don't reload the full diff):
>
>    ```bash
>    # Find DTOs the tests touch that carry rename_all = camelCase
>    rg -n '#\[serde\(rename_all\s*=\s*"camelCase"\)\]' backend/ --type rust | head -20
>    # Path-filtered diff for snake_case JSON accessors in test files only
>    gh pr diff <n> -- 'backend/**/tests/**' 'backend/**/*_test.rs' \
>      | rg -n '^\+.*json\[\s*"[a-z]+_[a-z_]+"\s*\]' | head -20
>    ```
>
>    Skip the check entirely if no Rust test paths are in the changeset.
>    If both produce hits AND they refer to the same DTO type, demand a fix
>    in the changes verdict (this is the bug class that bit PR #473 on 2026-05-24).
> 7. `gh pr review <n> --approve --body '<summary>'` OR
>    `gh pr review <n> --request-changes --body '<bullet list>'`.
>
> Return EXACTLY (one line):
> `verdict=<approve|changes> head_oid=<PR.headRefOid> note=<short>`

Capture → `reviewer_summary`, **`last_reviewed_oid = head_oid` (item #10 — MANDATORY)**,
`last_updated = now`. STATUS UNCHANGED.

**Data invariant (NEW — gap 1):** every reviewer write MUST set
`last_reviewed_oid` to the `head_oid` returned by the reviewer subagent. It is
NEVER acceptable to write `reviewer_summary != null` while leaving
`last_reviewed_oid == null` — that combination breaks the Phase 2 re-review
gate (the `PR.headRefOid != row.last_reviewed_oid` clause silently evaluates
true against `null` on some shells but false in `jq`-style equality, and the
behaviour is platform-dependent). If the reviewer subagent's return line is
missing `head_oid=…`, treat that as a failed reviewer run: do NOT persist
`reviewer_summary`, and re-spawn on the next cycle.

**Phase-end self-check (NEW — gap 1):** after persisting all reviewer rows,
scan `assignments.json` for the invariant violation:

```bash
BAD=$(jq -r '
  .assignments
  | map(select(.reviewer_summary != null and .last_reviewed_oid == null))
  | length' .research/management/assignments.json)
if [ "$BAD" != "0" ]; then
  echo "PHASE 5 INVARIANT VIOLATION: $BAD rows have reviewer_summary but null last_reviewed_oid" >&2
  jq -r '.assignments[] | select(.reviewer_summary != null and .last_reviewed_oid == null) | "  \(.task_id) pr=\(.pr_number)"' .research/management/assignments.json >&2
  # Do not exit — this is a data-quality warning. Phase 1 of the next run
  # will force a re-review (see gap-1 backfill rule).
fi
```

**Human-gate label sweep (P6).** After capture, scan the reviewer's `note=`
substring (case-insensitive) for any of these phrases — the canonical
allow-list. Match means: the reviewer is parking the PR in draft pending a
human-only check.

| Phrase fragment (case-insensitive) | Why it's a human gate |
|---|---|
| `macos reviewer required` | Cross-platform check the bot can't do |
| `needs domain expert review` | Subject-matter judgement call |
| `needs security review` | Sec team must sign off before un-draft |
| `needs product review` | Product/UX decision |
| `human review required` | Generic catch-all |
| `do not auto-merge` | Explicit operator override |

If any fragment matches, label the PR:

```bash
gh pr edit "$PR" --repo martin-janci/property-management --add-label needs-human-review
```

If no fragment matches and the PR currently has the label, leave it alone
(humans add this label too; the dispatcher only adds, never removes).

No cap on reviewer subagents — review every pending review row in parallel.

---

## Phase 5.5 — Attempt merge for approved + green PRs

SKIP if `$DISPATCHER_SKIP_MUTATING == 1` (recent-run gate from Phase 1 step 2).

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

**CI-stuck escalation (gap 4):** if the PR's CI rollup is `IN_PROGRESS` or
`QUEUED` AND `row.merge_attempted_at != null` AND
`(now - merge_attempted_at) > 6h` → the CI is wedged. Mark the row:

```python
row.status = "failed"
row.status_changed_at = now
row.last_updated = now
row.implementer_summary += ' [ci-stuck >6h after first merge attempt; escalating]'
```

Post a comment on the PR explaining (`gh pr comment <n> --body
"Auto-escalation: CI has been IN_PROGRESS for >6h since first merge attempt at
<merge_attempted_at>. Human action required — check the runner / re-trigger /
close the PR."`), then surface in Phase 7 (`CI-stuck escalations: [PR#<n> …]`).

Do NOT spawn the merger subagent for an escalated row.

If pre-flight passes (and not CI-stuck): spawn ONE Task subagent per PR
(cap 2 parallel). **Set `row.merge_attempted_at = now` BEFORE spawning**,
regardless of outcome — this is the back-off anchor.

> You are a PR merger. Invoke `.claude/skills/ppt-pr-merge/SKILL.md` end-to-end.
> Inputs: `pr_number=<n>`, `repo=martin-janci/property-management`, `base=dev`,
> `strategy=squash`, `delete_branch=true`. The skill verifies preconditions,
> auto-resolves mechanical conflicts, then `gh pr merge --squash --auto`.
> Return EXACTLY: `merged=<true|false|queued> pr=<n> note=<short>`

Capture line. `last_updated = now`. DO NOT manually set `status` here —
Phase 2 of the next cycle catches the GH `MERGED` state authoritatively.

---

## Phase 5.6 — Auto-rebase stale-approved PRs (NEW — item #6)

SKIP if `$DISPATCHER_SKIP_MUTATING == 1` (recent-run gate from Phase 1 step 2).

For each row `status == "review"` where:
- `reviewer_summary` starts with `"verdict=approve"`, AND
- the PR's `mergeable == "CONFLICTING"`, AND
- `(now - status_changed_at) > 4h`, AND
- `rebase_attempts < 3` (safety stop).

Spawn ONE Task subagent (cap 1 parallel — rebases serialize on the same base):

> You are a PR rebaser for a stale approved PR. Inputs: `pr_number=<n>`,
> `branch=<head_ref>`, `base=dev`. Do this exactly:
>
> 0. **Workspace isolation (MANDATORY — issue #7).** Run the standard
>    worktree preamble from the "Subagent workspace isolation" section
>    above (export `TASK_ID=pr-<n>`, `BRANCH=<head_ref>`). All subsequent
>    git operations run inside `/tmp/ppt-worktrees/pr-<n>/`. NEVER
>    `gh pr checkout` in the dispatcher's working tree — it displaces
>    `dev` and breaks Phase 6 of this run.
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

SKIP if `$DISPATCHER_SKIP_MUTATING == 1` (recent-run gate from Phase 1 step 2).

For each row `status == "review"` where `reviewer_summary` starts with
`"verdict=changes"`:

Spawn ONE Task subagent (cap 3 parallel — same as Phase 4's implementer cap):

> You are the PR follow-up driver. Invoke
> `.claude/skills/ppt-pr-followup/SKILL.md` in dispatcher mode for PR #<n>.
>
> 0. **Workspace isolation (MANDATORY — issue #7).** When step 2 below
>    spawns the original specialist via Task, the brief you pass that
>    specialist MUST include the standard worktree preamble from the
>    "Subagent workspace isolation" section above (export
>    `TASK_ID=<task_id>`, `BRANCH=<row.branch>`). The followup script
>    itself runs read-only `gh` calls and is safe in the dispatcher's
>    tree.
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

**Archive move (NEW — issue #9 — token spending).** For any row whose status
transitioned in this run to a terminal value (`merged` / `failed` / `done`):
append it to `assignments-archive.json` and DELETE it from `assignments.json`.
The active file must NEVER carry a terminal row after Phase 6 — that is
the whole point of the split. Pseudocode:

```bash
NEW_TERMINAL_IDS_JSON=$(printf '%s' "$transitions" | jq -R 'split("\n") | map(select(length>0)) | map(split(" ") | {id:.[0], new:.[2]}) | map(select(.new=="merged" or .new=="failed" or .new=="done") | .id)')
# Move them: append to archive, drop from active.
jq --argjson ids "$NEW_TERMINAL_IDS_JSON" '
  .assignments += ([..]_built_from_active_using_ids) | .archived_at = now
' .research/management/assignments-archive.json > /tmp/archive.new
jq --argjson ids "$NEW_TERMINAL_IDS_JSON" '
  .assignments |= map(select(.task_id as $t | $ids | index($t) | not))
' .research/management/assignments.json > /tmp/active.new
mv /tmp/archive.new .research/management/assignments-archive.json
mv /tmp/active.new  .research/management/assignments.json
```

(The exact jq is more verbose than shown — the point is `archive ∪= moved`,
`active −= moved`, atomically via mv. Implement with whatever clarity you
prefer; the invariant is what matters: post-Phase-6, no terminal rows in
active, every terminal row exactly once across the two files.)

```bash
# Row-count regression guard (issue #8, adapted for split — issue #9).
# Before split: assignments.json carried all rows; a sudden drop was always a bug.
# After split: rows legitimately leave assignments.json on terminal transition.
# Apply the regression guard to the COMBINED count (active + archive) so
# accidental wipes still trip it, but legitimate archive-moves do not.
NEW_ACTIVE=$(jq '.assignments | length' .research/management/assignments.json)
NEW_ARCH=$(jq '.assignments | length' .research/management/assignments-archive.json)
NEW_COMBINED=$((NEW_ACTIVE + NEW_ARCH))
OLD_ACTIVE=$(git show HEAD:.research/management/assignments.json 2>/dev/null | jq '.assignments | length' 2>/dev/null || echo 0)
OLD_ARCH=$(git show HEAD:.research/management/assignments-archive.json 2>/dev/null | jq '.assignments | length' 2>/dev/null || echo 0)
OLD_COMBINED=$((OLD_ACTIVE + OLD_ARCH))
if [ "$NEW_COMBINED" -lt "$((OLD_COMBINED - 2))" ]; then
  echo "PHASE 6 ABORT: combined assignments row count ${OLD_COMBINED} -> ${NEW_COMBINED} (loss > 2)" >&2
  echo "  Active: ${OLD_ACTIVE} -> ${NEW_ACTIVE}; Archive: ${OLD_ARCH} -> ${NEW_ARCH}" >&2
  echo "  Refusing to commit destructive write. Manual restore required:" >&2
  echo "  git checkout HEAD -- .research/management/assignments.json .research/management/assignments-archive.json" >&2
  exit 0
fi

git add .research/management/assignments.json \
        .research/management/assignments-archive.json \
        [.research/management/action-list.json if refilled]
# Commit-scope guard (#526): before the dispatcher's self-commit, refuse
# if `git diff --cached` strays outside `.research/management/`. Catches
# the PR #496 class of failure (stop hook bundling parallel-agent work
# into a doc-edit commit). Exit 2 = REFUSE — bail without committing.
bash .claude/skills/ppt-implement/scripts/commit-scope-guard.sh \
  --allow '.research/management/**' || {
  echo "dispatcher commit-scope-guard refused — staged paths outside .research/management/. NOT committing; surface in next run." >&2
  exit 0
}
git commit -m 'chore(research): dispatcher <yyyy-mm-dd HH:MM> — C claimed, R reviewed, M merge-attempts, X merged-now, F failed, A active, B <claimable>/<open> dep-blocked=<n>, RB rebased'
git push origin dev   # if another run committed since our pull: rebase + retry once;
                      # if still conflicts, log and bail — next run will re-evaluate state
```

---

## Phase 7 — Print summary (ALWAYS, hang lines too — item #9)

**Totals come from the archive (NEW — issue #9).** `Merged total` and
`Failed total` are counted from `assignments-archive.json`, NOT from
`assignments.json` (which now only has active rows). Use `jq | length`
so the file stays out of the LLM context:

```bash
MT_TOTAL=$(jq '[.assignments[] | select(.status=="merged" or .status=="done")] | length' \
  .research/management/assignments-archive.json)
F_TOTAL=$(jq '[.assignments[] | select(.status=="failed")] | length' \
  .research/management/assignments-archive.json)
# This-cycle counts come from in-memory transitions set, not from any file.
```

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
CI-stuck escalations:     [PR#<n> task=<id> waited=<h>, …]                (gap 4; [] if none)
Approved+CI-pending:      [PR#<n> task=<id> attempted=<iso8601> wait=<h>, …]  (gap 4; [] if none)
Rebase attempts (this run):[PR#<n> rebased=<true|false> <note>, …]  (item #6; [] if none)
Sandbox reclaims (this run):[<task_id> branch=<branch> reason=sandbox-timeout, …]  (P3; [] if none)
Empty branches deleted:   [<branch>, …]                             (item #1; [] if none)
Failed-dep cascades:      [<id> blocked-by=<dep_id>, …]             (issue #6; [] if none)
Skip-gate:                <none | "recent-run age=<m>m; mutating phases SKIPPED">  (issue #1)
Tier 2 response:          <http=<code> body="<truncated>" | not-fired>          (issue #5)
Review dedup-skipped:     [PR#<n> existing-at=<iso>, …]             (issue #3; [] if none)
Scope-drift flagged:      [PR#<n> task=<id> note=<paths>, …]        (item #3; [] if none)
Code-reuse warnings:      [PR#<n> task=<id> note=<helper>, …]       (item #4; [] if none)
Disk warning:             <none | "free=N%; cleaned to M%">         (item #7)
Merged total: <Mt_total>; this cycle: <Mt_this>
Failed total: <F_total>;  this cycle: <F_this>
Buffer:     claimable=<open_claimable_count>/36 (open=<open_count>, dep_blocked=<dep_blocked_count>) <T1: refilled +N | T2: upstream kicked | OK>
Post-merge: <due | skipped> [<scanned=N clean=K issues=M>]
Hang alerts:
  WARN (review >48h): [<task_id> PR#<n> age=<dd:hh:mm>, …]   (ALWAYS PRINT; [] if none)
  ALERT (review >7d): [<task_id> PR#<n> age=<dd:hh:mm>, …]   (ALWAYS PRINT; [] if none)
```

---

<!-- ============================================================ -->
<!-- TEMPORARY PHASE — REMOVE BY 2026-06-30 OR ONCE BASELINE IS   -->
<!-- UNDERSTOOD. Search for TEMP_PHASE_8 to find every related    -->
<!-- artifact (this section, HARD RULE bullet, README in           -->
<!-- .research/self-improvement/). Reports under                   -->
<!-- .research/self-improvement/*.md can stay as historical record  -->
<!-- after removal — they're write-once, never read by the         -->
<!-- routine.                                                       -->
<!-- ============================================================ -->

## Phase 8 — Self-review (TEMPORARY, TEMP_PHASE_8)

**Purpose:** generate a brief markdown post-mortem of this run so the
operator can spot recurring failure modes, token-spending anomalies, or
optimization opportunities that aren't visible from Phase 7's structured
counters alone. Write-once → human-read. The routine never reads its own
reports.

**Off-switch:** if `DISPATCHER_SELF_REVIEW=0` (or empty), SKIP this phase
entirely. Default behaviour when the env var is unset or `=1` is to run.
Skip is also implied when `$DISPATCHER_SKIP_MUTATING=1` (recent-run gate)
— no point reviewing a phase-2-only reconciliation.

```bash
if [ "${DISPATCHER_SELF_REVIEW:-1}" != "1" ] || [ "${DISPATCHER_SKIP_MUTATING:-0}" = "1" ]; then
  echo "Phase 8: self-review skipped (DISPATCHER_SELF_REVIEW=${DISPATCHER_SELF_REVIEW:-1}, skip_mutating=${DISPATCHER_SKIP_MUTATING:-0})"
  # fall through to end of run
else
  # Spawn the reviewer (see below)
fi
```

**Spawn:** one Task subagent (`subagent_type=general-purpose`,
**`model=opus`** — pin to Opus 4.7 for sharper diff-of-runs reasoning;
worth the extra cost for a temporary instrumented phase). NOT parallel
with anything else — this is the last thing the run does.

> You are the dispatcher self-review agent. Your job: produce ONE short
> markdown post-mortem of the just-finished dispatcher run and write it
> to `.research/self-improvement/<iso8601-utc>.md`.
>
> Inputs you have (use bash):
>
> ```bash
> # 1. The Phase 7 summary from the dispatcher commit that just landed
> git -C . log -1 --format="%H%n%s%n%n%b"
>
> # 2. Recent dispatcher commits for trend (last 8 chore(research): commits)
> git log --grep="chore(research): dispatcher" --pretty="%ai %s" -8
>
> # 3. Self-test status against current state
> bash .research/dispatcher-self-test.sh 2>&1 | tail -25
>
> # 4. Active assignments snapshot (small now post-archive split)
> jq '{active: (.assignments | length), by_status: (.assignments | group_by(.status) | map({s: .[0].status, n: length}))}' \
>   .research/management/assignments.json
>
> # 5. What got archived this run (count only — never load archive into context)
> jq '.assignments | length' .research/management/assignments-archive.json
>
> # 6. Phase 7 grep — look for non-empty alert lines in the commit body
> git log -1 --format="%b" | grep -E "^(Empty branches|Failed-dep cascades|Scope-drift|Code-reuse|CI-stuck|WARN|ALERT|disk_)"
> ```
>
> Inputs you do NOT need (do not read these — they bloat context):
> - `.research/management/assignments-archive.json` content
> - `.research/management/action-list-archive.json` content
> - `.research/management/coverage.json`
> - the dispatcher-prompt.md itself (you ARE the routine; you've seen it)
>
> Produce the report — keep it tight, **≤ 60 lines of markdown**:
>
> ```markdown
> # Dispatcher self-review — <iso8601-utc>
>
> Commit: <sha> — <short subject>
>
> ## Run shape
> Claimed=<C> Reviewed=<R> Merged-attempts=<M> Merged-now=<X> Failed=<F> Active=<A>
> Buffer=<claimable>/<open> dep-blocked=<n>
> Hang alerts: WARN=<n> ALERT=<n>
>
> ## What went well
> - <1-3 bullets — what worked, what shipped, what got unblocked>
>
> ## Anomalies / errors this run
> - <bullets — sandbox reclaims, empty-branches, scope-drift flags, CI-stuck,
>   reviewer dedup-skipped, disk warnings, anything unusual>
> - If nothing unusual: "None. Steady-state run."
>
> ## Trend vs. last 8 runs
> - <bullet — is failure rate rising, is buffer draining, are PRs hanging longer,
>   is review backlog growing, are version-bump commits eating throughput>
>
> ## Improvement ideas (concrete, scoped, optional)
> - <0-3 bullets — prompt edits, new self-test cases, missing automation>
> - <each one bounded to "1 line of change" / "small PR" / "needs separate spec">
>
> ## Self-test
> <copy the last few lines of dispatcher-self-test.sh output verbatim>
> ```
>
> Hard constraints:
>
> 1. Output file path EXACTLY: `.research/self-improvement/$(date -u +%Y-%m-%dT%H-%M-%SZ).md`.
> 2. Do NOT modify anything else. No edits to assignments / action-list /
>    prompts / skills. If you see something that looks broken, mention it
>    under "Improvement ideas" — do NOT fix it.
> 3. Do NOT commit. Just write the file. Phase 6 has already pushed the
>    dispatcher commit; this report lives uncommitted in the working tree
>    until the operator decides whether to keep it. The
>    `.research/self-improvement/` dir is git-tracked so reports DO show
>    up in `git status` as untracked — that surfaces them naturally.
> 4. Return EXACTLY (one line): `selfreview=ok path=<report-path> notes=<short>`

Capture the return line into the Phase 7 summary stdout as one extra line:

```
Self-review: <selfreview=ok path=... notes=... | skipped: DISPATCHER_SELF_REVIEW=0 | skipped: SKIP_MUTATING>
```

This line is NOT in the regular Phase 7 list above because it depends on
this whole phase being optional. Treat it as a Phase 8 epilogue line.

**Cost note (TEMP_PHASE_8):** Opus 4.7 input is ~5× Sonnet. A typical
self-review subagent run will consume ~5-10k input tokens (gathering bash
outputs + writing 60 lines markdown). Per-run cost: roughly $0.10-0.30 in
Opus pricing. At 12 runs/day that's ~$2-4/day. Acceptable for the
2-4 weeks of baseline-collection this phase exists for; **remove before
30 days are out** unless you've decided to keep it longer.

---

<!-- ============================================================ -->
<!-- END TEMP_PHASE_8                                              -->
<!-- ============================================================ -->

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
- never claim items whose `depends_on: [task_id, …]` array contains any task whose `assignments.json` row is not in `{merged, done}` (gap 3 — structured field replaces free-text `dependency` parsing)
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
- **recent-run skip-gate** (issue #1) — if `assignments.generated` is < 30min old, set `DISPATCHER_SKIP_MUTATING=1` and SKIP every mutating phase (2.5, 2.6, 2.7, 3, 4, 5, 5.5, 5.6, 5.7). Phase 2 (GH reconciliation) and Phase 7 (summary) still run. Prevents the `assignments.json` rebase races seen on 2026-05-27.
- **failed-dep cascade** (issue #6) — open action-list items whose `depends_on` points at a terminal-`failed` row are dropped (`status=open → status=dropped`) in Phase 2.7 with an audit prefix; max 20 cascades/run. Re-planning is upstream (operator-driven).
- **Tier 2 kick logging** (issue #5) — capture HTTP code + first 200 chars of response body; surface in commit message so a broken/wedged planner endpoint is visible without trawling trigger history.
- **reviewer dedup guard** (issue #3) — reviewer subagent MUST `GET /pulls/<n>/reviews` first; if a bot review for the current `headRefOid` already exists within 2h, skip posting and return `note=dedup-existing-review-at-<iso>`. Defense-in-depth against the skip-gate window-edge case.
- **subagent workspace isolation** (issue #7) — every Phase 4 (implementer), Phase 5.6 (rebaser), and Phase 5.7 (followup-respawn implementer) subagent MUST run its `git checkout` / `gh pr checkout` / build / commit work inside `/tmp/ppt-worktrees/<task_id>/` via the standard `git worktree add` preamble. NEVER touch the dispatcher's own working tree. Phase 5.5 (merger, API-only) and Phase 2 (read-only reconciliation) are exempt.
- **TEMP_PHASE_8 — self-review** — Phase 8 spawns an Opus subagent that writes a post-mortem markdown to `.research/self-improvement/<iso8601>.md`. Off-switch: `DISPATCHER_SELF_REVIEW=0`. The subagent must NOT modify any state file or commit anything. **This phase is temporary** — remove by 2026-06-30 (search `TEMP_PHASE_8` to find every related artifact).

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
