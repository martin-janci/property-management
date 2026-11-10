#!/usr/bin/env bash
# retry-remint.sh — give retryable FAILED tasks a bounded second life.
#
# Why this script exists (failed-task dead-end, 2026-07-07):
#
#   A `failed` row in assignments-archive.json permanently blocks its task_id
#   AND its stem: Phase 3's claim predicate rejects both (issue #1739 #3), and
#   backlog-refill.sh applies the same exclusion at promotion time. That is
#   correct as a duplicate-work guard, but it also means a task that failed
#   for a TRANSIENT reason — implementer died before even opening a PR
#   (pr_number=null: sandbox timeout, verify-env failure, disk pressure) —
#   can never be attempted again. Observed 2026-07-07: ~20 scored backlog
#   vectors (several score-3 `ready` bugs) dead-ended this way while the
#   buffer starved at honest_claimable=0.
#
#   dispatcher-prompt.md always intended a path out: "A failed task that
#   should be retried must use a suffixed id (e.g. `<id>-retry`), per the
#   stem/suffix convention" (Phase 3). This script IS that path, made
#   deterministic and bounded.
#
# What it does (idempotent, append-only, bounded):
#
#   Groups FAILED archive rows by stem() and re-mints an action-list row
#   `<stem>-retry<N>` when ALL of:
#     - the failure is classified retryable: `pr_number == null`
#       (died before a PR existed — infra, not a rejected implementation);
#     - cool-down passed: newest failure in the stem group is older than
#       RETRY_COOLDOWN_DAYS (default 7);
#     - retry budget left: < RETRY_MAX_ROUNDS (default 2) prior attempts
#       (rows in the stem group beyond the original);
#     - nothing in the stem group ever merged/done (work didn't land);
#     - no NON-TERMINAL action-list row and no active assignment
#       (in-progress/review/quarantined) shares the stem (T24 + PR 5/5);
#     - the minted id itself exists nowhere in action-list or assignments;
#     - dev-truth guards (issue #2153 — ghost retries): the stem's story is
#       NOT status="done" in coverage.json (work landed on dev under a
#       DIFFERENT task/PR), and no commit subject on origin/dev starts with
#       an anchored "<id>:" prefix that stems to the candidate stem (same
#       join convention as dev-reconcile.sh). Both degrade gracefully when
#       their input is missing (no exclusion).
#     - archive-marker guard (issue #2460 — out-of-band ghost blind spot):
#       the stem's LATEST action-list-archive row carries NO done/ghost marker
#       (`[RECONCILED-DONE`, `[DROPPED`, `[already-satisfied`, `[CASCADED-DROP`,
#       `[GHOST-DROP`, `[GHOST-RETRY-DROP`, `[BAD-RETRY-REMINT-DROP`). Those are
#       written by gc1-reconcile/dev-reconcile/cascade EXACTLY when the work
#       landed under a different id, was already satisfied, or is non-completable
#       — the signal the coverage/dev-log guards (which key on the task's OWN id)
#       structurally miss. Degrades gracefully when the archive is absent.
#     - already-landed summary guard (issue #2743 — cross-issue-id ghost, a
#       recurrence-hardening of #2460): NONE of the stem's failed rows carry an
#       `implementer_summary` matching an "already-landed" signal — `already
#       fixed`/`already landed`/`already satisfied`, `empty diff`, or `no PR
#       needed`. That summary is the escape hatch the ORIGINAL implementer
#       emits when it finds the work already merged on dev under a DIFFERENT
#       issue/PR id (e.g. gh-issue-2658/#2660 satisfying a `voice-webhook`
#       stem). The coverage/dev-log/subject-prefix guards all join on an id and
#       structurally cannot see a cross-id landing; this intrinsic-row signal
#       can. Degrades gracefully when the field is absent (no exclusion).
#
#   Minted rows carry `retry_of` (the original failed task_id) and
#   `retry_round`. `retry_of` is the field Phase 3 and backlog-refill.sh use
#   to EXEMPT the row from the stem-aware archive exclusion (the exact-id
#   exclusion still applies — that's what the -retryN suffix satisfies).
#   Bounded: ≤ RETRY_REMINT_CAP (default 6) per run, never past BUFFER_CEIL
#   headroom, fail-closed on count mismatch (same contract as backlog-refill).
#
# Usage:
#   ./.research/retry-remint.sh            # dry-run: print re-mint set
#   ./.research/retry-remint.sh --apply    # append rows to action-list.json
#   Injectables (tests): ACTION_LIST, ASSIGN, ASSIGN_ARCHIVE, ACTION_ARCHIVE,
#     COVERAGE_FILE, DEV_LOG_FILE (one commit subject per line; default is
#     `git log origin/dev --format=%s -300`), RETRY_NOW, RETRY_COOLDOWN_DAYS,
#     RETRY_MAX_ROUNDS, RETRY_REMINT_CAP, BUFFER_CEIL

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ACTION_LIST="${ACTION_LIST:-$REPO_ROOT/.research/management/action-list.json}"
ACTION_ARCHIVE="${ACTION_ARCHIVE:-$REPO_ROOT/.research/management/action-list-archive.json}"
ASSIGN="${ASSIGN:-$REPO_ROOT/.research/management/assignments.json}"
ASSIGN_ARCHIVE="${ASSIGN_ARCHIVE:-$REPO_ROOT/.research/management/assignments-archive.json}"
COVERAGE_FILE="${COVERAGE_FILE:-$REPO_ROOT/.research/management/coverage.json}"
DEV_LOG_MAX="${DEV_LOG_MAX:-300}"
NOW="${RETRY_NOW:-$(date -u +%Y-%m-%dT%H:%M:%SZ)}"
COOLDOWN_DAYS="${RETRY_COOLDOWN_DAYS:-7}"
MAX_ROUNDS="${RETRY_MAX_ROUNDS:-2}"
CAP="${RETRY_REMINT_CAP:-6}"
BUFFER_CEIL="${BUFFER_CEIL:-120}"
APPLY=0
[ "${1:-}" = "--apply" ] && APPLY=1

if [ ! -f "$ACTION_LIST" ]; then
  echo "retry-remint: action-list missing — nothing to do: $ACTION_LIST" >&2
  exit 0
fi

ASSIGN_INPUTS=()
[ -f "$ASSIGN" ]         && ASSIGN_INPUTS+=("$ASSIGN")
[ -f "$ASSIGN_ARCHIVE" ] && ASSIGN_INPUTS+=("$ASSIGN_ARCHIVE")
if [ "${#ASSIGN_INPUTS[@]}" -eq 0 ]; then
  echo "retry-remint: no assignment files — no failures to retry"
  exit 0
fi

# Scratch dir — large intermediates go through FILES, never argv:
# `--argjson` with a multi-100KB ledger blows ARG_MAX ("Argument list too long").
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

# All assignment rows (active + archive), flat.
jq -s '[ .[].assignments[]? ]' "${ASSIGN_INPUTS[@]}" > "$WORK/assign.json"

# Optional original-row metadata (action text / priority / owner_role) from
# the action-list archive; missing file degrades to sensible defaults.
if [ -f "$ACTION_ARCHIVE" ]; then
  jq '[ .items[]? ]' "$ACTION_ARCHIVE" > "$WORK/arch-items.json"
else
  echo '[]' > "$WORK/arch-items.json"
fi

# Dev-truth guard (a) — coverage-done (issue #2153): story ids already "done"
# in coverage.json. The archive ledger misses work that landed on dev under a
# DIFFERENT task_id/branch; coverage.json tracks dev truth per story. Optional
# file; missing/unparseable degrades gracefully to "no exclusion".
if [ -f "$COVERAGE_FILE" ]; then
  jq '[ .stories[]? | select(.status=="done") | .id ]' "$COVERAGE_FILE" \
    > "$WORK/cov-done.json" 2>/dev/null || echo '[]' > "$WORK/cov-done.json"
else
  echo '[]' > "$WORK/cov-done.json"
fi

# Dev-truth guard (b) — dev-log subject join (issue #2153): a commit on
# origin/dev whose SUBJECT begins with an anchored "<id>:" prefix (up to the
# first colon, no spaces — the same squash-merge join convention as
# dev-reconcile.sh, never a loose body grep) proves the stem's work landed
# even when no archive row recorded it. DEV_LOG_FILE (one subject per line)
# is injectable for offline tests; an unreachable remote degrades to
# "nothing landed" rather than aborting the dispatcher.
if [ -n "${DEV_LOG_FILE:-}" ]; then
  DEV_SUBJECTS=$(cat "$DEV_LOG_FILE" 2>/dev/null || true)
else
  DEV_SUBJECTS=$(git -C "$REPO_ROOT" log origin/dev --format=%s -"$DEV_LOG_MAX" 2>/dev/null || true)
fi
printf '%s\n' "$DEV_SUBJECTS" | jq -R . | jq -s . > "$WORK/dev-subjects.json"

# Headroom: never lift the open pool past BUFFER_CEIL.
OPEN_NOW=$(jq '[ .items[] | select(.status=="open") ] | length' "$ACTION_LIST")
HEADROOM=$(( BUFFER_CEIL - OPEN_NOW ))
if [ "$HEADROOM" -le 0 ]; then
  echo "retry-remint: open pool $OPEN_NOW >= ceil $BUFFER_CEIL — no headroom"
  exit 0
fi
[ "$CAP" -gt "$HEADROOM" ] && CAP="$HEADROOM"

CANDIDATES=$(jq -n \
  --slurpfile assign_f "$WORK/assign.json" \
  --slurpfile arch_f "$WORK/arch-items.json" \
  --slurpfile cov_done_f "$WORK/cov-done.json" \
  --slurpfile devlog_f "$WORK/dev-subjects.json" \
  --slurpfile al "$ACTION_LIST" \
  --arg now "$NOW" \
  --argjson cooldown_days "$COOLDOWN_DAYS" \
  --argjson max_rounds "$MAX_ROUNDS" '
  $assign_f[0] as $assign
  | $arch_f[0] as $arch_items
  | def stem: sub("-(impl|fix|v2|retry|followup|wip)[0-9]*$";"");
  def epoch: sub("[.][0-9]+Z$";"Z") | fromdateiso8601;
  ($now | epoch) as $now_e
  | ($cooldown_days * 86400) as $cooldown_s

  # Stems with any landed work — never retry those.
  | ([ $assign[] | select(.status=="merged" or .status=="done") | (.task_id | stem) ] | unique) as $landed_stems
  # Stems with an active (non-terminal) assignment — in flight, hands off.
  | ([ $assign[] | select(.status=="in-progress" or .status=="review" or .status=="quarantined") | (.task_id | stem) ] | unique) as $active_stems
  # Stems with a live (non-terminal) action-list row — T24 one-open-per-stem.
  | ([ $al[0].items[] | select(.status != "done" and .status != "dropped") | (.id | stem) ] | unique) as $al_live_stems
  # Every id already known anywhere (exact-id idempotency for the minted id).
  | ([ $assign[].task_id ] + [ $al[0].items[].id ] | unique) as $known_ids
  # Dev-truth (a): coverage.json stories already done — work landed on dev
  # under a different task/PR (issue #2153 ghost retries).
  | ($cov_done_f[0] | map(stem) | unique) as $cov_done_stems
  # Dev-truth (b): anchored "<id>:" subject prefixes on origin/dev, stemmed —
  # same join convention as dev-reconcile.sh (prefix must contain no spaces,
  # so a chore commit merely MENTIONING an id never matches).
  | ([ $devlog_f[0][] | capture("^(?<p>[^:\\s]+):") | .p | stem ] | unique) as $dev_landed_stems
  # Archive-marker guard (issue #2460): stems whose LATEST action-list-archive
  # row carries a done/ghost marker written by reconcile/cascade. These prove
  # the work landed under a DIFFERENT id, was already satisfied, or is
  # non-completable — the blind spot the coverage/dev-log guards (which key on
  # the failed task id itself) structurally miss. Latest = max last_updated (falls back
  # to first_open_at). Missing archive => empty array => no exclusion.
  | ([ $arch_items[]
       | select(.id != null)
       | { stem: (.id | stem),
           ts: (.last_updated // .first_open_at // "1970-01-01T00:00:00Z"),
           marked: (((.action // "")
             | test("\\[(RECONCILED-DONE|DROPPED|already-satisfied|CASCADED-DROP|GHOST-DROP|GHOST-RETRY-DROP|BAD-RETRY-REMINT-DROP)"))) }
     ]
     | group_by(.stem)
     | map(sort_by(.ts) | last)              # newest archive row per stem
     | map(select(.marked) | .stem)
     | unique) as $ghost_marked_stems

  | ([ $assign[] | select(.status=="failed") ]
     | group_by(.task_id | stem)
     | map(
         (.[0].task_id | stem) as $s
         | (map(.last_updated // .status_changed_at // .claimed_at // "1970-01-01T00:00:00Z") | max) as $newest
         | {
             stem: $s,
             rounds_used: (length - 1),          # attempts beyond the original
             newest_failure: $newest,
             retryable: (all(.[]; .pr_number == null)),
             # Already-landed guard (issue #2743): a failed row whose own
             # implementer_summary reports the work already merged on dev under a
             # DIFFERENT issue/PR id. An intrinsic-row signal (no id join), so it
             # catches the cross-issue-id ghost the coverage/dev-log/subject
             # guards structurally miss.
             already_landed: (any(.[];
               ((.implementer_summary // "")
                | test("already[ ._-]?(fixed|landed|satisfied)|empty[ ._-]?diff|no[ ._-]?pr[ ._-]?needed"; "i"))
             )),
             original_id: (map(.task_id) | sort | .[0])
           }
       )
     | map(select(
         .retryable
         and (.rounds_used < $max_rounds)
         and ((.stem | IN($landed_stems[])) | not)
         and ((.stem | IN($active_stems[])) | not)
         and ((.stem | IN($al_live_stems[])) | not)
         and ((.stem | IN($cov_done_stems[])) | not)
         and ((.stem | IN($dev_landed_stems[])) | not)
         and ((.stem | IN($ghost_marked_stems[])) | not)
         and ((.already_landed) | not)
         and (($now_e - (.newest_failure | epoch)) >= $cooldown_s)
       ))
     | map(. + { mint_id: (.stem + "-retry" + ((.rounds_used + 1) | tostring)) })
     | map(select((.mint_id | IN($known_ids[])) | not))
     | sort_by([.newest_failure, .stem])         # oldest failure first, deterministic
    ) as $groups

  | $groups
  | map(
      . as $g
      | ([ $arch_items[] | select(.id == $g.original_id) ] | .[0] // null) as $orig
      | {
          id:            $g.mint_id,
          action:        (($orig.action // $g.original_id) + " [retry \($g.rounds_used + 1)/\($max_rounds) of failed \($g.original_id)]"),
          owner_role:    ($orig.owner_role // "pm-tech-lead"),
          priority:      ($orig.priority // "medium"),
          status:        "open",
          source:        ("dispatcher-retry-remint " + $now
                          + " (retry_of=" + $g.original_id
                          + " reason=failed-no-pr cooldown_ok newest_failure=" + $g.newest_failure + ")"),
          deadline:      null,
          dependency:    null,
          depends_on:    [],
          first_open_at: $now,
          retry_of:      $g.original_id,
          retry_round:   ($g.rounds_used + 1)
        }
    )')

AVAIL=$(echo "$CANDIDATES" | jq 'length')
MINT_N=$(( CAP < AVAIL ? CAP : AVAIL ))

if [ "$MINT_N" -le 0 ]; then
  echo "retry-remint: no eligible failed tasks (avail=$AVAIL cap=$CAP) — nothing to re-mint"
  exit 0
fi

PROMOTE=$(echo "$CANDIDATES" | jq --argjson n "$MINT_N" '.[0:$n]')
echo "retry-remint: re-minting $MINT_N of $AVAIL eligible (cap=$CAP headroom=$HEADROOM):"
echo "$PROMOTE" | jq -r '.[] | "  \(.id)  (retry_of=\(.retry_of) round=\(.retry_round) pri=\(.priority))"'

if [ "$APPLY" != "1" ]; then
  echo "retry-remint: dry-run — pass --apply to write"
  exit 0
fi

COUNT_BEFORE=$(jq '.items | length' "$ACTION_LIST")
RESULT=$(jq --argjson add "$PROMOTE" '.items += $add' "$ACTION_LIST")
COUNT_AFTER=$(echo "$RESULT" | jq '.items | length')
if [ "$COUNT_AFTER" -ne $(( COUNT_BEFORE + MINT_N )) ]; then
  echo "retry-remint: ABORT — item count $COUNT_BEFORE + $MINT_N != $COUNT_AFTER" >&2
  exit 1
fi

TMP=$(mktemp)
echo "$RESULT" > "$TMP"
mv "$TMP" "$ACTION_LIST"
echo "retry-remint: appended $MINT_N row(s); action-list items $COUNT_BEFORE -> $COUNT_AFTER"
