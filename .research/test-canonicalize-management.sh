#!/usr/bin/env bash
# test-canonicalize-management.sh — focused tests for canonicalize-management.sh.
#
# Proves the churn-audit claim: two consecutive dispatcher-run snapshots that
# differ ONLY by array order and/or object-key order canonicalize to BYTE-
# IDENTICAL output (reorder/key churn eliminated), while a snapshot that differs
# by a genuine timestamp still differs after canonicalization (real audit data
# is NOT over-normalized).
#
# All fixtures are synthetic and live in a scratch dir — the live
# .research/management/*.json STATE files are never read or written here
# (single-writer rule: those belong to the dispatcher).
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CANON="$HERE/canonicalize-management.sh"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

PASS=0; FAIL=0
ok()   { echo "ok   - $*"; PASS=$((PASS+1)); }
bad()  { echo "FAIL - $*" >&2; FAIL=$((FAIL+1)); }

[ -x "$CANON" ] || { echo "canonicalizer not executable: $CANON" >&2; exit 1; }

# --- Fixtures: two "runs" of the SAME logical assignments.json state. ---------
# run A: rows in order [b, a], keys in author order.
cat > "$WORK/assignments.json" <<'EOF'
{
  "generated": "2026-11-28T10:00:00Z",
  "assignments": [
    { "task_id": "task-b", "status": "review", "branch": "auto/b", "pr_number": 2 },
    { "task_id": "task-a", "status": "in-progress", "branch": "auto/a", "pr_number": null }
  ]
}
EOF

# run B: SAME logical content — rows reordered [a, b], keys permuted,
# and only the `generated` timestamp advanced (genuine data change).
cat > "$WORK/assignments.runB.json" <<'EOF'
{
  "assignments": [
    { "branch": "auto/a", "pr_number": null, "task_id": "task-a", "status": "in-progress" },
    { "status": "review", "task_id": "task-b", "pr_number": 2, "branch": "auto/b" }
  ],
  "generated": "2026-11-28T10:05:00Z"
}
EOF

# T1 — --check flags a non-canonical file (unsorted array + permuted keys).
if "$CANON" --check "$WORK/assignments.json" >/dev/null 2>&1; then
  bad "T1 --check should report run-A fixture as NON-CANONICAL (exit 1)"
else
  ok "T1 --check flags unsorted/permuted file as non-canonical"
fi

# T2 — reorder + key churn collapses: both runs canonicalize to identical bytes
#      EXCEPT the genuine timestamp. Strip `generated` then compare.
# Copies keep the REGISTERED basename (assignments.json) in per-run subdirs so
# the registry still recognizes them.
mkdir -p "$WORK/runA" "$WORK/runB"
cp "$WORK/assignments.json"      "$WORK/runA/assignments.json"
cp "$WORK/assignments.runB.json" "$WORK/runB/assignments.json"
"$CANON" --apply "$WORK/runA/assignments.json" >/dev/null
"$CANON" --apply "$WORK/runB/assignments.json" >/dev/null
if diff <(jq 'del(.generated)' "$WORK/runA/assignments.json") <(jq 'del(.generated)' "$WORK/runB/assignments.json") >/dev/null; then
  ok "T2 two differently-ordered runs canonicalize to identical bytes (minus timestamp)"
else
  bad "T2 canonical forms still differ — reorder/key churn not fully collapsed"
fi

# T3 — genuine timestamp is preserved (NOT over-normalized): the two files still
#      differ on `generated` after canonicalization.
if diff "$WORK/runA/assignments.json" "$WORK/runB/assignments.json" >/dev/null; then
  bad "T3 timestamp was normalized away — real audit data must survive"
else
  ok "T3 genuine timestamp difference survives canonicalization"
fi

# T4 — idempotency: re-canonicalizing a canonical file is a no-op and --check passes.
mkdir -p "$WORK/again"
cp "$WORK/runA/assignments.json" "$WORK/again/assignments.json"
"$CANON" --apply "$WORK/again/assignments.json" >/dev/null
if diff "$WORK/runA/assignments.json" "$WORK/again/assignments.json" >/dev/null && "$CANON" --check "$WORK/runA/assignments.json" >/dev/null 2>&1; then
  ok "T4 canonical form is idempotent and passes --check"
else
  bad "T4 second canonicalize changed bytes or --check failed on canonical file"
fi

# T5 — count/id invariant preserved: no rows dropped or invented.
before="$(jq -S '[.assignments[].task_id] | sort' "$WORK/assignments.json")"
after="$(jq -S '[.assignments[].task_id] | sort' "$WORK/runA/assignments.json")"
if [ "$before" = "$after" ]; then
  ok "T5 task_id multiset preserved through --apply"
else
  bad "T5 task_id multiset changed through canonicalize"
fi

# T6 — action-list.json array sorts by .id (second registered shape).
cat > "$WORK/action-list.json" <<'EOF'
{ "version": 1, "items": [
  { "id": "z-item", "status": "open", "action": "z" },
  { "id": "a-item", "action": "a", "status": "open" }
], "generated": "2026-11-28T10:00:00Z" }
EOF
"$CANON" --apply "$WORK/action-list.json" >/dev/null
first_id="$(jq -r '.items[0].id' "$WORK/action-list.json")"
if [ "$first_id" = "a-item" ]; then
  ok "T6 action-list .items sorted ascending by id"
else
  bad "T6 action-list .items not sorted by id (first=$first_id)"
fi

# T7 — post-merge-review.runs kept newest-first (desc by run_at).
cat > "$WORK/post-merge-review.json" <<'EOF'
{ "runs": [
  { "run_at": "2026-11-25T00:00:00Z", "clean": 1 },
  { "run_at": "2026-11-27T00:00:00Z", "clean": 3 },
  { "run_at": "2026-11-26T00:00:00Z", "clean": 2 }
] }
EOF
"$CANON" --apply "$WORK/post-merge-review.json" >/dev/null
top="$(jq -r '.runs[0].run_at' "$WORK/post-merge-review.json")"
if [ "$top" = "2026-11-27T00:00:00Z" ]; then
  ok "T7 post-merge-review.runs ordered newest-first"
else
  bad "T7 runs not newest-first (top=$top)"
fi

# T8 — unregistered file is skipped by default, key-sorted with --apply-unknown.
cat > "$WORK/misc.json" <<'EOF'
{ "zeta": 1, "alpha": 2 }
EOF
cp "$WORK/misc.json" "$WORK/misc.orig.json"
"$CANON" --apply "$WORK/misc.json" >/dev/null 2>&1 || true
if diff "$WORK/misc.json" "$WORK/misc.orig.json" >/dev/null; then
  ok "T8a unregistered file untouched without --apply-unknown"
else
  bad "T8a unregistered file was modified without --apply-unknown"
fi
"$CANON" --apply --apply-unknown "$WORK/misc.json" >/dev/null 2>&1 || true
if [ "$(jq -r 'keys[0]' "$WORK/misc.json")" = "alpha" ]; then
  ok "T8b unregistered file key-sorted with --apply-unknown"
else
  bad "T8b unregistered file not key-sorted with --apply-unknown"
fi

# T9 — invalid JSON is reported, not silently rewritten.
printf '{ not json ' > "$WORK/broken.json"
cp "$WORK/broken.json" "$WORK/broken.orig.json"
"$CANON" --apply "$WORK/broken.json" >/dev/null 2>&1 || true
if diff "$WORK/broken.json" "$WORK/broken.orig.json" >/dev/null; then
  ok "T9 invalid JSON left untouched"
else
  bad "T9 invalid JSON was mangled"
fi

echo "-----------------------------------------"
echo "canonicalize-management tests: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ]
