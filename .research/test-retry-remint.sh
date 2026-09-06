#!/usr/bin/env bash
# Smoke test for retry-remint.sh — deterministic, offline.
# Exercises: retryable classification (pr_number=null only), cool-down gate,
# retry-round cap, landed-stem exclusion, active-stem exclusion, live
# action-list stem exclusion (T24), minted-id idempotency, retry_of/round
# stamping, per-run cap + ceil headroom, append-only count guard, the
# issue #2153 dev-truth guards (coverage-done exclusion + anchored dev-log
# subject exclusion, both degrading gracefully when their input is missing),
# and the issue #2460 archive-marker guard (skip a stem whose LATEST
# action-list-archive row carries a done/ghost marker — latest-row semantics
# and graceful degradation when the archive is absent), and the issue #2743
# already-landed summary guard (skip a stem whose failed row's own
# implementer_summary reports the work already merged on dev under a DIFFERENT
# issue/PR id — an intrinsic-row signal that catches the cross-issue-id ghost
# the id-join guards structurally miss; a benign summary must NOT exclude).
set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
SCRIPT="$HERE/retry-remint.sh"
FAIL=0
ok()   { printf '  ok    %s\n' "$1"; }
bad()  { printf '  FAIL  %s\n' "$1"; FAIL=1; }

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

AL="$TMP/action-list.json"
ALA="$TMP/action-list-archive.json"
ASG="$TMP/assignments.json"
ASGA="$TMP/assignments-archive.json"
COV="$TMP/coverage.json"
DEVLOG="$TMP/dev-log.txt"
NOW="2026-07-07T12:00:00Z"          # fixtures fail at 2026-06-20 → 17d ago

seed() {
  cat > "$AL" <<'EOF'
{ "version": 1, "items": [
  { "id": "unrelated-open", "action": "x", "status": "open", "priority": "low", "depends_on": [] },
  { "id": "live-stem-task", "action": "y", "status": "in-progress", "priority": "low", "depends_on": [] }
] }
EOF
  cat > "$ALA" <<'EOF'
{ "version": 1, "items": [
  { "id": "bug-retryable", "action": "Fix the retryable bug", "owner_role": "pm-backend", "priority": "high", "status": "dropped" },
  { "id": "bug-ghost-marked", "action": "[RECONCILED-DONE: fix landed on dev under a DIFFERENT story id] Fix the ghost-marked bug", "owner_role": "pm-backend", "priority": "high", "status": "done", "last_updated": "2026-06-20T00:00:00Z" }
] }
EOF
  cat > "$ASG" <<'EOF'
{ "assignments": [
  { "task_id": "active-stem-task", "status": "review", "pr_number": 5,
    "last_updated": "2026-06-20T00:00:00Z" }
] }
EOF
  cat > "$ASGA" <<'EOF'
{ "assignments": [
  { "task_id": "bug-retryable",       "status": "failed", "pr_number": null,
    "last_updated": "2026-06-20T00:00:00Z",
    "implementer_summary": "implementer died before opening a PR: sandbox timeout; retry safe" },
  { "task_id": "bug-had-pr",          "status": "failed", "pr_number": 42,
    "last_updated": "2026-06-20T00:00:00Z" },
  { "task_id": "bug-too-fresh",       "status": "failed", "pr_number": null,
    "last_updated": "2026-07-06T00:00:00Z" },
  { "task_id": "bug-landed",          "status": "failed", "pr_number": null,
    "last_updated": "2026-06-20T00:00:00Z" },
  { "task_id": "bug-landed-retry1",   "status": "merged", "pr_number": 43,
    "last_updated": "2026-06-21T00:00:00Z" },
  { "task_id": "bug-exhausted",        "status": "failed", "pr_number": null,
    "last_updated": "2026-06-01T00:00:00Z" },
  { "task_id": "bug-exhausted-retry1", "status": "failed", "pr_number": null,
    "last_updated": "2026-06-08T00:00:00Z" },
  { "task_id": "bug-exhausted-retry2", "status": "failed", "pr_number": null,
    "last_updated": "2026-06-15T00:00:00Z" },
  { "task_id": "active-stem-task-v2", "status": "failed", "pr_number": null,
    "last_updated": "2026-06-20T00:00:00Z" },
  { "task_id": "live-stem-task-v2",   "status": "failed", "pr_number": null,
    "last_updated": "2026-06-20T00:00:00Z" },
  { "task_id": "bug-second-round",        "status": "failed", "pr_number": null,
    "last_updated": "2026-06-10T00:00:00Z" },
  { "task_id": "bug-second-round-retry1", "status": "failed", "pr_number": null,
    "last_updated": "2026-06-15T00:00:00Z" },
  { "task_id": "bug-cov-done",        "status": "failed", "pr_number": null,
    "last_updated": "2026-06-20T00:00:00Z" },
  { "task_id": "bug-on-dev",          "status": "failed", "pr_number": null,
    "last_updated": "2026-06-20T00:00:00Z" },
  { "task_id": "bug-ghost-marked",    "status": "failed", "pr_number": null,
    "last_updated": "2026-06-20T00:00:00Z" },
  { "task_id": "bug-already-landed",  "status": "failed", "pr_number": null,
    "last_updated": "2026-06-20T00:00:00Z",
    "implementer_summary": "already-fixed on dev by fdeca2a (gh-issue-2658/#2660) under a different id — empty diff, no PR needed" }
] }
EOF
  # Dev-truth fixtures (issue #2153): a coverage story flipped "done" out of
  # band, and a dev commit whose SUBJECT starts with an anchored "<id>:".
  cat > "$COV" <<'EOF'
{ "stories": [
  { "id": "bug-cov-done",  "status": "done" },
  { "id": "bug-retryable", "status": "partial" }
] }
EOF
  cat > "$DEVLOG" <<'EOF'
bug-on-dev: fix landed out-of-band on dev (#1676)
fix(reality-web): unrelated conventional-commit subject
dispatcher notes bug-second-round: loose mention must NOT match (space before colon)
EOF
}

run() {
  ACTION_LIST="$AL" ACTION_ARCHIVE="$ALA" ASSIGN="$ASG" ASSIGN_ARCHIVE="$ASGA" \
  COVERAGE_FILE="$COV" DEV_LOG_FILE="$DEVLOG" \
  RETRY_NOW="$NOW" bash "$SCRIPT" "$@"
}

echo "T1 dry-run: eligibility filter"
seed
OUT=$(run)
grep -q "bug-retryable-retry1"    <<<"$OUT" && ok "retryable no-PR failure minted" || bad "bug-retryable missing: $OUT"
grep -q "bug-second-round-retry2" <<<"$OUT" && ok "second round minted as -retry2" || bad "bug-second-round-retry2 missing"
grep -q "bug-had-pr"       <<<"$OUT" && bad "failure WITH a PR must not be retried" || ok "pr_number!=null excluded"
grep -q "bug-too-fresh"    <<<"$OUT" && bad "fresh failure must wait out cool-down" || ok "cool-down enforced"
grep -q "bug-landed"       <<<"$OUT" && bad "landed stem must not re-mint" || ok "merged-in-stem excluded"
grep -q "bug-exhausted"    <<<"$OUT" && bad "exhausted retry budget must stop" || ok "retry cap (2 rounds) enforced"
grep -q "active-stem-task" <<<"$OUT" && bad "active assignment stem must block" || ok "active-stem excluded"
grep -q "live-stem-task"   <<<"$OUT" && bad "live action-list stem must block (T24)" || ok "live action-list stem excluded"
grep -q "bug-cov-done"     <<<"$OUT" && bad "coverage-done story must not re-mint (#2153)" || ok "coverage-done stem excluded"
grep -q "bug-on-dev"       <<<"$OUT" && bad "anchored dev-log subject must block (#2153)" || ok "dev-log subject-prefix excluded"
grep -q "bug-ghost-marked" <<<"$OUT" && bad "archive-marker stem must not re-mint (#2460)" || ok "archive-marker stem excluded"
grep -q "bug-already-landed" <<<"$OUT" && bad "already-landed implementer_summary must not re-mint (#2743)" || ok "already-landed summary stem excluded"
grep -q "bug-retryable-retry1" <<<"$OUT" && ok "benign (non-landed) implementer_summary does NOT exclude (#2743)" || bad "benign summary wrongly excluded bug-retryable"
grep -q "bug-second-round-retry2" <<<"$OUT" && ok "loose (non-anchored) dev-log mention does NOT exclude" || bad "non-anchored mention wrongly excluded bug-second-round"
grep -q "bug-retryable-retry1"    <<<"$OUT" && ok "coverage status=partial does NOT exclude" || bad "coverage partial wrongly excluded bug-retryable"
[ "$(jq '.items | length' "$AL")" = "2" ] && ok "dry-run leaves file untouched" || bad "dry-run mutated action-list"

echo "T2 apply: row shape + metadata inheritance + append-only"
run --apply >/dev/null
ROW=$(jq '.items[] | select(.id=="bug-retryable-retry1")' "$AL")
[ -n "$ROW" ] && ok "row appended" || bad "row not appended"
[ "$(jq -r '.retry_of'    <<<"$ROW")" = "bug-retryable" ] && ok "retry_of stamped"    || bad "retry_of wrong"
[ "$(jq -r '.retry_round' <<<"$ROW")" = "1" ]             && ok "retry_round stamped" || bad "retry_round wrong"
[ "$(jq -r '.priority'    <<<"$ROW")" = "high" ]          && ok "priority inherited from archive" || bad "priority not inherited"
[ "$(jq -r '.owner_role'  <<<"$ROW")" = "pm-backend" ]    && ok "owner_role inherited" || bad "owner_role not inherited"
[ "$(jq -r '.status'      <<<"$ROW")" = "open" ]          && ok "minted open"          || bad "status wrong"
[ "$(jq -r '.first_open_at' <<<"$ROW")" = "$NOW" ]        && ok "first_open_at=NOW"    || bad "first_open_at wrong"
[ "$(jq -r '.depends_on | length' <<<"$ROW")" = "0" ]     && ok "depends_on empty"     || bad "depends_on wrong"
ROW2=$(jq '.items[] | select(.id=="bug-second-round-retry2")' "$AL")
[ "$(jq -r '.priority' <<<"$ROW2")" = "medium" ] && ok "no-archive-metadata defaults to medium" || bad "default priority wrong"
[ "$(jq '.items | length' "$AL")" = "4" ] && ok "count grew by exactly the mint count" || bad "count wrong"

echo "T3 idempotent: second apply mints nothing"
OUT=$(run --apply)
grep -q "nothing to re-mint" <<<"$OUT" && ok "second run no-op (minted ids now live stems)" || bad "not idempotent: $OUT"

echo "T4 per-run cap"
seed
OUT=$(RETRY_REMINT_CAP=1 ACTION_LIST="$AL" ACTION_ARCHIVE="$ALA" ASSIGN="$ASG" ASSIGN_ARCHIVE="$ASGA" COVERAGE_FILE="$COV" DEV_LOG_FILE="$DEVLOG" RETRY_NOW="$NOW" bash "$SCRIPT" --apply)
grep -q "re-minting 1 of 2" <<<"$OUT" && ok "cap=1 mints exactly 1" || bad "cap not enforced: $OUT"
grep -q "bug-second-round-retry2" <<<"$OUT" && ok "oldest failure wins under cap" || bad "deterministic order broken"

echo "T5 ceil headroom"
seed
OUT=$(BUFFER_CEIL=1 ACTION_LIST="$AL" ACTION_ARCHIVE="$ALA" ASSIGN="$ASG" ASSIGN_ARCHIVE="$ASGA" COVERAGE_FILE="$COV" DEV_LOG_FILE="$DEVLOG" RETRY_NOW="$NOW" bash "$SCRIPT" --apply)
grep -q "no headroom" <<<"$OUT" && ok "ceil blocks minting" || bad "ceil not enforced: $OUT"

echo "T6 missing files no-op"
OUT=$(ACTION_LIST="$TMP/nope.json" bash "$SCRIPT" 2>&1); [ $? -eq 0 ] \
  && ok "missing action-list exits 0" || bad "missing action-list errored"
seed
OUT=$(ACTION_LIST="$AL" ASSIGN="$TMP/no1.json" ASSIGN_ARCHIVE="$TMP/no2.json" bash "$SCRIPT" 2>&1)
grep -q "no failures to retry" <<<"$OUT" && ok "missing assignments exits clean" || bad "missing assignments: $OUT"

echo "T7 dev-truth guards degrade gracefully when inputs are missing (#2153)"
seed
OUT=$(ACTION_LIST="$AL" ACTION_ARCHIVE="$ALA" ASSIGN="$ASG" ASSIGN_ARCHIVE="$ASGA" \
      COVERAGE_FILE="$TMP/no-cov.json" DEV_LOG_FILE="$TMP/no-devlog.txt" \
      RETRY_NOW="$NOW" bash "$SCRIPT" 2>&1); RC=$?
[ "$RC" -eq 0 ] && ok "missing coverage + dev-log exits 0" || bad "missing guard inputs errored (rc=$RC): $OUT"
grep -q "bug-retryable-retry1" <<<"$OUT" && ok "still mints without guard inputs" || bad "no-guard-input run minted nothing: $OUT"
grep -q "bug-cov-done-retry1"  <<<"$OUT" && ok "coverage guard inert when file missing" || bad "coverage guard fired without file"
grep -q "bug-on-dev-retry1"    <<<"$OUT" && ok "dev-log guard inert when file missing" || bad "dev-log guard fired without file"

echo "T8 archive-marker guard: latest-row semantics + graceful degradation (#2460)"
seed
# (b) an OLDER marked archive row superseded by a NEWER unmarked row in the
#     same stem must NOT exclude — the guard keys on the LATEST row only.
A8="$TMP/al8.json"; ALA8="$TMP/ala8.json"; ASG8="$TMP/asg8.json"; ASGA8="$TMP/asga8.json"
cat > "$A8" <<'EOF'
{ "version": 1, "items": [] }
EOF
cat > "$ALA8" <<'EOF'
{ "version": 1, "items": [
  { "id": "bug-superseded",     "action": "[DROPPED: stale] first attempt", "status": "dropped", "last_updated": "2026-05-01T00:00:00Z" },
  { "id": "bug-superseded-fix", "action": "genuine reopen, no marker",        "status": "open",    "last_updated": "2026-06-25T00:00:00Z" }
] }
EOF
cat > "$ASG8" <<'EOF'
{ "assignments": [] }
EOF
cat > "$ASGA8" <<'EOF'
{ "assignments": [
  { "task_id": "bug-superseded", "status": "failed", "pr_number": null, "last_updated": "2026-06-20T00:00:00Z" }
] }
EOF
OUT=$(ACTION_LIST="$A8" ACTION_ARCHIVE="$ALA8" ASSIGN="$ASG8" ASSIGN_ARCHIVE="$ASGA8" \
      RETRY_NOW="$NOW" bash "$SCRIPT")
grep -q "bug-superseded-retry1" <<<"$OUT" && ok "newer unmarked archive row supersedes older marker -> mints" || bad "latest-row semantics wrong: $OUT"
# (c) guard inert when the action-list-archive file is absent: bug-ghost-marked
#     (blocked only by the marker in T1) becomes eligible and mints.
OUT=$(ACTION_LIST="$AL" ACTION_ARCHIVE="$TMP/no-ala.json" ASSIGN="$ASG" ASSIGN_ARCHIVE="$ASGA" \
      COVERAGE_FILE="$COV" DEV_LOG_FILE="$DEVLOG" RETRY_NOW="$NOW" bash "$SCRIPT"); RC=$?
[ "$RC" -eq 0 ] && ok "missing archive exits 0" || bad "missing archive errored (rc=$RC): $OUT"
grep -q "bug-ghost-marked-retry1" <<<"$OUT" && ok "archive-marker guard inert when archive missing" || bad "marker guard fired without archive file: $OUT"

echo "T9 already-landed summary guard: signal variants + graceful degradation (#2743)"
# Each "already-landed" summary variant must exclude its stem; a benign summary
# and a MISSING field must NOT exclude. Intrinsic-row signal — no external file,
# so it fires regardless of coverage/dev-log presence.
A9="$TMP/al9.json"; ALA9="$TMP/ala9.json"; ASG9="$TMP/asg9.json"; ASGA9="$TMP/asga9.json"
cat > "$A9"   <<'EOF'
{ "version": 1, "items": [] }
EOF
cat > "$ALA9" <<'EOF'
{ "version": 1, "items": [] }
EOF
cat > "$ASG9" <<'EOF'
{ "assignments": [] }
EOF
cat > "$ASGA9" <<'EOF'
{ "assignments": [
  { "task_id": "sig-already-fixed",  "status": "failed", "pr_number": null, "last_updated": "2026-06-20T00:00:00Z", "implementer_summary": "already fixed on dev under #999" },
  { "task_id": "sig-empty-diff",     "status": "failed", "pr_number": null, "last_updated": "2026-06-20T00:00:00Z", "implementer_summary": "landed out-of-band; empty diff, closing" },
  { "task_id": "sig-no-pr-needed",   "status": "failed", "pr_number": null, "last_updated": "2026-06-20T00:00:00Z", "implementer_summary": "No PR needed — satisfied elsewhere" },
  { "task_id": "sig-benign",         "status": "failed", "pr_number": null, "last_updated": "2026-06-20T00:00:00Z", "implementer_summary": "verify-env failure: disk pressure, will retry" },
  { "task_id": "sig-no-field",       "status": "failed", "pr_number": null, "last_updated": "2026-06-20T00:00:00Z" }
] }
EOF
OUT=$(ACTION_LIST="$A9" ACTION_ARCHIVE="$ALA9" ASSIGN="$ASG9" ASSIGN_ARCHIVE="$ASGA9" \
      RETRY_NOW="$NOW" bash "$SCRIPT"); RC=$?
[ "$RC" -eq 0 ] && ok "guard run exits 0" || bad "guard run errored (rc=$RC): $OUT"
grep -q "sig-already-fixed" <<<"$OUT" && bad "'already fixed' must exclude" || ok "'already fixed' excluded"
grep -q "sig-empty-diff"    <<<"$OUT" && bad "'empty diff' must exclude"    || ok "'empty diff' excluded"
grep -q "sig-no-pr-needed"  <<<"$OUT" && bad "'no PR needed' must exclude"  || ok "'no PR needed' excluded"
grep -q "sig-benign-retry1"   <<<"$OUT" && ok "benign summary still mints"       || bad "benign summary wrongly excluded: $OUT"
grep -q "sig-no-field-retry1" <<<"$OUT" && ok "missing field degrades to no-exclusion (mints)" || bad "missing field wrongly excluded: $OUT"

echo
[ "$FAIL" = "0" ] && { echo "ALL PASS"; exit 0; } || { echo "FAILURES"; exit 1; }
