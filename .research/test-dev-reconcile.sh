#!/usr/bin/env bash
# Smoke test for dev-reconcile.sh.
#
# Synthesises a tiny action-list + an injected dev commit-log fixture
# (DEV_LOG_FILE) in a tmpdir and asserts the four behaviours that matter:
#   1. An OPEN item whose subject landed on dev is detected (dry-run reports it).
#   2. --apply closes ONLY the landed item (open->done) and stamps evidence.
#   3. A dispatcher chore commit that merely MENTIONS the id in its body is NOT
#      a match (subject-prefix join, not a body grep) — no false positive.
#   4. Re-running --apply on the reconciled file is a no-op (idempotent).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RECONCILE="$SCRIPT_DIR/dev-reconcile.sh"

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

AL="$TMP/action-list.json"
LOG="$TMP/dev-log.tsv"

fail_count=0
ok()   { printf '  ok    %s\n' "$1"; }
fail() { printf '  FAIL  %s\n' "$1" >&2; fail_count=$((fail_count+1)); }

# action-list: one landed-open item, one not-landed open item, one already-done.
cat > "$AL" <<'JSON'
{
  "generated": "test",
  "items": [
    { "id": "feat-landed-thing-backend",  "status": "open" },
    { "id": "feat-pending-thing-backend", "status": "open" },
    { "id": "feat-old-thing-backend",     "status": "done" }
  ]
}
JSON

# Injected dev log: a real squash-merge subject for the landed item, plus a
# decoy dispatcher chore commit that names the PENDING id only in its subject
# body text (after a colon-prefixed conventional type) — must NOT match.
printf '%s\t%s\n' \
  "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" "feat-landed-thing-backend: ship the thing (#999)" \
  "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb" "chore(research): dispatcher 2026-06-15 — claimed feat-pending-thing-backend" \
  > "$LOG"

# --- 1. dry-run detects the landed item, not the pending one ----------------
OUT=$(DEV_LOG_FILE="$LOG" ACTION_LIST="$AL" "$RECONCILE" 2>&1 || true)
if grep -q "landed-but-open=1" <<<"$OUT" && grep -q "feat-landed-thing-backend" <<<"$OUT"; then
  ok "dry-run detects the one landed-but-open item"
else
  fail "dry-run did not report exactly the landed item"; echo "$OUT" >&2
fi
if grep -q "feat-pending-thing-backend" <<<"$OUT"; then
  fail "dry-run FALSE-POSITIVE matched a body-only id (subject-prefix join broken)"
else
  ok "no false positive on the chore-body id mention"
fi
# dry-run must not mutate the file.
if [ "$(jq -r '.items[] | select(.id=="feat-landed-thing-backend") | .status' "$AL")" = "open" ]; then
  ok "dry-run wrote nothing"
else
  fail "dry-run mutated the action-list"
fi

# --- 2. --apply closes only the landed item + stamps evidence ---------------
DEV_LOG_FILE="$LOG" ACTION_LIST="$AL" DEV_RECONCILE_NOW="2026-06-15T00:00:00Z" "$RECONCILE" --apply >/dev/null 2>&1 || true
LANDED_ST=$(jq -r '.items[] | select(.id=="feat-landed-thing-backend") | .status' "$AL")
PENDING_ST=$(jq -r '.items[] | select(.id=="feat-pending-thing-backend") | .status' "$AL")
EVID=$(jq -r '.items[] | select(.id=="feat-landed-thing-backend") | .dev_reconciled.commit // ""' "$AL")
[ "$LANDED_ST" = "done" ] && ok "landed item flipped open->done" || fail "landed item not closed (status=$LANDED_ST)"
[ "$PENDING_ST" = "open" ] && ok "pending item left open"        || fail "pending item wrongly touched (status=$PENDING_ST)"
[ -n "$EVID" ]            && ok "dev_reconciled evidence stamped" || fail "no dev_reconciled stamp"
[ "$(jq '.items | length' "$AL")" = "3" ] && ok "item count invariant (3)" || fail "item count changed"

# --- 3 already covered by the no-false-positive assertion above -------------

# --- 4. idempotent: second --apply changes nothing --------------------------
BEFORE=$(jq -S . "$AL")
DEV_LOG_FILE="$LOG" ACTION_LIST="$AL" DEV_RECONCILE_NOW="2026-06-15T00:00:00Z" "$RECONCILE" --apply >/dev/null 2>&1 || true
AFTER=$(jq -S . "$AL")
[ "$BEFORE" = "$AFTER" ] && ok "second --apply is a no-op (idempotent)" || fail "re-apply mutated a clean file"

echo
if [ "$fail_count" -eq 0 ]; then
  echo "==> dev-reconcile smoke test PASSED"
else
  echo "==> dev-reconcile smoke test FAILED ($fail_count)" >&2
  exit 1
fi
