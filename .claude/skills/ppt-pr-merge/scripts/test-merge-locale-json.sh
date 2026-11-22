#!/usr/bin/env bash
# Smoke test for merge-locale-json.py — the key-union locale-JSON resolver.
#
# Covers the dispatcher-stall #3357 scenarios:
#   1. Two PRs add DIFFERENT keys to the same nested bundle   -> key-union (0)
#   2. Two PRs add different top-level namespaces             -> key-union (0)
#   3. One side changes a value, other side untouched         -> take change (0)
#   4. One side deletes a base key, other untouched           -> delete (0)
#   5. Both sides change the SAME key to different values      -> REFUSE (2)
#   6. Both sides ADD the same key with different values       -> REFUSE (2)
#   7. Object-vs-scalar structural mismatch                    -> REFUSE (2)
#   8. Git-index mode end-to-end inside a real merge conflict  -> key-union (0)
#   9. --sort-keys emits strict alphabetical order             -> (0)
#  10. Non-ASCII (sk/cs/de) values preserved verbatim          -> (0)
#
# Exit 0 = all pass, 1 = a case failed.

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MERGE="$HERE/merge-locale-json.py"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

PASS=0
FAIL=0

ok()   { printf 'ok   %s\n' "$1"; PASS=$((PASS + 1)); }
bad()  { printf 'FAIL %s\n' "$1"; FAIL=$((FAIL + 1)); }

# run_file <name> <expected_exit> <base|-> <ours> <theirs> [expected_json_file]
run_file() {
  local name="$1" exp="$2" base="$3" ours="$4" theirs="$5" expjson="${6:-}"
  local args=(--ours "$ours" --theirs "$theirs" --output "$WORK/out.json")
  [ "$base" != "-" ] && args=(--base "$base" "${args[@]}")
  set +e
  python3 "$MERGE" "${args[@]}" >"$WORK/stdout" 2>"$WORK/stderr"
  local rc=$?
  set -e
  if [ "$rc" -ne "$exp" ]; then
    bad "$name (exit $rc, expected $exp)"; sed 's/^/     /' "$WORK/stderr"; return
  fi
  if [ -n "$expjson" ]; then
    if ! python3 -c "import json,sys; a=json.load(open('$WORK/out.json')); b=json.load(open('$expjson')); sys.exit(0 if a==b else 1)"; then
      bad "$name (content mismatch)"
      echo "     --- got ---";      sed 's/^/     /' "$WORK/out.json"
      echo "     --- expected ---"; sed 's/^/     /' "$expjson"
      return
    fi
  fi
  ok "$name"
}

jwrite() { printf '%s\n' "$2" > "$WORK/$1"; }

# ---- Case 1: different keys in same nested namespace -> union ---------------
jwrite base1.json   '{"common":{"save":"Save","cancel":"Cancel"}}'
jwrite ours1.json   '{"common":{"save":"Save","cancel":"Cancel","retry":"Retry"}}'
jwrite theirs1.json '{"common":{"save":"Save","cancel":"Cancel","close":"Close"}}'
jwrite exp1.json    '{"common":{"save":"Save","cancel":"Cancel","retry":"Retry","close":"Close"}}'
run_file "nested-key-union" 0 "$WORK/base1.json" "$WORK/ours1.json" "$WORK/theirs1.json" "$WORK/exp1.json"

# ---- Case 2: different top-level namespaces added -> union ------------------
jwrite base2.json   '{"common":{"ok":"OK"}}'
jwrite ours2.json   '{"common":{"ok":"OK"},"faults":{"title":"Faults"}}'
jwrite theirs2.json '{"common":{"ok":"OK"},"news":{"title":"News"}}'
jwrite exp2.json    '{"common":{"ok":"OK"},"faults":{"title":"Faults"},"news":{"title":"News"}}'
run_file "toplevel-namespace-union" 0 "$WORK/base2.json" "$WORK/ours2.json" "$WORK/theirs2.json" "$WORK/exp2.json"

# ---- Case 3: one side changes a value, other untouched -> take change -------
jwrite base3.json   '{"common":{"ok":"OK","save":"Save"}}'
jwrite ours3.json   '{"common":{"ok":"OK","save":"Save it"}}'
jwrite theirs3.json '{"common":{"ok":"OK","save":"Save"}}'
jwrite exp3.json    '{"common":{"ok":"OK","save":"Save it"}}'
run_file "one-sided-change" 0 "$WORK/base3.json" "$WORK/ours3.json" "$WORK/theirs3.json" "$WORK/exp3.json"

# ---- Case 4: one side deletes a base key, other untouched -> delete ---------
jwrite base4.json   '{"common":{"ok":"OK","legacy":"Old"}}'
jwrite ours4.json   '{"common":{"ok":"OK"}}'
jwrite theirs4.json '{"common":{"ok":"OK","legacy":"Old"}}'
jwrite exp4.json    '{"common":{"ok":"OK"}}'
run_file "one-sided-delete" 0 "$WORK/base4.json" "$WORK/ours4.json" "$WORK/theirs4.json" "$WORK/exp4.json"

# ---- Case 5: both change the SAME key differently -> REFUSE (2) -------------
jwrite base5.json   '{"common":{"save":"Save"}}'
jwrite ours5.json   '{"common":{"save":"Store"}}'
jwrite theirs5.json '{"common":{"save":"Keep"}}'
run_file "same-key-divergent-change-refused" 2 "$WORK/base5.json" "$WORK/ours5.json" "$WORK/theirs5.json"

# ---- Case 6: both ADD the same key with different values -> REFUSE (2) ------
jwrite base6.json   '{"common":{"ok":"OK"}}'
jwrite ours6.json   '{"common":{"ok":"OK","retry":"Retry"}}'
jwrite theirs6.json '{"common":{"ok":"OK","retry":"Again"}}'
run_file "same-key-divergent-add-refused" 2 "$WORK/base6.json" "$WORK/ours6.json" "$WORK/theirs6.json"

# ---- Case 7: object-vs-scalar structural mismatch -> REFUSE (2) -------------
jwrite base7.json   '{"a":{"x":"1"}}'
jwrite ours7.json   '{"a":"scalar"}'
jwrite theirs7.json '{"a":{"x":"1","y":"2"}}'
run_file "object-vs-scalar-refused" 2 "$WORK/base7.json" "$WORK/ours7.json" "$WORK/theirs7.json"

# ---- Case 9: --sort-keys strict alphabetical -------------------------------
jwrite ours9.json   '{"common":{"zebra":"Z","alpha":"A"}}'
jwrite theirs9.json '{"common":{"zebra":"Z","alpha":"A","mid":"M"}}'
set +e
python3 "$MERGE" --ours "$WORK/ours9.json" --theirs "$WORK/theirs9.json" --sort-keys >"$WORK/sorted.json" 2>/dev/null
rc=$?
set -e
if [ "$rc" -eq 0 ] && [ "$(python3 -c "import json;print(list(json.load(open('$WORK/sorted.json'))['common']))")" = "['alpha', 'mid', 'zebra']" ]; then
  ok "sort-keys-alphabetical"
else
  bad "sort-keys-alphabetical (rc=$rc)"
fi

# ---- Case 10: non-ASCII values preserved verbatim --------------------------
jwrite base10.json   '{"common":{"ok":"OK"}}'
jwrite ours10.json   '{"common":{"ok":"OK","greet":"Dobrý deň"}}'
jwrite theirs10.json '{"common":{"ok":"OK","bye":"Schöne Grüße"}}'
set +e
python3 "$MERGE" --base "$WORK/base10.json" --ours "$WORK/ours10.json" --theirs "$WORK/theirs10.json" --output "$WORK/u10.json" >/dev/null 2>&1
rc=$?
set -e
if [ "$rc" -eq 0 ] && grep -q 'Dobrý deň' "$WORK/u10.json" && grep -q 'Schöne Grüße' "$WORK/u10.json"; then
  ok "non-ascii-preserved"
else
  bad "non-ascii-preserved (rc=$rc)"
fi

# ---- Case 8: git-index mode end-to-end -------------------------------------
GREPO="$WORK/repo"
mkdir -p "$GREPO"
(
  cd "$GREPO"
  git init -q
  git config user.email t@t; git config user.name t
  git config commit.gpgsign false
  printf '%s\n' '{"common":{"save":"Save","cancel":"Cancel"}}' > en.json
  git add en.json; git commit -qm base
  git checkout -q -b feat-a
  printf '%s\n' '{"common":{"save":"Save","cancel":"Cancel","retry":"Retry"}}' > en.json
  git commit -qam a
  git checkout -q main 2>/dev/null || git checkout -q master
  git checkout -q -b feat-b
  printf '%s\n' '{"common":{"save":"Save","cancel":"Cancel","close":"Close"}}' > en.json
  git commit -qam b
  # Merge feat-a into feat-b -> conflict on en.json
  set +e
  git merge feat-a -q 2>/dev/null
  set -e
  python3 "$MERGE" --git en.json >/dev/null 2>&1
  rcg=$?
  if [ "$rcg" -ne 0 ]; then echo "GITMODE_RC=$rcg"; exit 0; fi
  python3 -c "import json;d=json.load(open('en.json'));k=sorted(d['common']);print('KEYS='+','.join(k))"
) > "$WORK/git.out" 2>&1
if grep -q 'KEYS=cancel,close,retry,save' "$WORK/git.out"; then
  ok "git-index-mode"
else
  bad "git-index-mode"; sed 's/^/     /' "$WORK/git.out"
fi

echo "----"
echo "passed=$PASS failed=$FAIL"
[ "$FAIL" -eq 0 ]
