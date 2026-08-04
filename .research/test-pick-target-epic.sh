#!/usr/bin/env bash
# Smoke test for pick-target-epic.sh.
#
# Synthesises a tiny action-list / assignments / archive in a tmpdir and
# asserts the picker behaviour across the four cases that matter:
#   1. Empty objective → SELECT closest-to-done.
#   2. Current target still has claimable work → KEEP.
#   3. Current target exhausted (all claimed/merged) → REPICK.
#   4. All open work is dep-blocked or claimed → action=empty.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PICKER="$SCRIPT_DIR/pick-target-epic.sh"

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

# Common args pointing at fixtures inside $TMP.
ARGS=(--action-list "$TMP/action-list.json"
      --assignments "$TMP/assignments.json"
      --archive     "$TMP/archive.json"
      --objective   "$TMP/objective.json")

fail_count=0
ok()   { printf '  ok    %s\n' "$1"; }
fail() { printf '  FAIL  %s\n' "$1" >&2; fail_count=$((fail_count+1)); }

# Helper: write a minimal action-list with the named open items.
write_actions() {
  jq -n --argjson rows "$1" '{generated: "test", items: $rows}' > "$TMP/action-list.json"
}
write_assignments() {
  jq -n --argjson rows "$1" '{generated: "test", assignments: $rows}' > "$TMP/assignments.json"
}
write_archive() {
  jq -n --argjson rows "$1" '{generated: "test", assignments: $rows}' > "$TMP/archive.json"
}

# ---------------------------------------------------------------------------
# Case 1 — empty objective, multiple epics. Closest-to-done = gap-7 (1 open).
# ---------------------------------------------------------------------------
write_actions '[
  {"id":"gap-10b-1","status":"open","priority":"medium","depends_on":[]},
  {"id":"gap-10b-2","status":"open","priority":"medium","depends_on":[]},
  {"id":"gap-10b-3","status":"open","priority":"medium","depends_on":[]},
  {"id":"gap-7-1",  "status":"open","priority":"high",  "depends_on":[]},
  {"id":"gap-82-1", "status":"open","priority":"low",   "depends_on":[]},
  {"id":"gap-82-2", "status":"open","priority":"low",   "depends_on":[]}
]'
write_assignments '[]'
write_archive '[]'
rm -f "$TMP/objective.json"

out=$("$PICKER" --json "${ARGS[@]}")
ep=$(echo "$out" | jq -r '.epic_prefix')
action=$(echo "$out" | jq -r '.action')
if [ "$ep" = "gap-7" ] && [ "$action" = "select" ]; then
  ok "Case 1: empty objective → SELECT closest-to-done (gap-7)"
else
  fail "Case 1: expected ep=gap-7 action=select; got ep=$ep action=$action"
fi

# Persist for Case 2.
"$PICKER" --update "${ARGS[@]}" >/dev/null

# ---------------------------------------------------------------------------
# Case 2 — current target still has claimable work → KEEP.
# ---------------------------------------------------------------------------
# Same action-list, no assignments yet. gap-7 still has 1 claimable.
out=$("$PICKER" --json "${ARGS[@]}")
ep=$(echo "$out" | jq -r '.epic_prefix')
action=$(echo "$out" | jq -r '.action')
if [ "$ep" = "gap-7" ] && [ "$action" = "keep" ]; then
  ok "Case 2: current target has claimable work → KEEP"
else
  fail "Case 2: expected ep=gap-7 action=keep; got ep=$ep action=$action"
fi

# ---------------------------------------------------------------------------
# Case 3 — current target exhausted (all gap-7 work claimed/merged) → REPICK.
# Now closest-to-done is whichever remaining epic has fewest open.
# We claim gap-7-1 to active so it's no longer claimable; then expect repick
# to gap-82 (2 open) or gap-10b (3 open). gap-82 wins.
# ---------------------------------------------------------------------------
write_assignments '[
  {"task_id":"gap-7-1","status":"in-progress","branch":"auto-impl/gap-7-1"}
]'
out=$("$PICKER" --json "${ARGS[@]}")
ep=$(echo "$out" | jq -r '.epic_prefix')
action=$(echo "$out" | jq -r '.action')
if [ "$ep" = "gap-82" ] && [ "$action" = "repick" ]; then
  ok "Case 3: target exhausted → REPICK to next closest-to-done (gap-82)"
else
  fail "Case 3: expected ep=gap-82 action=repick; got ep=$ep action=$action"
fi

# ---------------------------------------------------------------------------
# Case 4 — all open work is dep-blocked or claimed → action=empty.
# ---------------------------------------------------------------------------
write_actions '[
  {"id":"gap-99-1","status":"open","priority":"medium","depends_on":["gap-99-0"]},
  {"id":"gap-99-2","status":"open","priority":"medium","depends_on":["gap-99-0"]}
]'
write_assignments '[]'
write_archive '[]'
rm -f "$TMP/objective.json"

out=$("$PICKER" --json "${ARGS[@]}")
ep=$(echo "$out" | jq -r '.epic_prefix')
action=$(echo "$out" | jq -r '.action')
if [ "$ep" = "NONE" ] && [ "$action" = "empty" ]; then
  ok "Case 4: all dep-blocked → action=empty"
else
  fail "Case 4: expected ep=NONE action=empty; got ep=$ep action=$action"
fi

# ---------------------------------------------------------------------------
# Case 5 — tie-break: two epics with same open count, higher priority wins.
# gap-50 and gap-51 each have 1 claimable open. gap-50:critical, gap-51:medium.
# Expect gap-50.
# ---------------------------------------------------------------------------
write_actions '[
  {"id":"gap-50-1","status":"open","priority":"critical","depends_on":[]},
  {"id":"gap-51-1","status":"open","priority":"medium","depends_on":[]}
]'
write_assignments '[]'
write_archive '[]'
rm -f "$TMP/objective.json"

out=$("$PICKER" --json "${ARGS[@]}")
ep=$(echo "$out" | jq -r '.epic_prefix')
if [ "$ep" = "gap-50" ]; then
  ok "Case 5: tie-break on open count → higher priority wins (gap-50)"
else
  fail "Case 5: expected ep=gap-50; got ep=$ep"
fi

# ---------------------------------------------------------------------------
# Case 6 — camelCase-derived flat task_id must yield a T21-valid epic_prefix.
# Regression for issue #2651: a closest-to-done epic whose task_id derives
# from a camelCase source filename (e.g. …-useOfflineSupport-ts) flows through
# the epic_prefix else-branch (full task_id). Before the fix the picker wrote
# the verbatim mixed-case id, which trips dispatcher self-test T21 (epic_prefix
# must be all-lowercase kebab) and aborts the finish-first commit. The picker
# must normalise the selected prefix to lowercase kebab.
# ---------------------------------------------------------------------------
write_actions '[
  {"id":"churn-hotspot-frontend-mobile-src-hooks-useOfflineSupport-ts","status":"open","priority":"medium","depends_on":[]}
]'
write_assignments '[]'
write_archive '[]'
rm -f "$TMP/objective.json"

# Same T21 acceptance regex as .research/dispatcher-self-test.sh.
T21_KEBAB='^[a-z0-9][a-z0-9._-]*$'

out=$("$PICKER" --json "${ARGS[@]}")
ep=$(echo "$out" | jq -r '.epic_prefix')
action=$(echo "$out" | jq -r '.action')
if [ "$action" = "select" ] && printf '%s' "$ep" | grep -Eq "$T21_KEBAB"; then
  ok "Case 6: camelCase task_id → T21-valid lowercase epic_prefix ($ep)"
else
  fail "Case 6: expected select + T21-valid lowercase kebab epic_prefix; got ep=$ep action=$action"
fi

# And the persisted objective.json must satisfy T21 too.
"$PICKER" --update "${ARGS[@]}" >/dev/null
persisted_ep=$(jq -r '.epic_prefix' "$TMP/objective.json")
if printf '%s' "$persisted_ep" | grep -Eq "$T21_KEBAB"; then
  ok "Case 6: persisted objective.epic_prefix passes T21 kebab ($persisted_ep)"
else
  fail "Case 6: persisted objective.epic_prefix trips T21; got '$persisted_ep'"
fi

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
if [ "$fail_count" -eq 0 ]; then
  echo "==> test-pick-target-epic: PASS"
  exit 0
else
  echo "==> test-pick-target-epic: FAIL ($fail_count case(s))"
  exit 1
fi
