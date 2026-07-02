#!/usr/bin/env bash
# Smoke test for issue-ingest.sh — deterministic, offline (fixture-fed).
# Covers: PR filtering, exclude-labels, dedup (id/stem/assignment/#N-reference),
# label->priority mapping, issue_ref + Closes #N stamp, CEIL/cap bounds,
# additive-count guard, idempotency.
set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
SCRIPT="$HERE/issue-ingest.sh"
FAIL=0
ok()  { printf '  ok    %s\n' "$1"; }
bad() { printf '  FAIL  %s\n' "$1"; FAIL=1; }

TMP="$(mktemp -d)"; trap 'rm -rf "$TMP"' EXIT
AL="$TMP/action-list.json"; ASG="$TMP/assignments.json"; ASGA="$TMP/assignments-archive.json"; ISS="$TMP/issues.json"
NOW="2026-07-02T12:00:00Z"

seed() {
  cat > "$AL" <<'JSON'
{"generated":"2026-07-01T00:00:00Z","items":[
  {"id":"gh-issue-100","action":"already ingested (Closes #100)","owner_role":"pm-backend","priority":"low","status":"open","source":"manual-seed","deadline":null,"dependency":null,"depends_on":[],"issue_ref":{"number":100}},
  {"id":"dep-bump-ammonia","action":"bump ammonia per #2009 RUSTSEC","owner_role":"pm-security","priority":"high","status":"open","source":"backlog","deadline":null,"dependency":null,"depends_on":[]}
]}
JSON
  cat > "$ASG" <<'JSON'
{"generated":"2026-07-01T00:00:00Z","assignments":[
  {"task_id":"gh-issue-200","branch":"auto-impl/gh-issue-200","status":"in-progress","claimed_at":"2026-07-01T00:00:00Z","last_updated":"2026-07-01T00:00:00Z","status_changed_at":"2026-07-01T00:00:00Z"}
]}
JSON
  echo '{"generated":"2026-07-01T00:00:00Z","assignments":[]}' > "$ASGA"
  cat > "$ISS" <<'JSON'
[
  {"number":300,"title":"Fix SSR crash on listing page","labels":[{"name":"bug"},{"name":"frontend"}],"html_url":"https://x/300"},
  {"number":301,"title":"Wire tenant migration import","labels":[{"name":"enhancement"},{"name":"backend"}],"html_url":"https://x/301"},
  {"number":302,"title":"Docs cleanup idea","labels":[{"name":"documentation"}],"html_url":"https://x/302"},
  {"number":303,"title":"Should we redesign X?","labels":[{"name":"discussion"}],"html_url":"https://x/303"},
  {"number":304,"title":"This is actually a PR","labels":[],"pull_request":{"url":"https://x/pr/304"},"html_url":"https://x/304"},
  {"number":100,"title":"dup of existing action-list id","labels":[{"name":"bug"}],"html_url":"https://x/100"},
  {"number":200,"title":"dup of in-progress assignment","labels":[{"name":"bug"}],"html_url":"https://x/200"},
  {"number":2009,"title":"cargo-deny ammonia advisory","labels":[{"name":"security"}],"html_url":"https://x/2009"}
]
JSON
}

run() { GH_ISSUES_FILE="$ISS" ISSUE_INGEST_NOW="$NOW" ACTION_LIST="$AL" ASSIGN="$ASG" ASSIGN_ARCHIVE="$ASGA" \
    BUFFER_CEIL="${BUFFER_CEIL:-120}" ISSUE_INGEST_CAP="${ISSUE_INGEST_CAP:-12}" bash "$SCRIPT" "$@"; }

echo "== issue-ingest smoke test =="

# 1. dry-run writes nothing
seed; B=$(jq -c . "$AL"); run >/dev/null 2>&1; [ "$(jq -c . "$AL")" = "$B" ] && ok "dry-run wrote nothing" || bad "dry-run mutated"

# 2. apply ingests only the eligible issues (300, 301) — 302 doc kept as low
seed; run --apply >/dev/null 2>&1
ING=$(jq -r '[.items[]|select(.source|startswith("dispatcher-issue-ingest "))|.issue_ref.number]|sort|join(",")' "$AL")
[ "$ING" = "300,301,302" ] && ok "ingested eligible issues 300,301,302" || bad "ingested set = [$ING] (expected 300,301,302)"

# 3. filters
has(){ jq -e --argjson n "$1" '[.items[]|select(.issue_ref.number==$n)]|length==1' "$AL" >/dev/null; }
has 303 && bad "discussion issue 303 ingested (should be excluded)" || ok "excluded label (discussion) skipped"
has 304 && bad "PR 304 ingested (should be filtered)" || ok "pull_request object filtered out"
# 100 already an action-list id, 200 an assignment, 2009 referenced via '#2009' text
CNT100=$(jq '[.items[]|select(.id=="gh-issue-100")]|length' "$AL"); [ "$CNT100" = "1" ] && ok "existing gh-issue-100 not duplicated" || bad "gh-issue-100 dup (count=$CNT100)"
has 200 && bad "issue 200 ingested despite in-progress assignment" || ok "assignment-tracked issue 200 skipped"
has 2009 && bad "issue 2009 ingested despite existing #2009 reference" || ok "issue 2009 skipped (referenced by another row)"

# 4. label -> priority + schema + Closes #N
p(){ jq -r --argjson n "$1" '.items[]|select(.issue_ref.number==$n)|.priority' "$AL"; }
[ "$(p 300)" = "high" ]   && ok "bug label -> high"          || bad "issue 300 priority=$(p 300)"
[ "$(p 301)" = "medium" ] && ok "enhancement label -> medium"|| bad "issue 301 priority=$(p 301)"
[ "$(p 302)" = "low" ]    && ok "unlabeled-ish -> low"       || bad "issue 302 priority=$(p 302)"
jq -e '.items[]|select(.issue_ref.number==300)| .status=="open" and .depends_on==[] and .first_open_at=="2026-07-02T12:00:00Z" and (.action|test("Closes #300")) and .owner_role=="pm-frontend"' "$AL" >/dev/null \
  && ok "ingested row schema (open, depends_on, first_open_at, Closes #N, role)" || bad "issue 300 row schema wrong"

# 5. idempotency: second --apply is a no-op
A=$(jq -c . "$AL"); run --apply >/dev/null 2>&1; [ "$(jq -c . "$AL")" = "$A" ] && ok "second --apply no-op (idempotent)" || bad "second --apply mutated"

# 6. CEIL headroom bounds ingestion
seed; BUFFER_CEIL=$(( $(jq '.items|length' "$AL") + 1 )) run --apply >/dev/null 2>&1
NADD=$(jq '[.items[]|select(.source|startswith("dispatcher-issue-ingest "))]|length' "$AL")
[ "$NADD" = "1" ] && ok "CEIL headroom caps ingest to 1" || bad "CEIL headroom not honored (added $NADD)"

# 7. per-run cap
seed; ISSUE_INGEST_CAP=1 run --apply >/dev/null 2>&1
NADD=$(jq '[.items[]|select(.source|startswith("dispatcher-issue-ingest "))]|length' "$AL")
[ "$NADD" = "1" ] && ok "per-run cap=1 honored" || bad "cap not honored (added $NADD)"

echo
if [ "$FAIL" = "0" ]; then echo "==> issue-ingest smoke test PASSED"; exit 0
else echo "==> issue-ingest smoke test FAILED"; exit 1; fi
