#!/usr/bin/env bash
# stale-pr-guard.test.sh — offline unit test for stale-pr-guard.sh.
# No git history or gh needed: drives the guard with a PRS_FILE fixture whose
# rows carry a precomputed commitsBehind, and a frozen NOW_EPOCH. Sibling of
# dispatcher-self-test.sh. Run by .github/workflows/stale-pr-guard.yml and
# locally: `bash .research/stale-pr-guard.test.sh`.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GUARD="$HERE/stale-pr-guard.sh"
NOW=$(date -u -d '2026-06-28T12:00:00Z' +%s)
FIX="$(mktemp)"
trap 'rm -f "$FIX"' EXIT

cat > "$FIX" <<'EOF'
[
  {"number":1903,"title":"test(BIT-268): ai batch","headRefName":"test/bit-268-ai","headRefOid":"x","createdAt":"2026-06-27T00:00:00Z","isDraft":false,"commitsBehind":3500},
  {"number":1860,"title":"test: BIT-263 financial","headRefName":"test/bit-263","headRefOid":"x","createdAt":"2026-06-23T00:00:00Z","isDraft":false,"commitsBehind":12},
  {"number":1842,"title":"chore(deps): bump","headRefName":"dependabot/x","headRefOid":"x","createdAt":"2026-06-28T09:00:00Z","isDraft":false,"commitsBehind":4},
  {"number":1754,"title":"chore: draft","headRefName":"chore/epic-2","headRefOid":"x","createdAt":"2026-06-23T00:00:00Z","isDraft":true,"commitsBehind":900}
]
EOF

fail() { echo "FAIL: $*" >&2; exit 1; }

OUT=$(NOW_EPOCH="$NOW" PRS_FILE="$FIX" BASE=origin/dev STALE_DAYS=3 BEHIND_LIMIT=200 "$GUARD" --json)

# 1. Exactly two flagged: #1860 (age) and #1903 (behind).
[ "$(echo "$OUT" | jq 'length')" = "2" ] || fail "expected 2 flagged, got $(echo "$OUT" | jq 'length')"
# 2. #1860 flagged on age, not behind.
echo "$OUT" | jq -e '.[]|select(.number==1860)|.reasons==["open>3d"]' >/dev/null || fail "#1860 reasons wrong"
# 3. #1903 flagged on behind.
echo "$OUT" | jq -e '.[]|select(.number==1903)|.reasons==["behind>200"]' >/dev/null || fail "#1903 reasons wrong"
# 4. Recent, in-range #1842 NOT flagged.
echo "$OUT" | jq -e 'any(.[]; .number==1842) | not' >/dev/null || fail "#1842 should not be flagged"
# 5. Draft #1754 excluded even though 900 behind.
echo "$OUT" | jq -e 'any(.[]; .number==1754) | not' >/dev/null || fail "draft #1754 must be excluded"
# 6. subtask id parsed.
echo "$OUT" | jq -e '.[]|select(.number==1903)|.subtask=="bit-268"' >/dev/null || fail "subtask parse wrong"

# 7. Enforce mode exits non-zero when flagged.
if NOW_EPOCH="$NOW" PRS_FILE="$FIX" BASE=origin/dev GUARD_ENFORCE=1 "$GUARD" >/dev/null 2>&1; then
  fail "GUARD_ENFORCE=1 should exit non-zero with flagged PRs"
fi

# 8. Clean set -> exit 0 under enforce.
CLEAN="$(mktemp)"; trap 'rm -f "$FIX" "$CLEAN"' EXIT
echo '[{"number":1,"title":"x","headRefName":"y","headRefOid":"z","createdAt":"2026-06-28T11:00:00Z","isDraft":false,"commitsBehind":1}]' > "$CLEAN"
NOW_EPOCH="$NOW" PRS_FILE="$CLEAN" BASE=origin/dev GUARD_ENFORCE=1 "$GUARD" >/dev/null 2>&1 || fail "clean set should exit 0"

echo "stale-pr-guard.test.sh: all 8 assertions passed"
