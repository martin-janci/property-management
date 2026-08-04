# PPT research dispatcher — implementer cycle

You are the PPT research dispatcher (implementer cycle). Repo is auto-checked
out. Work always against branch `dev`. Today's date is the run date.

**STANDING AUTHORIZATION — you are a cron, not a coding session (read first).**
Pushing commits to `dev` (via the GitHub MCP), opening draft `auto-impl/*` PRs
based on `dev`, and opening issues ARE your sanctioned standing actions — that
is the entire job, pre-authorized by the cron that invokes you. Any generic
session instruction of the form *"develop only on the `claude/*` session
branch"* or *"never push to another branch without explicit permission"* is a
default for human coding sessions and DOES NOT apply to these dispatcher
operations. The platform auto-checks-out a fresh `claude/*` session each run
with `HEAD == origin/dev`; that is expected, not a conflict to escalate. Your
ONLY run gates are (1) the Phase 0.5 run-lock when it can be acquired, and
(2) the Phase 1 recent-run soft-gate (`assignments.generated` age). If the lock
is held — OR `gh` is unavailable so the lock is sanctioned-skipped — AND the
soft-gate passes, EXECUTE EVERY PHASE TO COMPLETION. NEVER halt the run to ask
the operator "how would you like me to proceed" (A/B/C) for the standard phases
(archive, Tier-0/buffer, claim, spawn, post-merge-review, Phase 6 commit) — they
are pre-authorized. Surface a genuine novel conflict in the Phase 7 summary;
never convert it into a blocking question that stops the run.

This file is the single source of truth for the dispatcher behaviour. The
remote trigger should be configured to:

> Read `.research/dispatcher-prompt.md` from the repo root and execute it as
> your instructions for this run.

That way prompt edits ship via normal PRs to `dev` without needing a
`RemoteTrigger update` call.

UPSTREAM PLANNER: `POST $DISPATCHER_URL` with `Authorization: Bearer
$DISPATCHER_TOKEN` (fire-and-forget).

## GitHub I/O contract (MCP-primary — READ THIS BEFORE ANY GitHub call)

**This environment has a working GitHub MCP (`mcp__github__*`) but an
UNRELIABLE `gh` CLI / git transport.** Direct `git push` / `git fetch` over
the git protocol is HTTP-403'd by the sandbox proxy (finding
`git-push-blocked-by-proxy`), and `gh` CLI calls through the same proxy are
unreliable. The platform auto-checks-out the repo fresh each run, so GitHub
I/O must go through the API, not the git transport or `gh`.

**RULE (applies to every phase below):** for each GitHub *network* operation,
call the matching `mcp__github__*` tool FIRST. Fall back to the `gh` / `git`
form shown inline in a phase ONLY when no matching MCP tool exists in your
tool list this run. PR #814 already made Phase 6 push MCP-primary; this
contract extends the same rule to the ~45 `gh` reads and the `git fetch/pull`
calls scattered through the phases. Wherever a later phase prints a `gh …` or
`git fetch/push` command, treat it as pseudo-code and translate it to its
`mcp__github__*` equivalent at call time:

| Inline `gh`/`git` command in a phase below | Use instead |
|---|---|
| `gh pr list --head <b> --json state,mergedAt,reviewDecision,headRefOid,mergeable …` | `mcp__github__list_pull_requests` (filter head) → `mcp__github__get_pull_request` for the detail fields |
| `gh pr list --search 'head:auto-impl/…'` (orphan / dup-skip scan) | `mcp__github__list_pull_requests` (state=open, ≤100) + client-side `stem()` filter |
| `gh pr create --base dev --head <b> --draft …` | `mcp__github__create_pull_request` (`draft:true`) |
| `gh pr merge …` (Phase 5.5 — but defer to the `ppt-pr-merge` skill) | `mcp__github__merge_pull_request` (squash) |
| `gh api …/pulls/<n>/reviews` (reviewer dedup, issue #3) | `mcp__github__get_pull_request_reviews` |
| `PR.headRefOid` / `mergeable` / `reviewDecision` / `mergedAt` | fields on `mcp__github__get_pull_request` |
| `git fetch origin <b>` + `git rev-list --count origin/dev..origin/<b>` (`COMMITS_AHEAD`, item #1) | `mcp__github__get_pull_request` (compare base↔head commits) or `mcp__github__list_commits` |
| Phase 6 commit + push | `mcp__github__push_files` (already `PUSH_METHOD=mcp` default) |
| `git push origin --delete <b>` (empty-branch cleanup) | best-effort; skip if no MCP ref-delete tool — leaving an orphan branch is non-fatal |
| `gh api POST /git/refs` (Phase 0.5 atomic-CAS lock) | KEEP as `gh api` — there is no MCP atomic ref-CAS primitive; this is the one sanctioned `gh` use. If `gh api` fails, log `lock=skipped-gh-unavailable` and proceed (the Phase 1 `assignments.generated` soft-gate still guards overlap). |

Local git that does NOT touch the network stays as-is: `git rev-parse HEAD`,
`git status`, `git diff`/`git add` on the working tree, reading checked-out
files. Only GitHub *network* I/O is MCP-primary.

When you fall back to a `gh`/`git` form because the MCP tool was missing,
append `github_io_fallback=<op>` to the Phase 7 summary so the gap is visible
next run.

## Central store

`.research/management/assignments.json`

Schema per row:

```jsonc
{
  "task_id":            "string",
  "branch":             "auto-impl/<slug>",
  "status":             "in-progress | review | merged | failed | quarantined",
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
  "quarantined_at":     "iso-8601 | null", // (PR 5/5) set when Phase 5.7 quarantines a row after fix_rounds >= 3
  "quarantine_reason":  "string | null",   // (PR 5/5) short reason; e.g. "fix_rounds=3 exhausted; verdict still changes"

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
  "depends_on":  ["task_id", …],  // gap 3 — structured; canonical for Phase 3

  // -- Retry re-mint (2026-07-07, failed-task dead-end fix) — OPTIONAL --
  "retry_of":    "task_id",       // original FAILED task_id this row retries;
                                  // presence EXEMPTS the row from the Phase 3
                                  // stem-aware ARCHIVE exclusion (exact-id
                                  // exclusion still applies). Written only by
                                  // retry-remint.sh — never hand-add without
                                  // the cool-down + round-cap checks it does.
  "retry_round": 1                // 1-based; RETRY_MAX_ROUNDS (default 2) caps it
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
(`gap-…`, `pm-…`, `epic-…`) AGAINST the set of known action-list
ids. For unparseable non-empty values (owner-role names like
`pm-frontend`/`pm-qa`/`rust-backend`, or epic descriptions like
"Epic 2B WebSocket infrastructure"), write a **poisoned sentinel**
`depends_on: ["UNRESOLVED:<original-text-truncated-to-80-chars>"]`
instead of `[]` (issue #583). The Phase 3 `claimable()` predicate
already rejects any `depends_on` entry whose id is not a terminal
row in `assignments` — the poisoned sentinel naturally fails that
check, so the row stays blocked until a human resolves the legacy
text into a real task_id (or explicitly clears `depends_on` to
`[]`). Values of `null`, `""`, or the literal string `"none"`
(case-insensitive) in the legacy `dependency` field are treated as
no-dependency and migrate to `[]` (not sentinel). Phase 7 surfaces
poisoned rows under the `Unresolved-dep items:` line so they're
visible every cycle.

## Timestamp semantics

- `claimed_at` — set ONCE at claim; never changes.
- `last_updated` — bumped EVERY touch.
- `status_changed_at` — bumped ONLY when the `status` field value changes (hang signal).
- `merged_at` — set ONCE when row → `merged`; mirrors GH `PR.mergedAt`.
- Backfill missing `status_changed_at` = `claimed_at` on first read.
- `first_open_at` (**action-list items only**, NOT an assignments field) — set ONCE when an item is created `open` (backlog-refill promotion, coverage refill, or manual add); backfilled = NOW on first read for any legacy open row missing it (set-once — persist it by including `action-list.json` in the Phase 6 commit, so aging accrues across runs). Never changes while the item stays open. Consumed by the Phase 3 claim-sort **aging** term (anti-starvation); ignored everywhere else.
- Legacy compat: rows with `status == "done"` are treated equivalent to `merged` (terminal); do not migrate or touch them.

## State machine

| from        | to       | trigger                                                        |
|---          |---       |---                                                              |
| in-progress | review   | Phase 2 of a SUBSEQUENT run sees a PR exists on `<branch>` (gap 5 — async-spawn model). Fast-path: Phase 4 of THIS run captures `pr=<n>` from a sync return; either path is valid. |
| in-progress | failed   | Phase 2 detects sandbox-timeout AFTER one reclaim attempt OR Phase 2 detects empty-branch OR (fast-path) Phase 4 returns `pr=none` on a synchronous return |
| in-progress | in-progress | Phase 2 detects sandbox-timeout (`cloud-ok`: 60m, else 120m) AND `reclaim_attempts < 1` → re-spawn implementer once; bump `reclaim_attempts`, bump `status_changed_at` so the next grace window starts now |
| review      | merged   | Phase 2 sees PR MERGED on GH (set `merged_at`) — OR the Phase 5.8 merge-confirm sweep sees it in the SAME run (identical transition, same GH-truth source) |
| review      | failed   | Phase 2 sees PR CLOSED without merge — OR the Phase 2 stale-review SLA fires (`review` > `DISPATCHER_REVIEW_SLA_DAYS` with red CI and `fix_rounds >= 1` → close PR, open `from-stalled-pr` issue) |
| review      | review   | PR still open (no `status_changed_at` bump)                    |
| quarantined | failed   | (NEW 2026-07-07) Phase 2 quarantine-exit SLA: `quarantined_at` older than `DISPATCHER_QUARANTINE_SLA_DAYS` (default 7) with no operator action → close PR (branch preserved), open `from-stalled-pr` issue |
| review      | quarantined | (PR 5/5) Phase 5.7 sees `fix_rounds >= 3` AND latest `reviewer_summary` starts with `verdict=changes` (the quarantine gate at the top of Phase 5.7, before respawn). Set `quarantined_at=now`, `quarantine_reason="fix_rounds=<n> exhausted; verdict still changes"`. The PR is left OPEN on GitHub — the dispatcher just stops respawning + stops counting it toward WIP. Operator un-quarantines by editing `status` back to `review` (e.g. after a manual rebase or scope clarification). |

`merged` / `failed` are TERMINAL. **`quarantined` is SEMI-TERMINAL**: no
Phase 5/5.5 trigger returns from quarantined → other, and the operator may
manually flip the status back to `review` to resume work — but it is no
longer forever: the Phase 2 quarantine-exit SLA (above) fails the row after
`DISPATCHER_QUARANTINE_SLA_DAYS` (default 7) of operator inaction and
recycles it through a `from-stalled-pr` issue, so a quarantined stem can't
be held hostage indefinitely. Quarantined rows are excluded from claim
selection (Phase 3 dedup) AND from the WIP count (Phase 3 throttle) — they
free a slot without confusing the active pool.

## Hang detection (Phase 7)

`review` WARN > 48h, ALERT > 7d (since `status_changed_at`).

## Cap

Per-run, claim up to **`DISPATCHER_CLAIM_CAP` NEW** tasks (default **6**). There is NO
global in-progress cap — multiple cron runs can overlap and each may contribute up to
`DISPATCHER_CLAIM_CAP` in-progress, so the global in-progress count CAN exceed it. The
cap is per-run throughput, not global concurrency.

> **Throughput, deliberately.** This cap is the dispatcher's single biggest "how much
> work gets done per run" knob. It was raised from 3→6 (and `DISPATCHER_WIP_CAP` 8→16)
> on 2026-06-02 to make each run land more work — superseding the interim 8→12 WIP-only
> bump, by introducing `DISPATCHER_CLAIM_CAP` as the one source of truth and scaling
> every throughput knob to it. The WIP throttle below still provides back-pressure if
> the merge pipeline can't keep up, so a higher claim cap degrades gracefully (claims
> trickle to 0 when the pool is full) rather than piling up unbounded. Lower
> `DISPATCHER_CLAIM_CAP` only if you have a concrete reason to slow the routine down.

- Phase 3: `free_slots = min(DISPATCHER_CLAIM_CAP, max(0, DISPATCHER_WIP_CAP - WIP_NOW))` where `WIP_NOW` is the count of `{in-progress, review}` rows in `assignments.json`. Defaults: `DISPATCHER_CLAIM_CAP=6`, `DISPATCHER_WIP_CAP=16`. Set `DISPATCHER_WIP_CAP=0` to restore the WIP-unthrottled `free_slots=DISPATCHER_CLAIM_CAP` behavior. See *WIP-throttle preamble* in Phase 3.
- Phase 4: hard cap of `DISPATCHER_CLAIM_CAP` implementer subagents spawned per run (matches `free_slots` ceiling).

## Buffer

`action-list.json` should hold ≥ 72 open items (12 runs × 6 = 1 day of throughput at `DISPATCHER_CLAIM_CAP=6`).

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

**Spawn refusal rule.** Before spawning ANY subagent that performs git
operations (Phase 4 implementer, Phase 5.6 rebaser, Phase 5.7 followup
respawn), the dispatcher MUST assert that the subagent brief contains
the worktree preamble below verbatim, AND that `WORKTREE_PATH` resolves
to a directory under `/tmp/ppt-worktrees/`. If the assertion fails:
refuse the spawn, log
`Spawn-refused: <task_id> reason=missing-worktree-preamble` under
Phase 7, and surface the row for next-run retry. There is no "soft"
fallback — running git ops in the dispatcher CWD silently corrupts every
file the dispatcher commits in Phase 6, and the cost is high enough
(`subagent-workspace-contamination` finding: a local commit ahead of
remote that contaminated `assignments.json`) that the safe failure mode
is "don't spawn".

The dispatcher's working tree is recognizable by the path resolving to
the repo root (`git rev-parse --show-toplevel`) — any subagent
exec'ing git commands from that path must abort its own work and
return `status=blocked note=workspace-leak-prevented`.

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

## Phase 0.5 — Run-level lock (acquire)

**Why this exists (invariant, not war story):** at most one dispatcher run may
be in its *mutating* phases (3/3.5/5.5/6) against `origin/dev` at any instant.
When two runs overlap, every timing-sensitive phase corrupts: the claim pool is
read from a base that doesn't reflect the other run's archive (re-claiming
already-merged ids), Phase 6 archive-append races into duplicate rows, and the
local base drifts mid-run forcing a reset+re-run. The `assignments.generated`
soft-gate in Phase 1 only catches an overlapping run's *second pass*; it cannot
prevent two *first* passes from colliding. This lock closes that window.

**Mechanism — a GitHub ref as an atomic mutex, acquired over the REST API.**
Acquisition is a server-side compare-and-swap: `POST /git/refs` succeeds (201)
only if the ref does not exist, and returns 422 if it does. This is the only
atomic primitive available here, and it goes through `gh api` (REST), **not**
`git push` — direct push is HTTP-403'd by the local proxy in this environment
(finding `git-push-blocked-by-proxy`), so a push-based lock would never acquire.
TTL + holder identity ride in an annotated **tag object** the ref points at, so
the lock is fully self-describing and a dead holder is reclaimable.

```bash
REPO=$(gh repo view --json nameWithOwner -q .nameWithOwner)
LOCK_REF="refs/tags/dispatcher-lock"
LOCK_TTL_MIN=45                       # > a healthy run; a holder older than this is presumed dead
RUN_ID="${GITHUB_RUN_ID:-$(date -u +%Y%m%dT%H%M%SZ)-$$}"
HOST="$(hostname -s 2>/dev/null || echo unknown)"
NOW_EPOCH=$(date -u +%s)
git fetch origin dev --quiet 2>/dev/null || true   # Phase 0.5 runs before Phase 1's fetch
BASE_SHA=$(git rev-parse origin/dev 2>/dev/null || git rev-parse HEAD)  # tag anchor only

# Lock payload — everything a later run needs to judge staleness lives here.
LOCK_JSON=$(jq -nc --arg r "$RUN_ID" --arg h "$HOST" \
  --argjson acq "$NOW_EPOCH" --argjson exp "$((NOW_EPOCH + LOCK_TTL_MIN*60))" \
  '{run_id:$r, host:$h, acquired_at:$acq, expires_at:$exp}')

acquire_lock() {
  # Fast-path: if the ref already exists, someone holds it — skip the tag-object
  # POST entirely. This avoids leaking unreferenced tag objects on every contended
  # acquire (a non-trivial accumulation at ~12 runs/day over months). The two-step
  # is still required when the ref does NOT exist because POST /git/refs needs an
  # object to point at, but the common contended case is now zero-side-effect.
  if gh api "/repos/$REPO/git/$LOCK_REF" --jq .object.sha >/dev/null 2>&1; then
    return 1                              # ref exists → contended; caller reads holder
  fi
  # 1) create the tag object carrying the payload (cheap; orphaned only if we
  #    lose the race between the existence-check above and the ref POST below)
  local tag_sha
  tag_sha=$(gh api -X POST "/repos/$REPO/git/tags" \
    -f tag=dispatcher-lock -f message="$LOCK_JSON" -f object="$BASE_SHA" -f type=commit \
    --jq .sha 2>/dev/null) || return 1
  # 2) atomic CAS: create the ref pointing at it. 201 = we won; 422 = already held
  #    (lost the narrow race). Orphaned tag in the 422 case is documented above.
  gh api -X POST "/repos/$REPO/git/refs" \
    -f ref="$LOCK_REF" -f sha="$tag_sha" >/dev/null 2>&1
}

read_holder_field() {  # $1 = json key; echoes the current holder's value, or empty
  local ref_sha
  ref_sha=$(gh api "/repos/$REPO/git/$LOCK_REF" --jq .object.sha 2>/dev/null) || return 1
  gh api "/repos/$REPO/git/tags/$ref_sha" --jq .message 2>/dev/null \
    | jq -r --arg k "$1" '.[$k] // empty' 2>/dev/null
}
read_holder_exp()        { read_holder_field expires_at; }
read_holder_runid()      { read_holder_field run_id; }

release_lock() {
  [ "${DISPATCHER_LOCK_HELD:-0}" = "1" ] || return 0
  # Only delete a lock WE still own — if another run stole it after our TTL
  # lapsed, leave theirs alone (read_holder_runid defined above).
  local cur; cur=$(read_holder_runid)
  if [ "$cur" = "$RUN_ID" ]; then
    gh api -X DELETE "/repos/$REPO/git/$DISPATCHER_LOCK_REF" >/dev/null 2>&1 \
      && echo "lock: released ($RUN_ID)" || echo "lock: release failed — TTL (${LOCK_TTL_MIN}m) will reclaim"
  else
    echo "lock: not ours anymore (cur=$cur) — leaving for $cur"
  fi
}

if ! acquire_lock; then
  HOLDER_EXP=$(read_holder_exp)
  if [ -n "$HOLDER_EXP" ] && [ "$NOW_EPOCH" -lt "$HOLDER_EXP" ]; then
    echo "lock: held by a live run (expires in $(( (HOLDER_EXP-NOW_EPOCH)/60 ))m) — aborting fast, no mutation."
    exit 0                               # another run owns the mutex; do nothing
  fi
  # Stale (expired TTL) or unreadable holder → steal: repoint the ref, then re-confirm.
  echo "lock: stale/expired holder (exp=${HOLDER_EXP:-unknown}) — stealing."
  STEAL_TAG=$(gh api -X POST "/repos/$REPO/git/tags" -f tag=dispatcher-lock \
    -f message="$LOCK_JSON" -f object="$BASE_SHA" -f type=commit --jq .sha 2>/dev/null)
  gh api -X PATCH "/repos/$REPO/git/$LOCK_REF" -f sha="$STEAL_TAG" -F force=true >/dev/null 2>&1 \
    || { echo "lock: steal lost to a concurrent steal — aborting fast."; exit 0; }
  # Install release path BEFORE the re-confirm — any failure between the PATCH
  # success above and a later trap install would otherwise leak the stolen lock
  # until TTL expiry. release_lock is idempotent and ownership-checked, so it's
  # safe even if we lose the re-confirm right below.
  export DISPATCHER_LOCK_HELD=1 DISPATCHER_LOCK_REF="$LOCK_REF" RUN_ID
  trap release_lock EXIT
  HOLDER_EXP=$(read_holder_exp)          # re-read: confirm WE are the holder now
  [ "$HOLDER_EXP" != "$((NOW_EPOCH + LOCK_TTL_MIN*60))" ] && { echo "lock: lost re-confirm — aborting."; exit 0; }
else
  export DISPATCHER_LOCK_HELD=1 DISPATCHER_LOCK_REF="$LOCK_REF" RUN_ID
  trap release_lock EXIT                 # install immediately on a clean acquire
fi
echo "lock: acquired ($RUN_ID, ttl=${LOCK_TTL_MIN}m)"
```

**Release contract.** The lock MUST be released on EVERY exit path — normal
completion, a Phase 6 abort (goal-check / self-test / scope-guard bail), or a
mid-run error. `release_lock` is defined *before* `acquire_lock` is invoked
(see above) and the `trap` is installed the instant we win the ref — on a
clean acquire, in the `else` branch; on a steal, between the PATCH success
and the re-confirm read. This closes the window where a failure between
stealing the ref and installing the trap would have leaked the lock until
TTL expiry.

> The `exit 0` fast-abort *before* `DISPATCHER_LOCK_HELD=1` is set leaves no lock
> to release (we never acquired one) — correct. After acquisition, the `trap`
> guarantees release even on the Phase 6 `exit 0` bail paths. If the process is
> hard-killed (SIGKILL, sandbox teardown) the `trap` may not fire — that is what
> the TTL is for: the next run sees an expired holder and steals. Set
> `LOCK_TTL_MIN` comfortably above the 95th-percentile run duration.

The Phase 1 `assignments.generated` soft-gate (step 2 below) **stays** as a
cheap second line of defence for the narrow window between release and the next
run's `git pull`; the hard lock is primary. Phase 7 prints
`Run lock: acquired <run_id> ttl=<m>m | stole-stale | abort-held` so overlap and
TTL steals are visible per-run in the commit log.

---

## Phase 1 — Read state + preflight

1. **Sync to origin/dev (finding `stale-local-base-wastes-cycle`).** A plain
   `git pull --ff-only` FAILS when local `dev` has diverged (subagent
   contamination, a prior run's partial commit). A stale base then makes every
   timing-sensitive phase read a pool that doesn't reflect the archive's merged
   state — re-claiming already-merged ids — and forces a mid-run reset+re-run
   (recorded across 4 runs). So fetch first, and when local `dev` is behind OR
   has diverged from `origin/dev`, hard-reset to the remote BEFORE any gate runs:

   ```bash
   git fetch origin --quiet
   git checkout dev 2>/dev/null || git checkout -B dev origin/dev
   AHEAD=$(git rev-list --count origin/dev..HEAD 2>/dev/null || echo 0)
   BEHIND=$(git rev-list --count HEAD..origin/dev 2>/dev/null || echo 0)
   if [ "${AHEAD:-0}" -gt 0 ]; then
     # Local carries commits NOT on origin. The dispatcher never leaves work on
     # local dev (it commits via Phase 6 onto a fresh base), so AHEAD>0 is
     # contamination, not progress — discard it. origin/dev is the only truth.
     echo "preflight: local dev ahead=$AHEAD behind=$BEHIND (diverged) — hard-reset to origin/dev"
     git reset --hard origin/dev
   elif [ "${BEHIND:-0}" -gt 0 ]; then
     echo "preflight: local dev behind=$BEHIND — fast-forward"
     git pull --ff-only
   fi
   ```

   The first pass now runs against current base, eliminating the
   reset-and-re-run cycle `stale-local-base-wastes-cycle` recorded.
2. **Recent-run skip-gate** (NEW — issue #1, prevents the `assignments.json` rebase-race
   observed on 2026-05-27 when two cron runs ~1h apart both wrote `assignments.generated`):

   ```bash
   # Read assignments.generated from the freshly-pulled file.
   GEN_AT=$(jq -r '.generated // empty' .research/management/assignments.json 2>/dev/null)
   if [ -n "$GEN_AT" ]; then
     GEN_EPOCH=$(date -u -d "$GEN_AT" +%s 2>/dev/null || echo 0)
     NOW_EPOCH=$(date -u +%s)
     AGE_MIN=$(( (NOW_EPOCH - GEN_EPOCH) / 60 ))
     if [ "$AGE_MIN" -lt 25 ] && [ "$GEN_EPOCH" -gt 0 ]; then
       echo "skip-gate: assignments.generated=$GEN_AT (age=${AGE_MIN}m < 25m); another run is in flight"
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
   it survives cross-host parallel runs. Two runs within the same 25-min window
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
     Tier 1 when `open_claimable_count < BUFFER_FLOOR` (default 36). Most runs have a healthy buffer
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
   # awaiting-macos-build (finding ios-swiftui-prs-blocked-on-macos-build-gate) —
   # idempotent; Phase 5.5 tags macos-build-gated PRs with it.
   gh label create awaiting-macos-build --repo martin-janci/property-management \
     --color FBCA04 --description "Approved PR blocked on a macOS-only build the dispatcher cannot run" 2>/dev/null || true
   ```

   This creates `needs-human-review` + `awaiting-macos-build` once and is a no-op thereafter.

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

#### Stale-review / quarantine escalation SLA (NEW 2026-07-07)

The Phase 7 hang alerts (`review >48h` WARN, `review >7d` ALERT) only PRINT —
observed cost: PR #1812 sat in `review` with red CI for 13 days eating a WIP
slot, and quarantined rows had no exit at all. Apply this deterministic SLA
while iterating the same Phase 2 rows (skip if `$DISPATCHER_SKIP_MUTATING == 1`):

- **Stale red review** — `status == "review"` AND
  `(now - status_changed_at) > DISPATCHER_REVIEW_SLA_DAYS` (default 7) AND the
  PR's CI rollup has a `FAILURE`/`CANCELLED`/`TIMED_OUT` check AND
  `fix_rounds >= 1` (at least one repair was already attempted) →
  `new_status="failed"`, append `' [stale-review SLA: red CI after <d>d in
  review; escalating to issue]'` to `implementer_summary`, comment on the PR
  explaining the auto-escalation, close the PR (branch preserved), and open a
  GitHub issue titled `stale-review: <task_id> (was PR #<n>)` labeled
  `follow-up,from-stalled-pr` summarizing what was attempted and what CI
  rejected. The EVERY-CYCLE issue-ingest in Phase 2.6 re-mints it as a fresh
  `gh-issue-<N>` row — a clean-slate retry replaces an eternally-red zombie.
  Rows still in `fix_rounds == 0` are left to the normal Phase 5/5.7 loop.
- **Quarantine exit** — `status == "quarantined"` AND
  `(now - quarantined_at) > DISPATCHER_QUARANTINE_SLA_DAYS` (default 7) →
  the operator hasn't picked it up; stop holding the stem hostage.
  `new_status="failed"`, append `' [quarantine SLA: no operator action in
  <d>d]'`, comment + close the PR (branch preserved), and open the same
  `follow-up,from-stalled-pr` issue carrying the `quarantine_reason`.

Both paths surface in Phase 7 under `Stale-review escalations:`. Bounded: max
2 escalations per run (oldest `status_changed_at` first) so a backlog of
zombies drains gradually and each run's blast radius stays reviewable.

#### Orphan-PR discovery (stem-aware reconciliation)

A PR can exist on remote with no matching row in `assignments.json` —
typically because the spawning run crashed between `gh pr create` and the
row-persist step, or because a sibling implementer race-pushed a branch
the dispatcher never registered. The fire-and-forget pattern produced
4 such orphans in one observed run.

After the per-row pass above, scan remote for orphans:

```bash
# All open PRs whose head starts with auto-impl/ — the dispatcher's namespace.
gh pr list --state open --limit 100 --search 'head:auto-impl/' \
  --json number,headRefName,title,createdAt > /tmp/auto-impl-prs.json

# Build the active-stem index. stem() definition below — same as Phase 3.
ACTIVE_STEMS=$(jq -r '.assignments[]
  | select(.status=="in-progress" or .status=="review")
  | .task_id' .research/management/assignments.json \
  | sed -E 's/^(auto-impl|impl)\///; s/-(impl|fix|v2|retry|followup|wip)[0-9]*$//')
```

For each `auto-impl/` PR not already matched to a row by exact branch
match, compute `pr_stem = stem(headRefName)`. If `pr_stem` is in
`ACTIVE_STEMS` → this is a **stem-orphan**: an in-flight row exists for
the same work under a different suffix. Surface it in Phase 7 under
`Stem-orphans:` and DO NOT auto-link (linking would obscure the bug).
If `pr_stem` is NOT in `ACTIVE_STEMS` → this is a **clean orphan**:
backfill a synthetic row (`status=review`, `pr_number=<n>`,
`branch=<headRefName>`, `task_id=<headRefName.removeprefix("auto-impl/")>`,
`claimed_at=<PR.createdAt>`, `implementer_summary='[orphan-recovered]'`)
so subsequent phases can reason about it.

Phase 4's "write row before spawn" rule (see Phase 4) is the upstream
fix; this discovery step is the safety net that catches whatever slipped
through it.

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

**Cadence gate (finding `post-merge-gate-fooled-by-clone-mtime`).** Gate on the
ISO timestamp stored *inside* `last-merged-review.txt`, NOT the file's mtime. A
fresh clone resets every file's mtime to clone-time, so an mtime gate reads
`age < 24h` forever and the review never fires again after a clone. The file's
CONTENT is the last-run timestamp (the skill writes `date -u +%FT%TZ` into it),
and content survives a clone untouched. Run the post-merge review when the file
is missing/empty/unparseable OR its stored timestamp is > 24h old:

```bash
MARKER=.research/management/last-merged-review.txt
DUE=1
if [ -s "$MARKER" ]; then
  LAST=$(tr -d '[:space:]' < "$MARKER")
  LAST_EPOCH=$(date -u -d "$LAST" +%s 2>/dev/null || echo 0)
  if [ "$LAST_EPOCH" -gt 0 ]; then
    AGE_H=$(( ( $(date -u +%s) - LAST_EPOCH ) / 3600 ))
    [ "$AGE_H" -lt 24 ] && DUE=0
  fi
  # LAST_EPOCH==0 (missing/garbled timestamp) leaves DUE=1 — fail safe = run.
fi
```

If `DUE=1` (missing/empty/unparseable marker OR stored timestamp > 24h old):
spawn ONE Task subagent invoking `.claude/skills/ppt-review-merged/SKILL.md`
with `DISPATCHER_OWNED_COMMIT=1` in its env. Else SKIP (log
`Post-merge: skipped (last review <AGE_H>h ago < 24h)`).

Inputs: `repo=martin-janci/property-management`, `window=14d`, `base=dev`,
`max_prs=15`, `label=follow-up,from-merged-review`.

**State-write ownership (finding `subagent-race-on-dev-push`).** With
`DISPATCHER_OWNED_COMMIT=1` the skill WRITES `post-merge-review.json` +
`last-merged-review.txt` into the working tree and RETURNS without
committing/pushing. Phase 6 commits them with the rest of the dispatcher's
`.research/` delta — one push, not two. (The skill still opens GitHub issues
via API; that is not a `.research/` write and is unaffected.)

Return EXACTLY: `scanned=<N> clean=<K> issues=<M> note=<short>`.

---

## Phase 2.6 — Buffer guard

SKIP if `$DISPATCHER_SKIP_MUTATING == 1` (recent-run gate from Phase 1 step 2).

**Coverage referential-integrity cascade (NEW — finding `goal-check-gc1-orphan-action-list-items`).**
Run this FIRST, before computing the claimable pool — closing shipped-but-open
items shrinks `open_count`, so the same leak no longer inflates the buffer (it
was a hidden contributor to the GC3 overshoot too). The cascade is the
idempotent `gc1-reconcile.sh` and drains the two ACTIONABLE GC1 violations:

```bash
# (1) archive-terminal leak → auto-close any OPEN action-list item (ANY id shape,
#     not just gap-*: code-review-*, test-gap-*, screen-map-*, churn-hotspot-*, …)
#     whose exact task_id is terminal in the archive — merged/done → done,
#     failed → dropped (issue #1747, #1739; same leak as reclaim-of-already-merged-task-id).
# (2) stem orphan → emit to gc1-orphan-triage.md for a coverage author (NOT closed:
#     these are real stories missing from coverage.json — relink, never prune).
# Legitimate open follow-ups under a done story (exact task not merged) are left alone.
bash .research/gc1-reconcile.sh --apply
```

This is bounded and self-describing: every closed row gets a `gc1_closed`
stamp (merged PR + date), every orphan lands in the triage doc — **never a
silent prune of live work**. Include `action-list.json` (and the triage doc)
in the Phase 6 commit when the cascade closed any row. The leak pass (1)
matches on the **exact task_id** against the archive (any id shape); the orphan
pass (2) matches by the `<epic>-<story>` stem, identical to `goal-check.sh` GC1
— the stem, not the descriptive slug, is the stable join key (the slug mismatch
is what made GC1 false-flag ~95 live items as orphans before this fix). Going forward GC1 stays
green run-to-run because the leak is drained here every cycle; once the orphan
triage backlog reaches 0 (coverage authored for the missing stems), flip GC1 to
a hard Phase 6 gate (`hard=true`) the same way GC2 is enforced today.

**Dev-reconciliation pass (NEW — GH #1380 defect 1, `stale-gap-scan-buffer`).**
Run this RIGHT AFTER `gc1-reconcile.sh`, also before computing the claimable
pool. `gc1-reconcile` only catches leaks recorded in the dispatcher's OWN merge
ledger (`assignments-archive.json`). But work also lands on `dev` **out of
band** — a coverage feature shipped under a different branch/PR, a sibling
race-merge, or an implementer squash-merge the dispatcher never archived. Those
merges leave the action-list row `open`, so the next buffer-low refill re-claims
already-shipped work and the run logs `0 claimed … 0 merged` while no-op
implementers churn. This pass reconciles open items against the ACTUAL
integration branch, independent of the ledger:

```bash
# Closes any OPEN action-list item whose work already landed on $BASE (origin/dev)
# under a squash-merge whose SUBJECT begins with "<id>:". Subject-PREFIX join
# (anchored), never a body grep — so a dispatcher chore(research) commit that
# merely lists an id in its body is NOT a false positive. open -> done + a
# `dev_reconciled` evidence stamp (landing commit + subject). Idempotent.
bash .research/dev-reconcile.sh --apply
```

Bounded and self-describing like `gc1-reconcile`: it only ever flips
`open -> done` (item count is a guarded invariant — never adds/removes rows),
and every closed row carries the landing commit as evidence. Include
`action-list.json` in the Phase 6 commit when it closed any row. Items that land
out of band under a NON-id subject (no `<id>:` prefix) are not auto-matched —
those still need a manual reconcile note (that is how the Airbnb cluster was
drained in this defect). Smoke-tested by `.research/test-dev-reconcile.sh`.

**Backlog-honesty pass (NEW 2026-07-07 — supply-chain hardening).** Run RIGHT
AFTER `dev-reconcile.sh`, still before the buffer tiers. `gc1-reconcile` and
`dev-reconcile` keep the ACTION-LIST honest, but nothing kept `backlog.json`
(the Tier-1b refill SOURCE) honest against the assignment ledger: rows whose
assignment terminated weeks ago still read `open`/`ready`, so Tier-1b
re-evaluated and re-rejected the same ghosts every run and logged `avail=0`
while the backlog LOOKED full (observed 2026-07-07: all 20 open/ready rows
were terminal — mostly `failed` — in the archive). This pass flips them:
assignment merged/done → backlog `done`; failed → backlog `dropped`; each
flip carries a `reconciled` evidence stamp (never silent). Count-invariant,
idempotent, retry candidacy preserved (retry-remint reads the ARCHIVE, not
backlog.json). Smoke-tested by `.research/test-backlog-reconcile.sh`.

```bash
bash .research/backlog-reconcile.sh --apply
```

Include `backlog.json` in the Phase 6 commit when it flipped any row; surface
the count on the Phase 7 `Backlog-reconciled:` line.

**Open-issue ingestion (NEW 2026-07-02 — get things done).** Run RIGHT AFTER
the reconcilers, before the buffer-guard tiers, so freshly-ingested issues are
in this run's claim pool. Unlike the buffer refill (a starvation top-up), this
runs **EVERY cycle** regardless of buffer level — real work filed as GitHub
issues should always enter the pipeline. Fetch open issues via the **GitHub
MCP** (the cron can't run `gh` — proxy-403'd, #958), write them to a temp file,
then run the deterministic merge:

```bash
# 1. Fetch via MCP (NOT gh): mcp__github__list_issues owner=<owner> repo=<repo>
#    state=open (paginate). Persist the raw array to a temp file in the same
#    shape the REST /issues payload has: [{number,title,labels,html_url,
#    pull_request?}, …]. The MCP call is the ONLY network step; the merge is
#    deterministic + offline (GH_ISSUES_FILE injection, like dev-reconcile's
#    DEV_LOG_FILE). If the MCP fetch fails, SKIP (no file → the script no-ops)
#    — never block the run on issue ingestion.
GH_ISSUES_FILE=<tmp.json> bash .research/issue-ingest.sh --apply
```

It promotes untracked open issues into `action-list.json` as `gh-issue-<N>`
rows (PRs filtered; `EXCLUDE_LABELS` default `epic,discussion,question,wontfix,
duplicate,blocked,needs-triage` dropped; label→priority `security|critical|bug`
→high, `enhancement|backend|frontend|mobile|follow-up|from-merged-review`
→medium, else low). Dedup is thorough: skip any issue already an action-list
id/stem, an assignment id/stem, or referenced by `#<N>` in an existing row.
Each row carries `Closes #<N>` in its action, `first_open_at=NOW`, `depends_on:[]`,
and an `issue_ref:{number,url,labels}` stamp. Bounded: never past `BUFFER_CEIL`
headroom, ≤ `ISSUE_INGEST_CAP` (default 12) per run, append-only fail-closed.
Include `action-list.json` in the Phase 6 commit when it ingested any row;
surface the count on the Phase 7 `Issue-ingest:` line. Smoke-tested by
`.research/test-issue-ingest.sh`.

**Closing the loop (get things DONE).** A `gh-issue-<N>` PR targets `dev`, and
a `Closes #<N>` keyword only auto-closes on merge to the **default** branch
(`main`, not `dev`) — so the issue must be closed EXPLICITLY. When Phase 2 (or
Phase 5.5) observes a `gh-issue-<N>` assignment go `merged`, close issue `#<N>`
via `mcp__github__update_issue` (`state=closed`) and post a one-line
`mcp__github__add_issue_comment` linking the merged PR (`resolved by #<pr> on
dev`). Surface on the Phase 7 `Issues-closed:` line. If the close MCP call
fails, log it and continue — the merge already landed; a stale-open issue is
re-closed idempotently next run (the `gh-issue-<N>` row is terminal, so it is
not re-ingested).

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

**Buffer bounds (shared with `goal-check.sh` GC3 — finding `goal-check-gc3-buffer-overshoot`).**
The floor/target/ceiling are ONE source of truth, defined in `goal-check.sh`
(`BUFFER_FLOOR=36`, `BUFFER_TARGET=72`, `BUFFER_CEIL=120` — doubled 2026-06-02 to
track the claim-cap raise 3→6; they scale with `DISPATCHER_CLAIM_CAP`). Export the same three
here so refill, drain, and the GC3 gate can never disagree — a forked literal
is exactly how claimable drifted to ~2× the ceiling (112/60) unmeasured-against:

```bash
# Read the canonical defaults straight out of goal-check.sh (which owns them)
# so the dispatcher and the GC3 gate share one value. The grep pulls the
# `${BUFFER_FLOOR:-36}`-style default; the `:=` fallbacks below cover the case
# where the line moves or is unreadable, so the run never aborts on this.
read_bound() { grep -oE "$1:-[0-9]+" .research/goal-check.sh | head -1 | grep -oE '[0-9]+$'; }
BUFFER_FLOOR="${BUFFER_FLOOR:-$(read_bound BUFFER_FLOOR)}"
BUFFER_TARGET="${BUFFER_TARGET:-$(read_bound BUFFER_TARGET)}"
BUFFER_CEIL="${BUFFER_CEIL:-$(read_bound BUFFER_CEIL)}"
: "${BUFFER_FLOOR:=36}" "${BUFFER_TARGET:=72}" "${BUFFER_CEIL:=120}"
export BUFFER_FLOOR BUFFER_TARGET BUFFER_CEIL
```

- **Tier 0 (overflow drain — NEW):** if `open_claimable_count > BUFFER_CEIL` → the buffer is overfull (a coverage-rubric or planner push exceeded the cap; Tier 1's own top-up can never overshoot, but external pushes can). **Drain the excess back to backlog** so claimable converges to `BUFFER_CEIL`. The sort key is the canonical priority rank — `critical=4, high=3, medium=2, low=1` (anything else=0), the same `pri_rank` used by `.research/pick-target-epic.sh:99-104` — **ascending**, with `id` ascending as the deterministic tiebreaker so two runs picking the same overflowing buffer drop the same rows. Set `status="deferred"` on the lowest-ranking `(open_claimable_count - BUFFER_CEIL)` items (they stay in `action-list.json`, just leave the claimable pool — Tier 1 re-opens them per the inverse path below). This is a bounded, scored drop — **never silent**: log each deferred id with its rank. Log `Tier 0: drained <N> (claimable <old> → <BUFFER_CEIL>); deferred=[<id> rank=<r>, …]`. Include `action-list.json` in the Phase 6 commit when any item is deferred.

  Concrete jq (mirrors `pick-target-epic.sh`'s pri_rank — keep them in sync; any deviation re-opens the determinism gap this finding closed):

  ```bash
  jq -r '
    def pri_rank:
      if . == "critical" then 4
      elif . == "high" then 3
      elif . == "medium" then 2
      elif . == "low" then 1
      else 0 end;
    [ .items[] | select(.status=="open") ]
    | map(. + { _rank: (.priority | pri_rank) })
    | sort_by([._rank, .id])
    | .[0:(($claimable | tonumber) - ($ceil | tonumber))]
    | .[].id
  ' --arg claimable "$open_claimable_count" --arg ceil "$BUFFER_CEIL" \
    .research/management/action-list.json
  ```

  **Self-test impact.** `"deferred"` is a new action-list `status` value not present in any existing fixture. Self-tests that classify rows by status (T24 one-open-per-stem, the legacy-dependency invariants, any coverage of `action-list.json` rows) must treat `deferred` as a **non-terminal claimable-pool exclusion** — equivalent to `open` for stem-uniqueness and dep-graph checks, but NOT counted toward `open_claimable_count`. When adding fixtures, include at least one `deferred` row so the predicates are exercised.
- **Tier 1 (self-refill):** if `open_claimable_count < BUFFER_FLOOR` (half of the `BUFFER_TARGET` target) → **first, re-open any deferred rows** (inverse of Tier 0): flip `status` from `deferred` back to `open` in `pri_rank` *descending* order (id ascending tiebreaker) until either no deferred rows remain OR `open_claimable_count == BUFFER_TARGET`. This is the closing half of the Tier 0 / Tier 1 cycle — without it, deferred rows would accumulate and the buffer could starve while a valid backlog sits idle. Log `Tier 1: re-opened <N> deferred [<id>, …]` when any flip occurs. **Then**, if still below `BUFFER_FLOOR`, **NOW read `coverage.json`** (was previously loaded in Phase 1; deferred to here in issue #9). If coverage has stories → refill using rubric, appending only up to the cap: `refill_n = min(BUFFER_TARGET, BUFFER_CEIL) - open_claimable_count` (the `min` is belt-and-suspenders — `BUFFER_TARGET <= BUFFER_CEIL` by construction, so Tier 1 alone can never overshoot the GC3 ceiling). Log `Tier 1: <old_claimable> → <new_claimable> (+N, cap=BUFFER_CEIL)`. When `open_claimable_count >= BUFFER_FLOOR` the file is never opened, saving ~10k tokens / run.
- **Tier 1b (in-repo backlog self-refill — NEW 2026-07-02, planner-independent):** after the Tier-1 coverage rubric, if the buffer is still starved, promote fresh `open`/`ready` vectors from `.research/backlog.json` (the research routine's continuously-refreshed, SCORED, PLANNED output) into `action-list.json`. This runs **BEFORE** the Tier-2 planner kick so the buffer self-heals even while `DISPATCHER_URL` is the mis-configured no-op (GH #1380 defect 2 — an operator-only secret the dispatcher cannot fix). Deterministic + idempotent. It computes its **OWN honest claimable count** (open rows whose id AND stem are absent from assignments active+archive) rather than trusting `open_claimable_count`, so the metric blind spot that inflates the buffer with ghost rows (findings.json) cannot suppress the refill. Bounded: never lifts honest claimable above `BUFFER_CEIL`, and promotes at most `BACKLOG_REFILL_CAP` (default 24) per run to stay well inside the MCP inline-push size limit (issue #1014 — the same corruption vector `action-list-reconcile.sh` guards). Priority is **score-based** on the routine's own 0–8 scale (`>=6`→high, `3–5`→medium, `<3`→low; `>=3` is the routine's actionable bar, routine-prompt.md:121; `confidence:low` downgrades one tier); each promoted row is stamped `source="dispatcher-backlog-refill <iso>"`, `first_open_at=NOW`, `depends_on:[]`, plus a `backlog_ref` evidence object. Only ever APPENDS rows (never mutates/removes existing ones; the script fails closed if the item count doesn't grow by exactly the promote count). Include `action-list.json` in the Phase 6 commit when it promoted any row; surface the count on the Phase 7 `Backlog-refill:` line.
  ```bash
  # Defaults (36/72/120, cap 24) already match goal-check.sh — pass overrides only if the run set them.
  bash .research/backlog-refill.sh --apply
  ```
- **Tier 1c (failed-task retry re-mint — NEW 2026-07-07):** after Tier 1b, if
  the buffer is still starved (`< BUFFER_FLOOR`), give retryable FAILED tasks a
  bounded second life. A `failed` archive row permanently blocks its id AND
  stem (issue #1739 #3 — correct as a dup guard), but that also dead-ends tasks
  that failed for TRANSIENT reasons: `pr_number=null` means the implementer
  died before a PR even existed (sandbox timeout, verify-env failure), not that
  the implementation was rejected. `retry-remint.sh` re-mints those as
  `<stem>-retry<N>` action-list rows carrying `retry_of` + `retry_round`, under
  hard guards: cool-down `RETRY_COOLDOWN_DAYS` (default 7) since the newest
  failure in the stem group, `RETRY_MAX_ROUNDS` (default 2) lifetime rounds per
  stem, never for stems with landed work or any active/live sibling (T24 +
  PR 5/5 hold), ≤ `RETRY_REMINT_CAP` (default 6) per run, never past
  `BUFFER_CEIL` headroom, append-only fail-closed. **Dev-truth guards (issue
  #2153 — ghost retries):** the archive ledger alone is a blind spot — work
  can land on dev OUT OF BAND under a different task_id/PR (observed: 2
  reality-web `code-review-*` retries whose fixes had merged via #1676 etc.,
  plus `10b-2-feature-flag-management-retry1` for a story coverage already
  marked done). So before minting, the script also excludes (a) stems whose
  story id is `status="done"` in `coverage.json` (`COVERAGE_FILE` injectable;
  missing file degrades to no exclusion), and (b) stems matched by an anchored
  `<id>:` commit-subject prefix on `origin/dev` — the same squash-merge join
  convention as `dev-reconcile.sh`, never a loose body grep (`DEV_LOG_FILE`
  injectable for tests; default `git log origin/dev --format=%s -300`;
  unreachable remote degrades to no exclusion). The `retry_of` field is what
  Phase 3 and backlog-refill.sh honor to exempt the row from the STEM-aware
  archive exclusion (exact-id exclusion still applies — satisfied by the
  `-retry<N>` suffix). Smoke-tested by `.research/test-retry-remint.sh`.

  ```bash
  bash .research/retry-remint.sh --apply
  ```

  Include `action-list.json` in the Phase 6 commit when it minted any row;
  surface the count on the Phase 7 `Retry-reminted:` line.
- **Tier 1d (demand-driven vector generation — NEW 2026-07-07):** after Tier
  1c, if the buffer is STILL `< BUFFER_FLOOR`, don't wait for the daily
  cadence — starvation should trigger PRODUCTION, not just promotion. Spawn
  exactly ONE generator subagent this run (cap 1; skip if
  `$DISPATCHER_SKIP_MUTATING == 1`), alternating deterministically on run
  parity (`stats.runs % 2`):
  - **even** → a `ppt-dev-review` static-review slice (its normal segment
    rotation applies) — findings become `code-review-*` signals;
  - **odd** → a `ppt-review-merged` pass over the last 14 days — findings
    become labeled follow-up GitHub issues, which the EVERY-CYCLE issue-ingest
    above then pulls into the claim pool automatically.

  The generator runs under `DISPATCHER_OWNED_COMMIT=1` like every analysis
  subagent (single-writer rule) — it writes artifacts into the working tree /
  GitHub issues and RETURNS; this orchestrator folds any `.research/**` deltas
  into its own Phase 6 commit. Freshly-generated items enter the pool next run
  (or this run via issue-ingest ordering) — do NOT re-run the earlier tiers
  after the generator returns. Log `Tier 1d: spawned <dev-review|review-merged>
  (claimable=<n> < floor=<floor>)` and surface the outcome on the Phase 7
  `Generator:` line. If the 24h post-merge gate in Phase 2.5 already ran a
  review-merged pass THIS run, spawn the dev-review slice instead (never two
  review-merged passes in one run).
- **Tier 2 (upstream kick):** if `open_claimable_count` still `< BUFFER_FLOOR/2` (default 18 — deep-starvation last resort, scaled with the floor so it stays proportional to throughput; raised from a hardcoded 12 alongside `BUFFER_FLOOR` 18→36 so Tier-2 doesn't leave a wide dead band below the Tier-1 floor) OR coverage missing → `curl POST $DISPATCHER_URL` with `Bearer $DISPATCHER_TOKEN`, the `anthropic-version` header, a `{"text": …}` body, and `--max-time 10`. **Capture the response code AND first 200 chars of body** (NEW — issue #5: HTTP 400 from the planner used to vanish into fire-and-forget; now we see it. The body/header shape is fixed per issue #1151 / #1380 — see the comment below):

  ```bash
  T2_TMP=$(mktemp)
  # The $DISPATCHER_URL routine-fire endpoint accepts ONLY a `{"text": "..."}`
  # body and REQUIRES the `anthropic-version` header (issue #1151 / #1380): the
  # old `{"reason","claimable"}` body 400'd with `claimable` rejected as an extra
  # input, so the deep-starvation kick never fired. Carry the buffer context in
  # the `text` trigger message instead.
  T2_CODE=$(curl -sS -X POST "$DISPATCHER_URL" \
    -H "Authorization: Bearer $DISPATCHER_TOKEN" \
    -H "anthropic-version: 2023-06-01" \
    -H "Content-Type: application/json" \
    -d '{"text":"buffer-low: claimable='"$open_claimable_count"'/'"$BUFFER_TARGET"' — refill planner"}' \
    -o "$T2_TMP" -w '%{http_code}' --max-time 10 2>/dev/null || echo "curl-error")
  T2_BODY=$(head -c 200 "$T2_TMP" 2>/dev/null | tr '\n' ' ' | sed 's/[[:space:]]\+/ /g')
  rm -f "$T2_TMP"
  echo "Tier 2: http=$T2_CODE body=\"${T2_BODY:-<empty>}\""
  ```

  Still semantically fire-and-forget (we don't retry on non-2xx), but the body
  surfaces in the dispatcher commit log so a stuck/broken planner endpoint is
  visible without grepping the trigger's run history.

  **Endpoint contract + misconfig diagnostic (GH #1380 defect 2).** `$DISPATCHER_URL`
  MUST point at the **planner / coverage-refill routine trigger** — a webhook that
  accepts the small `{"reason":…,"claimable":…}` JSON above with
  `Authorization: Bearer $DISPATCHER_TOKEN` and returns 2xx. It is NOT an
  Anthropic API endpoint. **Diagnostic:** if Tier-2 logs `http=400` with a body
  that complains about a missing/invalid `anthropic-version` header (or otherwise
  looks like a Messages-API error), then `$DISPATCHER_URL` has been mis-set to an
  **Anthropic API proxy** (e.g. the CCR `…/v1/messages` proxy) instead of the
  planner trigger — the proxy rejects this payload because it expects an
  `anthropic-version` header and a Messages body. This is a **secret/env
  misconfiguration on the cloud trigger, not an in-repo bug**: the dispatcher
  cannot self-heal it (we never commit the secret). Remediation — the operator/CTO
  must repoint the trigger secret:
  - Set `DISPATCHER_URL` to the planner routine's webhook trigger URL (the same
    Claude.ai-routine trigger shape as the dispatcher's own
    `trig_01RDNN7kYxzr4XULbi4xn5r2`; see `.research/dispatcher-trigger-bootstrap.md`),
    NOT an `api.anthropic.com` / CCR proxy URL.
  - Keep `DISPATCHER_TOKEN` as the bearer that webhook authorizes.
  - If the planner is intentionally an Anthropic Messages call, then instead add
    `-H "anthropic-version: 2023-06-01"` and send a Messages payload — but the
    intended design here is a planner trigger, so prefer repointing the URL.

  Until the secret is corrected, Tier-2 stays a logged no-op (the buffer is
  refilled by Tier-0/Tier-1 + dev-reconcile, which do not depend on the planner),
  so a broken `DISPATCHER_URL` degrades gracefully rather than wedging the run.
- Else: SKIP, log `buffer OK: claimable=<open_claimable_count>/36 (open=<open_count>, dep_blocked=<dep_blocked_count>)`.

The Phase 6 commit message MUST surface both counts:

```
chore(research): dispatcher <date> — C claimed, R reviewed, M merge-attempts,
X merged-now, F failed, A active, B <claimable>/<open> dep-blocked=<n>, RB rebased, DS dup-skipped
```

**Recompute the trailer AFTER Phase 5.8 (finding `metrics-trailer-undercounts-skips`).**
The subject is written at Phase 6 (post-merge), not at claim time, so `X`/`M`
reflect this run's actual merges — including rows the Phase 5.8 merge-confirm
sweep flipped to `merged` this run. `DS` is the dup-skip count (sum of the three
Phase 3 cross-PR dedup guards) — it was previously recorded in the Phase 7 body
but omitted from the subject, so trend reads off the commit log undercounted run
activity. Every Phase 7 structured counter that has a subject token MUST agree
with its Phase 7 line.

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

## Phase 3 — Claim new (PER-RUN cap = `DISPATCHER_CLAIM_CAP`, default 6) — with same-epic guard (item #2)

SKIP if `$DISPATCHER_SKIP_MUTATING == 1` (recent-run gate from Phase 1 step 2).

### Finish-first preamble (PR 3/5 — behind `DISPATCHER_FINISH_FIRST=1`)

When the flag is set, the dispatcher biases all free slots toward **one
target epic per run** instead of spraying claims across epics. The goal
is depth-first convergence: close out one epic before starting the next.

The target epic lives in `.research/management/objective.json` (schema:
`{schema_version, epic_prefix, selected_at, last_action, reason,
stats_at_selection}`). The picker keeps it idempotent — re-running on a
target with claimable work is a no-op KEEP.

```bash
if [ "${DISPATCHER_FINISH_FIRST:-0}" = "1" ]; then
  PICK_OUT=$(.research/pick-target-epic.sh --update --json)
  TARGET_EP=$(echo "$PICK_OUT" | jq -r .epic_prefix)
  PICK_ACTION=$(echo "$PICK_OUT" | jq -r .action)
  # Surface in Phase 7 → "Target epic:" line (see Phase 7 spec).
  echo "Phase 3 finish-first: $PICK_OUT"
fi
```

The picker rule is **closest-to-done**: prefer the epic with the fewest
remaining claimable open tasks, tie-break by max priority. The dispatcher
abandons the current target only when it is exhausted (0 claimable opens
left). See `pick-target-epic.sh` for the full rule.

### WIP-throttle preamble (PR 4/5 — addresses `approved-but-unmergeable-pool-grows`)

Before sizing `free_slots`, compute the dispatcher's current
**Work-In-Progress** count and derive how many new claims this run can
absorb without growing the pool further. The throttle is a global cap on
rows whose status is `in-progress` or `review`. Goal: prevent the
approved-but-unmergeable pool from compounding when merges are slower
than claims, and create back-pressure that surfaces merge-pipeline
bottlenecks early.

```bash
CLAIM_CAP="${DISPATCHER_CLAIM_CAP:-6}"   # per-run claim/spawn ceiling (default 6)
WIP_CAP="${DISPATCHER_WIP_CAP:-16}"  # default 16 (raised from 8→12→16, 2026-06-02); set 0 to disable
if [ "$WIP_CAP" = "0" ]; then
  free_slots=$CLAIM_CAP
  WIP_NOW=$(jq '[.assignments[] | select(.status=="in-progress" or .status=="review")] | length' \
    .research/management/assignments.json)
  echo "Phase 3 WIP throttle: disabled (DISPATCHER_WIP_CAP=0); WIP=$WIP_NOW (informational); free_slots=$CLAIM_CAP"
else
  WIP_NOW=$(jq '[.assignments[] | select(.status=="in-progress" or .status=="review")] | length' \
    .research/management/assignments.json)
  # max(0, cap - WIP), then clamp to the per-run cap (DISPATCHER_CLAIM_CAP).
  HEADROOM=$(( WIP_CAP - WIP_NOW ))
  [ "$HEADROOM" -lt 0 ] && HEADROOM=0
  free_slots=$HEADROOM
  [ "$free_slots" -gt "$CLAIM_CAP" ] && free_slots=$CLAIM_CAP
  echo "Phase 3 WIP throttle: WIP=$WIP_NOW/$WIP_CAP free_slots=$free_slots (cap=$CLAIM_CAP)"
fi
```

**Smooth back-pressure.** With WIP=15 and cap=16, this run claims at most
1 new task. With WIP=16, claims 0. With WIP=20 (already over cap, e.g.
because PRs piled into review), claims 0 until merges drain it.
Phases 5 / 5.5 / 5.6 / 5.7 (review, merge, rebase, respawn) still run —
they drain the pool. Only Phase 3 (new claims) is throttled.

**Interaction with finish-first (PR 3/5).** The WIP throttle applies
UNIFORMLY regardless of `DISPATCHER_FINISH_FIRST`. Finish-first
concentrates the *quality* of claims (one epic); WIP throttles the
*quantity* (how many can be open at once). The two compose: if WIP is at
cap, no claim happens even when finish-first has a target ready. The
target persists in `objective.json` and the next run picks it up once
merges create headroom.

**Disable via `DISPATCHER_WIP_CAP=0`.** When disabled, Phase 3 claims up to the
full per-run cap (`free_slots=DISPATCHER_CLAIM_CAP`); the current WIP is logged
informationally.

```python
free_slots = $free_slots   # from the WIP-throttle preamble above

# gap 3 + issue #9: claimable iff every depends_on entry is in TERMINAL_IDS
# (the set built in Phase 2.6 from active + archive — reuse it here, do not
# rebuild). active_ids is the set of task_ids in the active assignments file.
def claimable(c, terminal_ids):
    for dep_id in (c.get("depends_on") or []):
        if dep_id not in terminal_ids:
            return False
    return True

# Fresh archive read RIGHT BEFORE claim (reclaim-of-already-merged-task-id
# finding). The TERMINAL_IDS computed in Phase 2.6 may be stale by the
# time we claim — a concurrent dispatcher run can archive a task_id
# between Phase 2.6 and Phase 3. Refresh the terminal set from disk on
# the fly so the claim predicate sees the latest archive state. Cheap
# (one jq invocation, archive stays at filesystem level).
TERMINAL_IDS=$(jq -r '.assignments[]
                     | select(.status=="merged" or .status=="done")
                     | .task_id' \
  .research/management/assignments.json \
  .research/management/assignments-archive.json \
  | sort -u)

# ARCHIVED_IDS = ALL archived/terminal ids, INCLUDING failed (issue #1739 #3).
# The claim self-exclusion below must reject ANY id already in the archive,
# not just merged/done — a previously-FAILED task_id otherwise passes the
# filter, gets claimed, then trips the cross-file duplicate self-test (T4) at
# the Phase 6 gate (the row exists in the archive as `failed`). Distinct from
# TERMINAL_IDS, which feeds dependency satisfaction (`claimable`): a failed dep
# must NOT count as satisfied, so failed ids stay OUT of TERMINAL_IDS and only
# join the self-exclusion set here. A failed task that should be retried must
# use a suffixed id (e.g. `<id>-retry`), per the stem/suffix convention.
ARCHIVED_IDS=$(jq -r '.assignments[]
                     | select(.status=="merged" or .status=="done" or .status=="failed")
                     | .task_id' \
  .research/management/assignments.json \
  .research/management/assignments-archive.json \
  | sort -u)

# PR 5/5 — stem-aware active check, parallel to the terminal check.
# active_stems and quarantined_stems both block re-claim under a
# suffix-variant slug; quarantined deserves explicit attention because
# the operator may be mid-triage and a parallel claim under -impl/-v2
# would duplicate the manual work in flight.
active_stems = {stem(r.task_id) for r in assignments
                if r.status in ("in-progress", "review", "quarantined")}

# MOBILE_NATIVE_GATED = the set of open action-list ids whose work is
# mobile-native/KMP-owned and therefore STRUCTURALLY UNLANDABLE in the cloud
# runner (issue #2652 — the `mobile-native/` verify gate `./gradlew
# spotlessCheck test` fails at config: AGP resolves from dl.google.com, which
# the egress proxy 403-denies). Claiming one burns a slot + an implementer
# subagent that can only ever return partial/blocked. This is the claim-time
# analogue of the Phase 5.5 `awaiting-macos-build` PR gate. The predicate is a
# deterministic shell helper (id-token OR pure-mobile-native plan Files scope);
# see the "Mobile-native / KMP claim-time gate" subsection below.
MOBILE_NATIVE_GATED=$(.research/mobile-native-gate.sh \
  --action-list .research/management/action-list.json | cut -f1 | sort -u)

candidates = [c for c in action-list
              if c.status == "open"
              and c.id not in active_ids
              and c.id not in ARCHIVED_IDS                  # exact-id archive check (incl. failed — issue #1739 #3)
              and c.id not in MOBILE_NATIVE_GATED           # issue #2652: skip KMP/mobile-native — unlandable in cloud
              and (c.get("retry_of")                        # retry re-mint rows (Tier 1c) EXEMPT from the stem-aware
                   or stem(c.id) not in {stem(t) for t in ARCHIVED_IDS})   # archive check — their stem matches the failed
                                                            # original BY DESIGN; retry-remint.sh already enforced
                                                            # cool-down + round cap + landed/active-stem holds.
              and stem(c.id) not in active_stems            # stem-aware active+quarantined check (PR 5/5) — NOT exempted:
                                                            # a live sibling still blocks a retry claim
              and claimable(c, TERMINAL_IDS)]               # dep satisfaction: merged/done only (failed deps unsatisfied)

# --- Anti-starvation aging (NEW 2026-07-02) --------------------------------
# Static priority alone STARVES low-priority backlog: it waits forever behind
# a steady drip of higher-priority claims. Age each open candidate so it
# eventually competes. This is LOCAL to the claim sort — Tier-0 defer and
# pick-target-epic.sh keep raw pri_rank, so their determinism invariants are
# untouched. One priority mapping for the whole file: the canonical pri_rank
# (critical=4 high=3 medium=2 low=1 else=0), same as pick-target-epic.sh:99.
AGE_BOOST_HOURS = int(env "DISPATCHER_AGE_BOOST_HOURS" default 48)
def pri_rank(p):   # canonical — bigger = higher priority
    return {"critical": 4, "high": 3, "medium": 2, "low": 1}.get(p, 0)
for c in candidates:
    if c.get("first_open_at") is None:      # set-once backfill (persist in Phase 6 commit)
        c["first_open_at"] = NOW
def eff_rank(c):                            # bounded aging: at most +1 tier, capped at 4
    aged = (NOW_EPOCH - iso_epoch(c["first_open_at"])) >= AGE_BOOST_HOURS * 3600
    return min(4, pri_rank(c["priority"]) + (1 if aged else 0))
# Sort: higher eff_rank first, then source_rank, then OLDEST first_open_at first
# (deterministic tiebreak — the longest-waiting item wins a tie).
candidates.sort(key=lambda c: (-eff_rank(c), source_rank(c["source"]), c["first_open_at"]))
```

An item's aged boost lifts it by **one tier only** (e.g. a `low` waiting past
`DISPATCHER_AGE_BOOST_HOURS` competes as `medium`), so priority still dominates
fresh work but nothing waits forever. Because the boost never exceeds +1 and
never touches Tier-0/pick-target ranking, no existing priority invariant or
finish-first behaviour changes — only the claim order among the eligible pool.

The stem-aware terminal check catches the case where a suffix-variant
slug (e.g. `<id>-impl`, `<id>-v2`) is being claimed while the canonical
`<id>` is already terminal. Without it the dispatcher would reclaim the
same gap under a renamed task_id and produce a duplicate PR.

The PR 5/5 stem-aware active+quarantined check extends the same idea to
non-terminal states: if `<id>` is already `in-progress`, `review`, or
`quarantined`, no variant `<id>-impl` can be claimed in parallel.
Quarantined inclusion matters because the operator may be doing manual
work on the row — silently claiming a sibling defeats the quarantine
contract.

The legacy `dependency` free-text field is NOT consulted by the claim
predicate. Only `depends_on` is.

**Cross-file uniqueness (issue #9 — archive split):** Phase 3 MUST refuse to
claim any task_id that appears in either `assignments.json` (active) OR
`assignments-archive.json` (terminal). Reusing a task_id from the archive
would resurrect a merged/failed task with a fresh `claimed_at` — a bug.
The `c.id not in ARCHIVED_IDS` check above enforces this (ARCHIVED_IDS
includes `failed`, so previously-failed ids are also refused — issue #1739 #3).

**Mobile-native / KMP claim-time gate (NEW — issue #2652):**

The mandatory verify gate for any `mobile-native/` change is
`cd mobile-native && ./gradlew spotlessCheck test`, which fails at Gradle
*configuration* in the cloud runner: AGP resolves from `dl.google.com`, and the
org egress proxy 403-denies that host. Every mobile-native/KMP task is
therefore **structurally unlandable here** — an implementer subagent can only
ever return `partial`/`blocked`, so claiming one wastes a claim slot and a
subagent. This is the *claim-time* counterpart of the Phase 5.5
`awaiting-macos-build` gate for iOS/Swift PRs (which fires at merge time,
after a PR already exists): both are **expected terminal-for-dispatcher states,
NOT transient blocks**, and neither the AGP-egress fix nor the ppt-bridge MCP
route is an in-repo change the dispatcher can make (both are operator-only infra
options — see issue #2652).

The predicate is a deterministic helper, `.research/mobile-native-gate.sh`,
run once per Phase 3 in batch mode over `action-list.json` (the
`MOBILE_NATIVE_GATED` set built above). A candidate is gated iff **either**:
- its `id` carries a `mobile-native` or `kmp` token (e.g.
  `code-review-mobile-native-*`, `churn-hotspot-mobile-native-*`,
  `gap-N-kmp-*`) — the churn/code-review id prefixes called out in #2652 are
  the strong signal. This deliberately does **not** match `mobile-rn` (React
  Native — verifiable in cloud via jest) or `frontend/apps/mobile/**`; or
- the candidate's plan `## Files` scope is **pure** `mobile-native/` (≥1 path
  under `mobile-native/` and every non-test source path under it). A
  cross-stack plan that also carries a verifiable backend/frontend slice is
  intentionally NOT gated.

Gated candidates are excluded from the `candidates` list (the
`c.id not in MOBILE_NATIVE_GATED` term above) so they **never consume a claim
slot**. They are NOT dropped: surface every gated OPEN id in Phase 7 under the
dedicated `Mobile-native gated:` line so the operator sees exactly which
backlog items are parked on infra (mirrors `Awaiting-macOS-build:`). Because
the dispatcher can never clear this in-repo, do not retry and do not let gated
items inflate any starvation/buffer-health metric — they are parked, not
failed. (The helper's smoke test is `.research/test-mobile-native-gate.sh`;
self-test T32 asserts the wiring stays encoded.)

**Same-epic burst-claim guard (NEW — item #2):**

Define `epic_prefix(task_id)` as the first matching pattern:
- `^(gap-\d+[a-z]?)` (e.g. `gap-10b` from `gap-10b-stub-handlers`)
- `^(pm-[a-z-]+?)-` (e.g. `pm-security` from `pm-security-resolve-435-followups`)
- else: full `task_id`

After candidate sort, walk the list and **claim at most 3 tasks per `epic_prefix` per run** (`DISPATCHER_SAME_EPIC_CAP`, default 3), unless the epic has at least one task in `merged` status in `assignments.json` already (cold-epic protection: avoid spending every free slot on the same blocked epic). Once an `epic_prefix` has reached the per-epic cap, skip further same-prefix candidates and continue scanning for a different-prefix candidate. If no different-prefix candidate exists, claim only what passed the cap — a smaller `free_slots` is fine, do not pad with same-prefix. (Raised from 2→3 on 2026-06-02 to track the higher per-run claim cap while still spreading across ≥2 epics when more than one has ready work.)

**Finish-first override (PR 3/5).** When `DISPATCHER_FINISH_FIRST=1` AND the
finish-first preamble selected a target epic (`TARGET_EP != "NONE"`):

1. Before sort, **filter candidates** to those whose `epic_prefix == TARGET_EP`.
2. The same-epic per-run cap is **lifted** — claim up to all free slots from the
   target epic. The whole point of finish-first is depth-first concentration.
3. **Fallback to the unfiltered pool** if the target epic yields 0 claimable
   candidates after the filter (e.g. all dep-blocked at this exact moment).
   This prevents the run from going idle when the target is temporarily
   stuck; the picker will repick on the next exhaustion.
4. Cold-epic protection (the existing carve-out) doesn't apply when
   finish-first is active — the operator chose to concentrate on the target
   even if it has no merges yet; that's the bootstrap case.

The fallback is intentionally narrow: it only triggers when the *filtered*
candidate list is empty, NOT when finish-first claims fewer than 3 from the
target. If the target has only 1 claimable task, claim 1 — don't pad with
other-epic candidates. Filling the run with off-target work defeats the
finish-first contract and dilutes the signal in Phase 7's `Target epic:`
line.

**Cross-PR dedup guards.**

**Invariant.** At any time, for any `stem`, AT MOST ONE non-terminal unit
of work exists across (a) open PRs on remote, (b) `assignments.json` rows
in `{in-progress, review}`, (c) ready plans about to be claimed. Two units
sharing a `stem` is a bug; two units touching ≥2 non-test files in common
is a bug.

**Stem definition** (single source of truth, reused by Phase 3, the routine
promotion gate, and the implementer pre-create check):

```python
SUFFIX_RE = r'-(impl|fix|v2|retry|followup|wip)\d*$'
def stem(task_id_or_branch_or_slug):
    s = re.sub(r'^(auto-impl|impl)/', '', task_id_or_branch_or_slug)
    return re.sub(SUFFIX_RE, '', s)
```

The suffix list is the *full* set of "second-attempt" markers used by the
routine and by hand-edits. Add a new marker here in exactly one place when
a new convention is observed; never inline-redefine the regex elsewhere.

For each candidate that survived the same-epic guard, run three checks
before appending. ANY check tripping → skip this candidate (do NOT append;
do NOT consume a slot), continue scanning the sorted candidate list. Log
each skip to Phase 7 under `Dup-skipped:` with the structured reason code
listed below; the codes are the contract Phase 8 reads when proposing
improvements.

1. **Open-PR collision (reason code: `open-pr`).** Cheap GH probe per
   candidate — title-substring search plus head-prefix search:

   ```bash
   gh pr list --state open --limit 100 \
     --json number,title,headRefName,isDraft \
     --search "in:title $stem_cur" > /tmp/dup-title.json
   gh pr list --state open --limit 100 \
     --json number,title,headRefName,isDraft \
     --search "head:auto-impl/$stem_cur" > /tmp/dup-head.json
   ```

   Hit predicate: any matched row whose `stem(headRefName)` equals
   `stem(candidate.id)`. Title-substring alone is not sufficient — a
   bug-fix PR mentioning the stem in prose would over-trigger. The
   stem-of-branch-name comparison is the load-bearing test.

   Log: `Dup-skip: <candidate.id> reason=open-pr stem=<stem> conflicts_with=#<n>`.

2. **In-flight assignment collision (reason code: `open-assignment`).**
   Same-stem rows in `assignments.json` whose status is `in-progress` or
   `review` block the claim even when no PR is open yet (sibling
   implementer mid-implement, or PR not yet pushed):

   ```python
   for row in assignments["assignments"]:
       if row["status"] in ("in-progress", "review") \
              and stem(row["task_id"]) == stem(candidate.id):
           skip(reason="open-assignment",
                conflicts_with=row["task_id"],
                other_status=row["status"])
           break
   ```

3. **File-touch overlap (reason code: `file-overlap`).** Read
   `candidate`'s plan at `.research/plans/<candidate.id>.md`, parse the
   `## Files` section into a set `files_cur`. For each open assignment
   row (`status in {in-progress, review}`), parse the same section from
   its plan. Compute the intersection.

   Trip when `|files_cur ∩ files_other| >= 2` AND at least one entry in
   the intersection is not a test file (test paths: `**/tests/**`,
   `**/*_test.rs`, `**/*.test.{ts,tsx,js,jsx}`, `**/__tests__/**`).
   Test-only overlap is allowed (parallel test work is fine).

   Log: `Dup-skip: <candidate.id> reason=file-overlap with=<other_id> shared=<count>:<first-3-paths>`.

4. **Computed-branch collision (reason code: `branch-collision`; issue
   #1747, #1739).** Even with the hash-suffixed branch name above, never
   claim a candidate whose **computed branch** equals a branch that is
   already active or recently merged — that would force-push over a live or
   just-landed PR. This is the guard the old stem-only ladder lacked: it
   compares the *exact computed branch string*, not a stem, so it catches
   the truncation-collision class directly. Build the blocked-branch set
   once per run from (a) every `in-progress`/`review` row's `branch` in
   `assignments.json`, (b) open `auto-impl/*` PR head refs, and (c)
   recently-merged `auto-impl/*` PR head refs (last 14 days):

   ```bash
   # (a) active rows + (b) open auto-impl/ PR heads + (c) recently-merged heads
   jq -r '.assignments[] | select(.status=="in-progress" or .status=="review")
            | .branch // empty' .research/management/assignments.json > /tmp/blocked-branches.txt
   gh pr list --state open --limit 100 --search 'head:auto-impl/' \
     --json headRefName -q '.[].headRefName' >> /tmp/blocked-branches.txt
   gh pr list --state merged --limit 100 --search 'head:auto-impl/ merged:>='"$(date -u -d '14 days ago' +%F)" \
     --json headRefName -q '.[].headRefName' >> /tmp/blocked-branches.txt
   sort -u /tmp/blocked-branches.txt -o /tmp/blocked-branches.txt
   ```

   Hit predicate: `compute_branch(candidate.id)` (the exact string from the
   branch-computation block below) appears in `/tmp/blocked-branches.txt`.

   Log: `Dup-skip: <candidate.id> reason=branch-collision branch=<branch> conflicts_with=<active-row|PR>`.

The four guards form a defense-in-depth ladder: (1) catches collisions
across runs, (2) catches collisions within the same run, (3) catches
semantically-equivalent slugs whose stems differ but whose plans land on
the same files, (4) catches the exact-computed-branch collision class
(truncated-prefix families) before it can force-push over a live or
recently-merged branch. Every skip writes a structured row that Phase 8
aggregates to detect systemic drift (e.g. repeated `open-assignment` skips
on the same epic signal a planner bug, not a claim-time issue).

Implementation note: guards 1–3 rely on `stem(...)`; guard 4 relies on
`compute_branch(...)` (defined in the branch-computation block below).
Define each once at the top of Phase 3 and reuse. The `gh pr list` calls are bounded by
`free_slots` (≤ `DISPATCHER_CLAIM_CAP` per run × 2 calls) — at most `2 × DISPATCHER_CLAIM_CAP` (default 12) extra `gh` invocations.

**Branch computation (collision-safe — issue #1747, #1739).**
For each picked task, compute the branch as the 40-char kebab prefix **plus a
short hash of the FULL `task_id`**, so distinct ids that share a 40-char prefix
get distinct branches:

```bash
# Stable, collision-safe branch name for a task_id.
# The 40-char kebab prefix keeps the branch human-readable; the 8-char sha1 of
# the FULL id disambiguates whole task families that share that prefix (e.g.
# churn-hotspot-backend-servers-api-server-* — 10+ ids that all truncate to
# auto-impl/churn-hotspot-backend-servers-api-server and used to collapse to ONE
# branch, capping the family to one in-flight item at a time).
compute_branch() {
  local id="$1"
  local kebab; kebab=$(printf '%s' "$id" \
    | tr '[:upper:]' '[:lower:]' | tr -cs 'a-z0-9' '-' | sed 's/^-//; s/-$//')
  local slug="${kebab:0:40}"; slug="${slug%-}"   # trim a dangling hyphen
  local h; h=$(printf '%s' "$id" | sha1sum | cut -c1-8)
  printf 'auto-impl/%s-%s' "$slug" "$h"
}
branch="$(compute_branch "$task_id")"
```

The hash suffix only affects **new** claims — in-flight branches keep their
existing names (no orphaning of live PRs #1683/#1718 etc.). Two distinct ids
can no longer share a branch unless they sha1-collide on the full id.

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

If fewer than 3 candidates available (buffer drained), claim what's there — don't block. Log: `Phase 3: claimed=<N> same_epic_skipped=<K> dup_skipped=<D>` where `D` is the count of cross-PR-dedup-guard rejections (sum across the three guards above).

---

## Phase 3.5 — Persist claims BEFORE spawning implementers

Addresses the `orphaned-prs-reconciliation-noise` finding. The
fire-and-forget Phase 4 model means a spawned implementer can open a PR
on remote before the dispatcher's Phase 6 commits the claim row to
`assignments.json` on `dev`. If the dispatcher aborts between Phase 4
and Phase 6 (sandbox timeout, network blip, manual interrupt), the PR
exists on remote but the row doesn't — Phase 2 of the next run sees an
orphan PR with no row to link to.

**Rule.** When Phase 3 claims ≥1 new row, commit + push the claim rows
to `dev` BEFORE entering Phase 4. The commit body is minimal — just the
list of claims — so it doesn't pollute the Phase 6 main commit:

```bash
if [ "$CLAIMED_COUNT" -gt 0 ]; then
  bash .research/dispatcher-self-test.sh >/tmp/pre-claim-self-test.log 2>&1 || {
    echo "Phase 3.5 ABORT: self-test FAIL after claim — refusing to publish claims. See /tmp/pre-claim-self-test.log" >&2
    exit 0
  }
  git add .research/management/assignments.json
  bash .claude/skills/ppt-implement/scripts/commit-scope-guard.sh \
    --allow '.research/management/**' || exit 0
  git commit -m "chore(research): dispatcher pre-spawn claim — ${CLAIMED_COUNT} new task(s)"
  git push origin dev || {
    # Push lost a race with a concurrent run — bail without spawning.
    # Phase 6 of this run is a no-op; Phase 2 of the next run will see
    # whichever rows actually landed on dev.
    echo "Phase 3.5 ABORT: push lost race — claims not durable, NOT spawning implementers." >&2
    exit 0
  }
fi
```

This commit is intentionally separate from the Phase 6 main commit. The
two-commit pattern guarantees: by the time any implementer starts work,
its claim row is durable on `dev`. If the rest of the run aborts, the
claim is still recoverable. If the push loses a race, the dispatcher
bails *before* spawning, so no orphan PR can ever exist for this run.

Phase 6's commit later in the run picks up everything else (Phase 2
reconciliation results, Phase 5 reviewer summaries, archive moves) on
top of this base.

---

## Phase 4 — Spawn implementer subagents (PARALLEL via Task)

SKIP if `$DISPATCHER_SKIP_MUTATING == 1` (recent-run gate from Phase 1 step 2).

One per newly-claimed task IN THIS RUN. Hard cap = `DISPATCHER_CLAIM_CAP` (default 6).

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

### State-write ownership (single-writer contract — finding `subagent-race-on-dev-push`)

**Invariant.** `.research/**` on `dev` has exactly one writer: the dispatcher
process. Every other actor either writes to its own feature branch or acts on a
PR through the GitHub API — never commits or pushes `.research/**`.

**Why.** Each independent push to `dev` is a discrete event: `research-land`
replays it and (outside the GITHUB_TOKEN-no-retrigger path) `version-bump` fires.
When a spawned skill pushes its own `.research/` state mid-run, two failures
follow: (1) the version churns once per extra push (observed 0.2.964→0.2.968 in
one run), and (2) the dispatcher's Phase 6 `git commit` finds *nothing to commit*
because the skill already published the delta the dispatcher intended to own.
Collapsing all `.research/` writes into the dispatcher's two commits makes the
Phase 6 delta deterministic and cuts dev pushes to the minimum.

**Per-actor write rules:**

| Actor (phase) | May write | MUST NOT |
|---|---|---|
| Implementer (4), rebaser (5.6), followup (5.7) | its own feature branch, inside `/tmp/ppt-worktrees/<task_id>/` | touch `.research/**`; push `dev`; run git ops in the dispatcher CWD |
| Reviewer (5) | a GitHub PR review via API | write any file; push anything |
| Merger (5.5) | `gh pr merge` (the PR landing) + the PR head branch for conflict fixups | write `.research/**` |
| Analysis: dev-review (1.5), project-management (1.6), post-merge-review (2.5) | `.research/**` artifact files **in the working tree**, then RETURN | `git add/commit/push` — persistence is deferred to the owning orchestrator's commit |
| **Orchestrator** — dispatcher (2.5/3.5/6) or routine (1.5/1.6 → its Phase 6 `git add .research/`) | commits + pushes ALL `.research/**` for the phases it owns | — |

**Mechanism.** When the dispatcher spawns an analysis skill it exports
`DISPATCHER_OWNED_COMMIT=1`. Each such skill checks that flag and, when set,
skips its own commit/push step (it still writes its files). Phase 6 then stages
those files alongside the assignment/action-list state and commits once. Skills
invoked standalone (no flag) keep committing as before.

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

**Spawn config:** every reviewer subagent runs with
`subagent_type=general-purpose, model=opus` (pinned to Opus 4.8). The
reviewer's job — diff-triage, security read, scope-drift judgement, vendor
the verdict line — benefits materially from Opus's reasoning over Sonnet's
default; the dispatcher's per-PR review is the single point where the
bot decides "merge or block", so cost-per-PR is well spent. Reviewer cap
is unbounded by phase contract; the cost ceiling is the count of pending
`review` rows in `assignments.json`.

For each row where `status == "review"` AND (`reviewer_summary` is null OR `PR.headRefOid != row.last_reviewed_oid`):

**Spawn the reviewer subagent** with the verbatim prompt at
`.research/management/pr-reviewer-prompt.md`. Do NOT inline the rubric here —
pass the Task subagent the runtime values for this row, then a one-line
instruction to read that file and follow it exactly, e.g.:

> You are a code reviewer for PR #<n>. Task: `<task_id>: <action>`. Specialist:
> `<sp>`. Owner: `<role>`. The implementer flagged: `scope_drift=<bool>`,
> `code_reuse_warn=<short|none>`. Read `.research/management/pr-reviewer-prompt.md`
> and follow it exactly — it is your complete instruction set (dedup guard,
> smart-triage, hot-path rules, JSON-key-case check (item #5), verdict steps).
> Substitute the runtime values
> above wherever the file references `<n>` / `<task_id>` / `<action>` / `<sp>` /
> `<role>` / `scope_drift` / `code_reuse_warn`. Return ONLY the single
> `verdict=<approve|changes> head_oid=<PR.headRefOid> note=<short>` line it specifies.

The subagent reads its ~100-line triage rubric/schema in its own context, so
those lines never load into the dispatcher run. Keep the spawn config above,
the capture/invariant logic, and the human-gate sweep below inline — they are
dispatcher-side.

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

## Phase 5.4 — Pre-merge autofix (mechanical short-circuit, PR 5/5)

SKIP if `$DISPATCHER_SKIP_MUTATING == 1` (recent-run gate from Phase 1 step 2).

**Purpose.** Many "ci-fail" approved PRs fail only because the base
moved and the failures are confined to **mechanical paths**: SQLx
offline JSON, `Cargo.lock`, generated OpenAPI clients, lockfiles.
Routing these directly to Phase 5.7 (full implementer respawn) is
wasteful — the fix is a rebase + maybe a one-line regenerate command,
not another implementation pass. Phase 5.4 intercepts these BEFORE
Phase 5.5's per-blocker classification so the cheaper fix runs first.

**Mechanical-only path set:**

```
backend/.sqlx/**
Cargo.lock
**/Cargo.lock
backend/crates/api-client/**
frontend/packages/api-client/**
frontend/packages/openapi-client/**
**/pnpm-lock.yaml
**/package-lock.json
**/Gemfile.lock
docs/api/openapi.yaml
docs/api/openapi.json
```

(`ppt-pr-merge`'s existing auto-resolve set is the source of truth —
keep this list in sync; do not let it drift.)

**Trigger.** For each row `status == "review"` AND `reviewer_summary`
starts with `verdict=approve`:

```bash
gh pr view <pr> --json statusCheckRollup,mergeable,headRefOid --jq \
  '{merge: .mergeable, head: .headRefOid,
    failing: [.statusCheckRollup[] | select(.conclusion=="FAILURE") | .name]}'
```

If `failing` is non-empty OR `merge == "CONFLICTING"`, fetch the changed
file set against `dev` and check whether the entire delta on the
*failing* paths is inside the mechanical-only set. Use a single `gh pr diff
<n> --name-only` and `grep -v` the mechanical patterns; if any non-
mechanical path remains, **skip Phase 5.4** for this row and fall through
to Phase 5.5's normal classification.

**Action.** Spawn ONE Task subagent per qualifying row (cap 2 parallel,
same as Phase 5.6's rebaser cap — both serialize on `dev` base):

> You are a pre-merge mechanical autofixer for PR #<n>, branch `<branch>`.
> Read `.research/management/premerge-autofix-prompt.md` and follow it exactly
> — it is your complete instruction set (worktree preamble, ppt-pr-merge Step 2
> conflict auto-resolution, force-push, CI re-trigger). Substitute the runtime
> values above wherever the file references `<n>` / `<branch>` /
> `<workflow-on-the-pr>`. Return ONLY the single
> `premerge=<applied|skipped|failed> pr=<n> note=<short>` line it specifies.

**Bookkeeping.** Each `premerge=applied` bumps `rebase_attempts += 1`
(single-owner rule — dispatcher does the write; subagent only returns).
**Cap: 1 mechanical-autofix per row per 24h.** Without the cap, a
genuinely-stuck PR could loop here forever; the existing 6h CI-stuck
back-off in Phase 5.5 (gap 4) takes over if mechanical autofix doesn't
unstick it.

**What Phase 5.4 is NOT.** It does NOT format / lint code, does NOT
modify non-mechanical paths, does NOT merge. It's a focused
"rebase + regenerate" shortcut that converts a mechanical-fail into a
green PR before Phase 5.5 has to decide.

Phase 5.5 in this same run picks up the now-green PR via the standard
path; the row goes from `approve + ci-fail` to `approve + ci-green`
between phases of the same dispatcher cycle, no extra wait.

---

## Phase 5.5 — Attempt merge for approved + green PRs

SKIP if `$DISPATCHER_SKIP_MUTATING == 1` (recent-run gate from Phase 1 step 2).

For each row `status=="review"` where `reviewer_summary` starts with `"verdict=approve"`:

**Pre-flight + per-blocker routing.**

```bash
gh pr view <pr_number> --json statusCheckRollup,isDraft,reviewDecision,mergeable,state,labels
```

Classify the PR into exactly one **blocker** based on the pre-flight result,
and route per the table below. The previous "skip if anything's wrong" rule
let approved PRs accumulate in a stranded pool with no escalation — the
`approved-but-unmergeable-pool-grows` finding (5 PRs stuck across 3 runs).
Each blocker now has a dedicated path.

| Blocker classification | Detection | Routing |
|---|---|---|
| `state-closed` | `state != OPEN` | Skip silently; Phase 2 already reconciled. |
| `ci-fail` | CI rollup has `FAILURE`/`CANCELLED`/`TIMED_OUT`/`ACTION_REQUIRED` | Route to **Phase 5.7 respawn** (verdict=changes path) so the implementer can fix the failing build. Do NOT mark `merged=false`. |
| `ci-in-progress` | All checks `IN_PROGRESS`/`QUEUED` | If `merge_attempted_at` is null or < 6h old → SKIP this cycle, wait. If ≥6h → `ci-stuck escalation` below. |
| `ci-unknown` | `statusCheckRollup` is empty/null (0 checks reported) | Re-trigger CI via `gh workflow run` (best-effort) and SKIP this cycle. Log under `Phase 7 → CI-retriggered:`. After 2 cycles with still-zero checks, escalate to human-review label and SKIP further attempts (genuine repo config issue, not a transient blip). |
| `dirty` | `mergeable == "CONFLICTING"` | Route to **Phase 5.6 auto-rebase** (see below). |
| `draft-ready` | `isDraft == true` AND review approved AND CI green | Pass through; `ppt-pr-merge` Step 1 auto-promotes draft → ready when the approval + green-CI gates pass. Letting drafts through is the whole point of the auto-promote path; pre-filtering re-introduces the stall (observed: 0 merge attempts in a run where every approved PR was draft). |
| `macos-build-gated` | PR head touches iOS/Swift paths (`mobile-native/**/iosApp/**`, `**/*.swift`, `**/*.xcodeproj/**`) AND a required check needs a macOS build that no runner provides (CI shows the macOS/xcodebuild check `EXPECTED`/missing, never `SUCCESS`) | **Expected terminal-for-dispatcher state, NOT a transient block.** Tag the PR `awaiting-macos-build` (idempotent), SKIP, and surface in Phase 7 under `Awaiting-macOS-build:` — a distinct digest, NOT the `Approved+human-gated` or merge-failure buckets. The dispatcher can never clear this (no macOS runner), so do not retry and do not let it inflate the unmergeable-pool metric. (finding `ios-swiftui-prs-blocked-on-macos-build-gate`) |
| `needs-human-review` | Label `needs-human-review` present | SKIP; surface in Phase 7 under `Approved+human-gated:`. Do NOT keep retrying — humans add this label specifically to stop the dispatcher. |
| `ready` | None of the above | Proceed with merge spawn. |

**Skip-vs-fail discipline.** Only `ci-fail` mutates row state (routes to
5.7 which sets `status=in-progress`); every other "skip" leaves the row
in `status=review` and emits a Phase 7 line so the operator can see why
it didn't merge this cycle. Approved-but-blocked PRs never disappear
from the visible state.

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
the Phase 5.8 merge-confirm sweep (same run, GH-truth re-poll) or Phase 2
of the next cycle catches the GH `MERGED` state authoritatively.

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
> `branch=<head_ref>`, `base=dev`. Read `.research/management/rebaser-prompt.md`
> and follow it exactly — it is your complete instruction set (worktree
> preamble, checkout, `git rebase origin/dev`, mechanical-only conflict
> resolution per ppt-pr-merge Step 2, force-push-with-lease). Substitute the
> runtime values above wherever the file references `<n>` / `<head_ref>` /
> `<branch>`. Return ONLY the single `rebased=<true|false> pr=<n> note=<short>`
> line it specifies. Do NOT touch `rebase_attempts` — the dispatcher owns it.

Capture line. Bump `last_updated = now`.

**`rebase_attempts` ownership: dispatcher-only, exactly once per spawn.**
Earlier behaviour had both the rebaser subagent AND the dispatcher
incrementing the counter, double-counting to 2 instead of 1
(`double-counted-rebase-attempts` finding). The rule:

- The **dispatcher** increments `rebase_attempts += 1` here, immediately
  after capturing the rebaser's return line, regardless of `rebased=true/false`.
  This is the single authoritative write.
- The **rebaser subagent** MUST NOT touch `rebase_attempts` (the row is
  not even accessible from the subagent's worktree under the workspace-
  isolation rule).
- The rebaser's return line does NOT carry an attempt count — only
  `rebased=<true|false>` and a short note. The dispatcher derives the new
  count from the existing row.

The same single-owner pattern applies to every counter on the assignment
row: only the dispatcher writes; subagents communicate via return lines.

Phase 5.5 next run will pick up the now-clean PR via the standard path.

---

## Phase 5.7 — Respawn implementer on `verdict=changes` (NEW)

SKIP if `$DISPATCHER_SKIP_MUTATING == 1` (recent-run gate from Phase 1 step 2).

**Quarantine gate (PR 5/5).** Before respawning, check `fix_rounds`. If
`fix_rounds >= 3` the implementer has already had 3 attempts at this row
and the reviewer is still returning `verdict=changes`. Further respawns
are unlikely to converge — they burn implementer subagent budget and
keep the row in the WIP pool, blocking new claims under the WIP throttle
(PR 4/5). Quarantine the row instead:

```bash
for row in rows_with_verdict_changes:
  if (row.fix_rounds // 0) >= 3:
    row.status = "quarantined"
    row.status_changed_at = now
    row.quarantined_at = now
    row.quarantine_reason = "fix_rounds=" + str(row.fix_rounds) + " exhausted; verdict still changes after " + str(row.fix_rounds) + " respawns"
    row.last_updated = now
    # Log to Phase 7 — see "Quarantined this run" line.
    continue  # SKIP the respawn for this row
  # else: fall through to the respawn spawn block below
```

The quarantined row is left ALONE on GitHub — the PR stays open, the
branch is preserved, the implementer history (3 attempts' worth of
commits) is intact for the operator to inspect. The dispatcher just
stops feeding the implementer/reviewer cycle on this row. Un-quarantine
is a manual edit (`status: "quarantined" → "review"`) once the operator
has done whatever the bot couldn't.

For each REMAINING row `status == "review"` where `reviewer_summary`
starts with `"verdict=changes"` AND `fix_rounds < 3`:

Spawn ONE Task subagent (cap `DISPATCHER_CLAIM_CAP` parallel, default 6 — same as Phase 4's implementer cap):

> You are the PR follow-up driver for PR #<n>, task `<task_id>`, specialist
> `<sp>`. Read `.research/management/pr-followup-prompt.md` and follow it
> exactly — it is your complete instruction set (workspace-isolation brief,
> `ppt-pr-followup` dispatcher-mode script, respawn-brief handling, post-respawn
> `status=review` flip). Substitute the runtime values above wherever the file
> references `<n>` / `<task_id>` / `<row.branch>` / `<sp>` / `<k>`. Return ONLY
> the single `followup=<...> pr=<n> specialist=<sp> round=<k>` line it specifies.

Capture line. The script already mutated `assignments.json`; this phase
adds nothing further to the file beyond the post-respawn `status=review`
flip.

Idempotency: the script's `status=in-progress` write makes a second
Phase 5.7 invocation a no-op until the spawned implementer finishes and
the next reviewer pass posts a fresh `reviewer_summary`. Hard cap is 3
fix rounds per row; subsequent calls return `failed`.

---

## Phase 5.8 — Merge-confirm sweep (NEW 2026-07-07)

SKIP if `$DISPATCHER_SKIP_MUTATING == 1` (recent-run gate from Phase 1 step 2).

Phase 5.5 enables auto-merge and deliberately does not set `merged` — but
waiting for NEXT run's Phase 2 to confirm costs a full cycle during which
already-merged PRs still count as `review` in `WIP_NOW` (throttling claims),
the trailer reads `N merge-attempts, 0 merged-now` (misleading), and
`Closes #<N>` issue-closing is delayed (observed 2026-07-07: 9 of 11 `review`
rows were already MERGED on GitHub). This sweep closes that lag inside the
same run.

For each row whose PR this run touched in Phase 5.5 (a merge subagent was
spawned OR the pre-flight classified it `ci-in-progress`/`ready`), do ONE
cheap re-poll at the end of the merge phases:

```bash
# MCP-primary, one call per row: mcp__github__get_pull_request → .state, .merged_at
```

- If GH reports `MERGED` → apply the SAME transition Phase 2 would:
  `status="merged"`, `merged_at=PR.mergedAt`, `status_changed_at=now`,
  `last_updated=now`; for `gh-issue-<N>` tasks, close issue `#<N>` now (the
  Phase 2.6 "closing the loop" rule) instead of next cycle.
- Anything else (`OPEN`, auto-merge still pending checks, `CLOSED`) → leave
  the row untouched; next run's Phase 2 remains authoritative for every
  non-MERGED outcome (especially CLOSED-without-merge → failed).

One poll per row, no retry, no wait loop — this is a snapshot, not a monitor.
Rows confirmed here count toward the trailer's `merged-now` token and free
their WIP slot for the Phase 6 accounting. Surface under Phase 7
`Merge-confirmed:`.

---

## Phase 6 — Persist & push

Update `assignments.generated = now`. Include `action-list.json` if Phase 2.6 Tier 1 refilled.

**Archive move (issue #9 — token spending).**

**Invariant** (T19 + T18b): post-Phase-6, `assignments.json` carries NO
row whose status is terminal (`merged` / `failed` / `done`); AND
`assignments-archive.json` contains AT MOST ONE row per `task_id`. Every
terminal row appears exactly once across the two files.

**Sweep semantics, not transition-set semantics.** Compute the move set
from the current file state, not from this run's transitions set. Earlier
revisions of this spec scoped the move to "rows whose status transitioned
in this run" — that is a bug: if a run sets `status=merged` but is killed
before the archive-move step (sandbox timeout, network blip, manual
interrupt), the row is orphaned in active forever, since subsequent runs
won't see it in *their* transitions set either. The sweep formulation is
idempotent and self-healing — every run cleans up any orphans left by
prior runs at zero extra cost.

The move is delegated to `.research/archive-reconcile.sh --apply`. The script
is the single source of truth for sweep semantics (sibling of
`.research/gc1-reconcile.sh`); it can also be invoked outside a dispatcher
run — see the **Merge-time reconcile** note below.

```bash
bash .research/archive-reconcile.sh --apply
```

The script does the equivalent of:

- Build the move set from current active file state (sweep — `status ∈
  {merged, failed, done}`).
- Append matching rows to `assignments-archive.json` and upsert by
  `task_id` (concurrent runs that each re-archive the same id used to
  produce duplicate archive rows — T4 FAIL: gap-82-4-*-impl + 3 others
  present twice; `group_by | map(last)` is the dedupe).
- Drop those rows from `assignments.json`.
- Stamp `archived_at = now` on the archive.
- T17 guard: refuse to write if combined active+archive count would
  change (move, not copy).

**Merge-time reconcile (issue #759).** Phase 6 only runs inside a
dispatcher run, so a stacked dispatcher PR that merges `origin/dev` into
its branch will pull dev's newer terminal rows for ids the branch still
has as `review` — the CI self-test then trips T19 before the next
dispatcher run gets a chance to sweep (observed on PR #734: two leaked
rows, `gap-82-5-ios-keychain-push-v2` and `gap-82-3-search-enhancements-fix`).
After any `git merge origin/dev` on a branch that carries
`.research/management/assignments.json`, run:

```bash
bash .research/archive-reconcile.sh           # dry-run; lists any leaks
bash .research/archive-reconcile.sh --apply   # move + commit alongside the merge
```

The script is idempotent and a no-op on a clean state.

The Phase 7 summary's `Merged-now` / `Failed (this run)` counters are still
derived from the in-memory transitions set — those are this-run-shaped
quantities, distinct from the file-state-shaped move set above.

**Goal-check (ENFORCING — PR 2).** Before committing, run the deterministic
goal checks with enforcement on. A hard failure (GC2 — coverage regression)
ABORTS the commit (the run bails; the next run re-evaluates once coverage is
corrected). GC1/GC3 remain record-only (`hard=false`). The `goal-check: …`
line is still recorded in the commit body on the pass path.

```bash
# PR 2: enforce. GC2 (coverage regression) hard-fails the run; GC1/GC3 stay
# record-only (hard=false). Capture the summary line either way.
GOAL_CHECK=$(GOAL_CHECK_ENFORCE=1 ./.research/goal-check.sh --json 2>/dev/null); GOAL_RC=$?
[ -z "$GOAL_CHECK" ] && GOAL_CHECK='[]'
GOAL_LINE=$(echo "$GOAL_CHECK" | jq -r 'map("\(.check)=\(if .passed then "ok" else "FAIL" end)") | join(" ")')
echo "goal-check: $GOAL_LINE (enforce rc=$GOAL_RC)"
if [ "$GOAL_RC" -ne 0 ]; then
  echo "ABORT: goal-check hard failure (coverage regression — GC2). Not committing." >&2
  exit 0   # bail without committing; next run re-evaluates once coverage is fixed
fi
```

Append `goal-check: <GOAL_LINE>` as a trailing line in the dispatcher commit
message body so convergence health is visible per-run in `git log`.

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

# Action-list archive-move reconcile (issue #1014 / #2102 ROOT-CAUSE fix).
# Mirror of the assignments split (archive-reconcile.sh keeps assignments.json
# small): sweep terminal action-list rows (done/dropped) into
# action-list-archive.json so action-list.json stays well under the 64 KiB MCP
# inline-push ceiling. An un-swept, bloated action-list.json is exactly what a
# Phase-6 MCP-push fallback truncates and silently corrupts on dev (#1014). The
# script is idempotent and combined-count-guarded (see its header) — a no-op on
# already-clean state, so it is safe to run unconditionally every Phase 6.
AL_ITEMS_BEFORE=$(jq '.items | length' .research/management/action-list.json 2>/dev/null || echo 0)
if ! bash .research/action-list-reconcile.sh --apply; then
  echo "PHASE 6: action-list-reconcile --apply failed (combined-count guard or jq error) — leaving action-list.json untouched and continuing; T26 self-test below will catch any residual terminal bloat." >&2
fi
AL_ITEMS_AFTER=$(jq '.items | length' .research/management/action-list.json 2>/dev/null || echo 0)
# Stage the reconciled pair only when rows actually moved (keeps the commit
# tight — no churn on a clean no-op run).
if [ "$AL_ITEMS_AFTER" != "$AL_ITEMS_BEFORE" ]; then
  git add .research/management/action-list.json \
          .research/management/action-list-archive.json
fi

git add .research/management/assignments.json \
        .research/management/assignments-archive.json \
        [.research/management/action-list.json if refilled or GC1 cascade closed rows] \
        [.research/management/gc1-orphan-triage.md if GC1 cascade wrote it] \
        [.research/management/post-merge-review.json .research/management/last-merged-review.txt if Phase 2.5 ran]
# State-write ownership (finding subagent-race-on-dev-push): the dispatcher is
# the single writer of .research/management/** for the phases IT owns, so when
# Phase 2.5 runs its post-merge artifacts are folded into THIS commit rather
# than pushed by ppt-review-merged itself (it now defers under
# DISPATCHER_OWNED_COMMIT=1). All paths are under .research/management/, so the
# commit-scope guard below still passes. (Phase 1.5/1.6 artifacts are owned by
# the routine, not the dispatcher — the routine's own Phase 6 `git add .research/`
# stages those; do not duplicate them here.)
# Commit-scope guard (#526): before the dispatcher's self-commit, refuse
# if `git diff --cached` strays outside `.research/management/`. Catches
# the PR #496 class of failure (stop hook bundling parallel-agent work
# into a doc-edit commit). Exit 2 = REFUSE — bail without committing.
bash .claude/skills/ppt-implement/scripts/commit-scope-guard.sh \
  --allow '.research/management/**' || {
  echo "dispatcher commit-scope-guard refused — staged paths outside .research/management/. NOT committing; surface in next run." >&2
  exit 0
}

# MCP-push size guard (issue #1014 / #2102 / #2126) — MUST run BEFORE `git commit`,
# while `git diff --cached` still reflects the staged push payload. FAIL CLOSED:
# hard-check every STAGED file against the 64 KiB MCP inline-push ceiling. A
# blocked push is recoverable (the next run retries on a fixed base); a silently-
# truncated MCP push is not. The structural fix is the action-list reconcile
# staged above; this guard is the belt-and-suspenders that catches any file still
# oversize (e.g. the reconcile was skipped or failed).
#
# REGRESSION #2126: this guard was previously invoked in the push branch AFTER
# `git commit`, where the index equals HEAD and `git diff --cached` returns an
# empty set — so `--staged` inspected nothing, the guard printed "no files to
# check — OK", exited 0 every run, and never saw the real push payload (dead
# code). It is now placed here, right after the final `git add`, so the check
# actually fires. The guard self-skips when PUSH_METHOD != mcp (direct `git push`
# has no inline-content limit), so it is safe to run unconditionally.
if ! PUSH_METHOD="${PUSH_METHOD:-mcp}" bash .research/mcp-push-size-guard.sh --staged; then
  echo "PHASE 6 ABORT: mcp-push-size-guard tripped — a staged file exceeds the MCP inline-push ceiling; refusing to commit/MCP-push (would truncate/corrupt it on dev, #1014). Remediation: re-run the reconcilers to shrink state, or land this run via 'PUSH_METHOD=git' where the proxy allows. Not marking this run successful." >&2
  exit 0
fi

# Pre-commit self-test gate. Phase 8 finding `self-test-fail-recurs-each-run`:
# T11 / T4 / T18 / T19 were enforced only by the post-hoc self-test, so every
# claim/archive cycle could introduce a fresh invariant violation that was
# then back-filled reactively. Running the self-test BEFORE commit converts
# it from a forensic ritual into a real gate. Abort on FAIL with a recovery
# hint so the operator (or the next run) can fix the state before publishing
# a known-broken snapshot to dev.
if ! bash .research/dispatcher-self-test.sh >/tmp/dispatcher-self-test.log 2>&1; then
  echo "PHASE 6 ABORT: dispatcher-self-test FAIL — refusing to commit state that violates invariants." >&2
  tail -25 /tmp/dispatcher-self-test.log >&2
  echo "" >&2
  echo "  Recovery options:" >&2
  echo "    1. Inspect /tmp/dispatcher-self-test.log to identify the failing T<N>." >&2
  echo "    2. If state is genuinely corrupt: 'git checkout HEAD -- .research/management/'" >&2
  echo "       and let the next run rebuild from a known-good base." >&2
  echo "    3. If the self-test itself is wrong: fix the test, not the data, in a follow-up PR." >&2
  echo "" >&2
  echo "  Skip-gate (operator-only): DISPATCHER_SELF_TEST_GATE=0 bypasses this check" >&2
  echo "  for a single run. Use only when you have manually verified state and need" >&2
  echo "  to publish despite a known-stale self-test invariant." >&2
  if [ "${DISPATCHER_SELF_TEST_GATE:-1}" != "0" ]; then
    exit 0
  fi
  echo "  DISPATCHER_SELF_TEST_GATE=0 — continuing despite self-test FAIL." >&2
fi

git commit -m 'chore(research): dispatcher <yyyy-mm-dd HH:MM> — C claimed, R reviewed, M merge-attempts, X merged-now, F failed, A active, B <claimable>/<open> dep-blocked=<n>, RB rebased, DS dup-skipped'
INTENDED_TREE=$(git rev-parse HEAD^{tree})   # the state tree we MUST land (rebase-ours guard)

# --- Push method (finding `git-push-blocked-by-proxy`) ---
# PRIMARY push path is the GitHub API (mcp__github__push_files), NOT `git push`:
# the local proxy HTTP-403s direct push in this environment, so `git push` burns a
# full retry/backoff loop before failing EVERY run (recurred R8). Default to MCP;
# fall back to git push only when the MCP tool is unavailable. `PUSH_METHOD=git`
# forces the legacy path for environments where direct push works.
if [ "${PUSH_METHOD:-mcp}" = "mcp" ]; then
  # NOTE (issue #2126): the MCP inline-push size guard already ran BEFORE
  # `git commit` above (right after the final `git add`), where
  # `git diff --cached` still reflected the staged payload. Do NOT re-run it
  # here with `--staged` — post-commit the index equals HEAD, so `git diff
  # --cached` is empty and the guard is a dead no-op (the exact bug #2126
  # fixed). The staged tree that guard vetted is the same tree this MCP push
  # emits, so the push payload is already known-safe.
  : # land the Phase 6 file delta via mcp__github__push_files onto dev (one commit).
    # A GITHUB_TOKEN/API push does not re-trigger version-bump on its own, which also
    # caps the rapid-bump churn (finding subagent-race-on-dev-push).
else
  # --- git push fallback, with rebase-ours conflict discipline ---
  git fetch origin dev --quiet
  if ! git push origin dev 2>/tmp/push.err; then
    # Origin moved since our pull → rebase our state commit onto it.
    # CONFLICT-RESOLUTION OWNERSHIP (finding `rebase-ours-takes-upstream-discards-state`):
    # in a REBASE, `--ours` is the UPSTREAM side (origin/dev) and `--theirs` is OUR
    # commit — the INVERSE of a merge. For files the dispatcher OWNS, ALWAYS keep
    # OUR side → `git checkout --theirs <file>`. Using `--ours` silently keeps
    # upstream and discards the dispatcher's state write; the next push then reports
    # "Everything up-to-date" with the work gone. Owned paths: everything under
    # `.research/management/**` plus `VERSION`.
    if ! git rebase origin/dev 2>/tmp/rebase.err; then
      for f in $(git diff --name-only --diff-filter=U); do
        case "$f" in
          .research/management/*|VERSION) git checkout --theirs -- "$f" && git add "$f" ;;
          *) echo "PHASE 6 ABORT: real (non-owned) conflict in $f — git rebase --abort; bail." >&2
             git rebase --abort; exit 0 ;;
        esac
      done
      git rebase --continue 2>>/tmp/rebase.err || { git rebase --abort; exit 0; }
    fi
    PUSH_OUT=$(git push origin dev 2>&1 || true)
    echo "$PUSH_OUT"
    # No-op-push detector (finding `rebase-ours-takes-upstream-discards-state`):
    # "Everything up-to-date" right after we committed owned-state changes is the
    # tell that the resolution dropped our delta. Do NOT exit 0 as success.
    if printf '%s' "$PUSH_OUT" | grep -qi 'Everything up-to-date'; then
      echo "PHASE 6 ABORT: push was a no-op after a state commit — owned-state delta lost in rebase. Recover from reflog ($INTENDED_TREE) and re-push; not marking this run successful." >&2
      exit 0
    fi
  fi
fi

# Post-push verification (defence-in-depth): confirm the landed origin/dev tree
# carries our owned-state changes; if not, surface loudly rather than silently pass.
git fetch origin dev --quiet
if git diff --quiet "$INTENDED_TREE" origin/dev -- .research/management/ 2>/dev/null; then
  : # owned-state delta present on origin/dev — OK
else
  echo "PHASE 6 WARN: origin/dev .research/management/ differs from the intended tree — verify the state landed (rebase-ours / concurrent-run check)." >&2
fi
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
Dup-skipped:              [<id> reason=<open-pr|open-assignment|file-overlap> conflicts_with=<#n|task_id>, …]   (cross-PR dedup guards; [] if none)
Target epic:              <epic_prefix open=N claimable=M action=<keep|select|repick|empty> | off>   (PR 3/5 finish-first; "off" when DISPATCHER_FINISH_FIRST≠1)
WIP throttle:             <wip=N/cap=M free_slots=K | disabled (cap=0)>   (PR 4/5; smooth back-pressure on Phase 3 claims)
Quarantined this run:     [<task_id> reason=<short>, …]               (PR 5/5; [] if none)
Quarantined total:        <N>   (PR 5/5; rows currently in status=quarantined across active assignments)
Pre-merge autofix:        [PR#<n> premerge=<applied|skipped|failed> <note>, …]  (PR 5/5 Phase 5.4; [] if none)
Transitions (this run):   [<id> in-progress→review, …]             ([] if none)
In-progress (global now): <N> total across all overlapping runs (no cap)
In review (PR open):      <M>
Merge attempts (this run):[PR#<n> merged=<true|false|queued> <note>, …]
CI-stuck escalations:     [PR#<n> task=<id> waited=<h>, …]                (gap 4; [] if none)
Approved+CI-pending:      [PR#<n> task=<id> attempted=<iso8601> wait=<h>, …]  (gap 4; [] if none)
Approved+human-gated:     [PR#<n> task=<id>, …]                     (Phase 5.5 needs-human-review SKIP; [] if none)
Awaiting-macOS-build:     [PR#<n> task=<id>, …]                     (Phase 5.5 macos-build-gated; structural — no macOS runner; NOT a merge failure; [] if none)
Mobile-native gated:      [<id> signal=<id|files>, …]               (Phase 3 issue #2652: KMP/mobile-native candidates skipped at claim time — AGP egress-blocked; structural, NOT a claim failure; parked not dropped; [] if none)
Rebase attempts (this run):[PR#<n> rebased=<true|false> <note>, …]  (item #6; [] if none)
Sandbox reclaims (this run):[<task_id> branch=<branch> reason=sandbox-timeout, …]  (P3; [] if none)
Empty branches deleted:   [<branch>, …]                             (item #1; [] if none)
Failed-dep cascades:      [<id> blocked-by=<dep_id>, …]             (issue #6; [] if none)
Unresolved-dep items:     [<id> dep="<truncated-legacy-text>", …]   (issue #583 — poisoned-sentinel rows whose legacy `dependency` text didn't parse; need human resolution; [] if none)
GC1 cascade:              <closed-leak=<N> orphan-triage=<M> | clean>   (gc1-reconcile; archive-terminal closes + coverage-missing orphans)
Backlog-reconciled:       <flipped=<N> [<id>→<done|dropped>, …] | clean>   (backlog-honesty pass; backlog.json vs assignment ledger)
Backlog-refill:           <promoted=<N> honest-claimable=<A>→<B> capped=<0|1> | clean (healthy) | none-available>   (Tier 1b backlog.json self-refill; planner-independent)
Retry-reminted:           <minted=<N> [<id> retry_of=<orig> round=<k>, …] | none-eligible | not-fired>   (Tier 1c failed-task retry re-mint)
Generator:                <spawned=<dev-review|review-merged> outcome=<short> | not-fired>   (Tier 1d demand-driven vector generation; fires only when claimable < floor after 1b/1c)
Merge-confirmed:          [PR#<n> task=<id>, …]   (Phase 5.8 same-run GH-truth flips to merged; [] if none)
Stale-review escalations: [<task_id> PR#<n> age=<d>d → issue #<m>, …]   (Phase 2 SLA: stale red reviews + quarantine exits; [] if none)
Issue-ingest:             <ingested=<N> [#<n>, …] capped=<0|1> | none-untracked | mcp-fetch-failed>   (open GH issues → action-list gh-issue-<N>; every cycle)
Issues-closed:            [#<n> resolved-by=PR#<m>, …]   (gh-issue-<N> assignments that merged this run and were closed via MCP; [] if none)
Aged claims (this run):   [<id> waited=<h>h <low→medium|medium→high>, …]   (Phase 3 anti-starvation aging boost; [] if none)
Run lock:                 <acquired <run_id> ttl=<m>m | stole-stale exp=<iso> | abort-held expires-in=<m>m>  (Phase 0.5)
Skip-gate:                <none | "recent-run age=<m>m; mutating phases SKIPPED">  (issue #1)
Tier 2 response:          <http=<code> body="<truncated>" | not-fired>          (issue #5)
Review dedup-skipped:     [PR#<n> existing-at=<iso>, …]             (issue #3; [] if none)
Scope-drift flagged:      [PR#<n> task=<id> note=<paths>, …]        (item #3; [] if none)
Code-reuse warnings:      [PR#<n> task=<id> note=<helper>, …]       (item #4; [] if none)
Disk warning:             <none | "free=N%; cleaned to M%">         (item #7)
Merged total: <Mt_total>; this cycle: <Mt_this>
Failed total: <F_total>;  this cycle: <F_this>
Buffer:     claimable=<open_claimable_count>/<BUFFER_TARGET> ceil=<BUFFER_CEIL> (open=<open_count>, dep_blocked=<dep_blocked_count>) <T0: drained -N | T1: refilled +N | T2: upstream kicked | OK>
Post-merge: <due (last <AGE_H>h ago | no-marker) scanned=N clean=K issues=M | skipped (last <AGE_H>h ago < 24h)>   (content-age gate, not mtime)
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

**Purpose:** identify systemic issues in the dispatcher pipeline and
propose concrete fixes. Two outputs:

1. A per-run report at `.research/self-improvement/<iso8601-utc>.md`
   (human-readable narrative — kept lightweight).
2. A structured findings backlog at
   `.research/self-improvement/findings.json` (the load-bearing output —
   each finding has a stable `finding_id`, recurrence count, proposed
   fix, and severity; this is what an operator triages, what the routine
   surfaces in the daily brief, and what a future automation could turn
   into auto-fix PRs).

Self-review is NOT a post-mortem narrator. Its job is to look across the
last N dispatcher commits + this run's Phase 7 counters + the structured
skip/failure logs (`Dup-skipped`, `Sandbox reclaims`, `Failed-dep
cascades`, `Empty branches`, `Scope-drift`, `Code-reuse`, `CI-stuck`,
`Hang alerts`) and answer:

- Which counters are non-zero **with recurrence ≥3** over the last 7
  days? (A single bad run is noise; a pattern is a bug.)
- For each recurring counter, what's the upstream cause? (Planner bug?
  Prompt ambiguity? Missing automation? Race condition?)
- What is the smallest spec/skill/code change that would prevent the
  recurrence? Name file:line where possible.

The routine never auto-applies findings — humans (or a future, separately
scoped, kill-switched auto-fix flow) act on them. But the findings ARE
machine-readable, so downstream tooling can consume them.

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
**`model=opus`** — pin to Opus 4.8 for sharper diff-of-runs reasoning;
worth the extra cost for a temporary instrumented phase). NOT parallel
with anything else — this is the last thing the run does.

**Spawn the self-review subagent** with the verbatim prompt at
`.research/self-improvement/reviewer-prompt.md`. Do NOT inline it here —
pass the Task subagent a one-line instruction to read that file and follow
it exactly, e.g.:

> Read `.research/self-improvement/reviewer-prompt.md` and follow it exactly.
> It is your complete instruction set; you are the dispatcher self-review
> agent. Return only the single `selfreview=ok …` status line specified there.

The subagent reads its rubric/schema/templates in its own context, so those
~185 lines never load into the dispatcher run. Keep the off-switch, spawn
mechanics, and return-line capture below inline — they are dispatcher-side.

Capture the return line into the Phase 7 summary stdout as one extra line:

```
Self-review: <selfreview=ok path=... findings_total=N findings_new=K findings_acknowledged=A notes=... | skipped: DISPATCHER_SELF_REVIEW=0 | skipped: SKIP_MUTATING>
```

When `findings_new > 0` OR `findings_acknowledged > 0` the routine's daily
brief surfaces the delta — see `routine-prompt.md` Brief template additions
under *Self-review findings*. Operators triage `findings.json` directly;
the dispatcher never auto-applies a fix.

This line is NOT in the regular Phase 7 list above because it depends on
this whole phase being optional. Treat it as a Phase 8 epilogue line.

**Cost note (TEMP_PHASE_8):** Opus 4.8 input is ~5× Sonnet. A typical
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

- **standing authorization — never block on a confirmation prompt** (finding `dispatcher-stalls-on-branch-rule-confirmation`, 2026-06-03): pushing to `dev` via MCP, opening draft `auto-impl/*` PRs, and opening issues are PRE-AUTHORIZED standing actions; generic session git rules ("session-branch only / never push elsewhere without permission") do NOT apply to the dispatcher. The lock + soft-gate are the ONLY run gates — when they clear, run every phase to completion. NEVER halt with a "how would you like me to proceed" A/B/C question for the standard mutating phases; surface novel conflicts in Phase 7 instead of blocking on them.
- per-run cap: claim at most `DISPATCHER_CLAIM_CAP` (default 6) NEW tasks AND spawn at most `DISPATCHER_CLAIM_CAP` implementer subagents
- **WIP throttle (PR 4/5)**: `free_slots = min(DISPATCHER_CLAIM_CAP, max(0, DISPATCHER_WIP_CAP - WIP_NOW))`. Defaults `DISPATCHER_CLAIM_CAP=6`, `DISPATCHER_WIP_CAP=16` (raised from 8→12→16, 2026-06-02). Set `DISPATCHER_WIP_CAP=0` to disable the WIP throttle. Counts rows in `{in-progress, review}`. Smooth back-pressure: claims trickle until exactly at cap, then 0 until merges drain.
- per-run **per-epic** cap of 3 (`DISPATCHER_SAME_EPIC_CAP`, item #2). Lifted when `DISPATCHER_FINISH_FIRST=1` AND a target epic is selected (PR 3/5).
- state transitions are MANDATORY; `merged` / `failed` are TERMINAL
- legacy `status == "done"` is equivalent to `merged` (counting only); never auto-migrate or rewrite those rows
- `status_changed_at` bumped ONLY on actual status value change
- max 1 post-merge reviewer subagent per run
- max 2 pr-merge subagents in parallel in Phase 5.5
- max 1 rebase subagent in parallel in Phase 5.6 (item #6)
- max `DISPATCHER_CLAIM_CAP` (default 6) followup/respawn subagents in parallel in Phase 5.7 (matches Phase 4 implementer cap)
- no cap on reviewer (Phase 5) subagents
- never re-claim an id already in assignments (regardless of its status)
- never claim items whose `depends_on: [task_id, …]` array contains any task whose `assignments.json` row is not in `{merged, done}` (gap 3 — structured field replaces free-text `dependency` parsing)
- **GitHub I/O is MCP-primary** (see *GitHub I/O contract* at the top): every GitHub network call uses `mcp__github__*` first; `gh` CLI / `git fetch/push` are 403-unreliable in this env and are fallback-only. The sole sanctioned `gh` use is the Phase 0.5 atomic ref-CAS lock (`gh api POST /git/refs`), which has no MCP equivalent.
- never push to `main`
- never bypass git hooks (no `--no-verify`)
- never set `assignment.status="merged"` inside Phase 5.5 — only Phase 2 and the Phase 5.8 merge-confirm sweep set `merged`, and BOTH only from a fresh GH `MERGED` read (never from `gh pr merge`'s own exit status)
- Tier 2 upstream kick is fire-and-forget with `--max-time 10`
- always defer to `ppt-implement`, `ppt-review-merged`, `ppt-pr-merge`, `ppt-pr-followup` skills
- buffer guard adds items only; never edits/removes existing
- do NOT inline implementer / reviewer / merger logic
- Phase 2's `<2h grace, no branch yet` case respects that another run's implementer may still be working on this task — don't fail prematurely
- **empty-branch detection** (item #1) is deterministic: `git rev-list --count origin/dev..origin/<branch> == 0` → fail row immediately, delete branch
- **scope-drift** and **code-reuse-warn** (items #3, #4) are non-blocking on implementer return, but feed the reviewer prompt and are surfaced in Phase 7
- **reviewer re-runs** (item #10) gate on `PR.headRefOid != row.last_reviewed_oid` — never re-review the same SHA
- **auto-rebase** (item #6) is bounded at 3 attempts per row; after that a human must intervene
- **retry re-mint** (Tier 1c) is bounded: `RETRY_MAX_ROUNDS` (default 2) lifetime rounds per stem, cool-down `RETRY_COOLDOWN_DAYS` (default 7), only for failures with `pr_number=null`; the `retry_of` field is written ONLY by `retry-remint.sh`
- **escalation SLA** (Phase 2) is bounded at 2 escalations per run, oldest first; it never deletes a branch, and every escalation leaves a PR comment + a `from-stalled-pr` issue (nothing disappears silently)
- **Tier 1d generator** spawns at most 1 subagent per run, only when `claimable < BUFFER_FLOOR` after Tiers 1b/1c, and runs under `DISPATCHER_OWNED_COMMIT=1` (single-writer rule)
- **sandbox-reclaim** (P3) is bounded at 1 attempt per row; the helper at `.claude/skills/ppt-pr-followup/scripts/sandbox-reclaim.sh` picks the timeout (60m for `Mode: cloud-ok`, 120m otherwise) and classifies the row as wait/reclaim/fail. Reclaim re-spawns the same specialist with the same brief and bumps `reclaim_attempts`; a second sandbox-timeout becomes `failed` with `reason: sandbox-failure-after-reclaim`
- **disk preflight** (item #7) aborts the run gracefully at <5% free; never crashes mid-subagent
- **recent-run skip-gate** (issue #1) — if `assignments.generated` is < 25min old, set `DISPATCHER_SKIP_MUTATING=1` and SKIP every mutating phase (2.5, 2.6, 2.7, 3, 4, 5, 5.5, 5.6, 5.7, 5.8, and the Phase 2 escalation SLA). Phase 2 (GH reconciliation) and Phase 7 (summary) still run. Prevents the `assignments.json` rebase races seen on 2026-05-27.
- **failed-dep cascade** (issue #6) — open action-list items whose `depends_on` points at a terminal-`failed` row are dropped (`status=open → status=dropped`) in Phase 2.7 with an audit prefix; max 20 cascades/run. Re-planning is upstream (operator-driven).
- **Tier 2 kick logging** (issue #5) — capture HTTP code + first 200 chars of response body; surface in commit message so a broken/wedged planner endpoint is visible without trawling trigger history.
- **reviewer dedup guard** (issue #3) — reviewer subagent MUST `GET /pulls/<n>/reviews` first; if a bot review for the current `headRefOid` already exists within 2h, skip posting and return `note=dedup-existing-review-at-<iso>`. Defense-in-depth against the skip-gate window-edge case.
- **subagent workspace isolation** (issue #7) — every Phase 4 (implementer), Phase 5.6 (rebaser), and Phase 5.7 (followup-respawn implementer) subagent MUST run its `git checkout` / `gh pr checkout` / build / commit work inside `/tmp/ppt-worktrees/<task_id>/` via the standard `git worktree add` preamble. NEVER touch the dispatcher's own working tree. Phase 5.5 (merger, API-only) and Phase 2 (read-only reconciliation) are exempt.
- **state-write ownership / single writer** (finding `subagent-race-on-dev-push`) — `.research/**` on `dev` is written by the ORCHESTRATOR process only, never by a spawned skill. The dispatcher owns the phases it runs (the Phase 3.5 claim commit + the Phase 6 main commit); the research routine owns its own Phase 1.5/1.6 outputs via its single `git add .research/` at the routine's Phase 6. NO spawned subagent may `git add/commit/push` any `.research/**` file: analysis skills (dev-review, project-management, post-merge-review) WRITE their artifacts into the working tree and RETURN, and the owning orchestrator folds them into its one commit. Implementers/rebasers/followups commit only on their own feature branch inside their worktree; reviewers/mergers act only via the GitHub API on the PR. Each independent `.research/` push to `dev` is a separate research-land replay + version-bump and can empty the orchestrator's commit by pre-empting its delta — see the contract under "Subagent execution model". The orchestrator sets `DISPATCHER_OWNED_COMMIT=1` in the env of every analysis subagent it spawns; those skills gate their own commit/push on it. (`ppt-project-management` and `ppt-dev-review` already return without committing; `ppt-review-merged` is the one that self-pushed and is fixed here.)
- **TEMP_PHASE_8 — self-review** — Phase 8 spawns an Opus subagent that writes a post-mortem markdown to `.research/self-improvement/<iso8601>.md`. Off-switch: `DISPATCHER_SELF_REVIEW=0`. The subagent must NOT modify any state file or commit anything. **This phase is temporary** — remove by 2026-06-30 (search `TEMP_PHASE_8` to find every related artifact).
- **goal-check (ENFORCING, PR 2)** — Phase 6 runs `GOAL_CHECK_ENFORCE=1 .research/goal-check.sh`. GC2 (coverage regression) ABORTS the commit; GC1/GC3 are recorded but non-blocking. The `goal-check:` summary line is still written to the commit body.

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
