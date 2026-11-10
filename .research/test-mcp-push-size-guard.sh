#!/usr/bin/env bash
# Smoke test for mcp-push-size-guard.sh (issue #2126).
#
# The guard is the belt-and-suspenders backstop that keeps dispatcher Phase 6
# from MCP-pushing a file larger than the 64 KiB inline ceiling (issue #1014:
# a truncated inline push corrupts state silently on dev). PR #2114 shipped the
# guard but wired it AFTER `git commit` in Phase 6, where `git diff --cached` is
# empty — so the `--staged` mode was never actually exercised and the guard was
# dead code. This harness locks in the behaviours that matter:
#
#   (a) an OVERSIZE STAGED file trips exit 3 (the real push-payload check).
#   (b) PUSH_METHOD=git SKIPS (exit 0) — direct git push has no inline limit.
#   (c) the EXACT Phase-6 sequence — final `git add` -> guard --staged (BEFORE
#       any commit) — catches an oversize staged file. This is the regression
#       the #2126 reorder fixes: run the guard while the index still differs
#       from HEAD.
#   (d) archive-routing (#1162/#2743): when the ONLY oversize staged file is a
#       known append-only archive (assignments-archive.json / action-list-
#       archive.json), the guard exits 4 ("route via git push") — NOT 3 — so a
#       legit archive append can never fail-close-wedge a Phase 6 push.
#   (e) an oversize ACTIVE state file alongside an oversize archive STILL trips
#       exit 3 — the #1014 corruption class fail-closes hard regardless.
#
# Plus a few guardrails: a small staged file passes; explicit-file mode still
# trips on an oversize arg; and (the dead-guard regression itself) running the
# guard AFTER a commit sees an empty staged set and is a no-op — proving why the
# post-commit placement was broken.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GUARD="$SCRIPT_DIR/mcp-push-size-guard.sh"

CEIL=65536                       # matches the guard's default MCP_INLINE_MAX_BYTES
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

fail_count=0
ok()   { printf '  ok    %s\n' "$1"; }
fail() { printf '  FAIL  %s\n' "$1" >&2; fail_count=$((fail_count+1)); }

# Assert a command exits with an expected code without aborting the run.
expect_rc() {  # $1=expected rc  $2=label ; command follows on stdin via "$@"
  local want="$1" label="$2"; shift 2
  local got=0
  "$@" >/dev/null 2>&1 || got=$?
  if [ "$got" = "$want" ]; then ok "$label (rc=$got)"; else fail "$label expected rc=$want got rc=$got"; fi
}

# --- Build an isolated git repo so `git diff --cached` is deterministic -------
REPO="$TMP/repo"
mkdir -p "$REPO"
git -C "$REPO" init -q
git -C "$REPO" config user.email test@example.com
git -C "$REPO" config user.name test
# Seed one committed file so HEAD exists.
echo "seed" > "$REPO/seed.txt"
git -C "$REPO" add seed.txt
git -C "$REPO" -c commit.gpgsign=false commit -qm seed

mk_file() {  # $1=path  $2=size-bytes
  head -c "$2" /dev/zero | tr '\0' 'x' > "$1"
}

SMALL="$REPO/small.json"       ; mk_file "$SMALL" 1024                 # 1 KiB
BIG="$REPO/action-list.json"   ; mk_file "$BIG"   $((CEIL + 4096))     # 68 KiB > ceiling

# =============================================================================
# (a) an OVERSIZE STAGED file trips exit 3
# =============================================================================
git -C "$REPO" add small.json action-list.json
expect_rc 3 "(a) oversize STAGED file trips exit 3" \
  bash -c "cd '$REPO' && PUSH_METHOD=mcp bash '$GUARD' --staged"

# =============================================================================
# (b) PUSH_METHOD=git skips (exit 0) regardless of staged size
# =============================================================================
expect_rc 0 "(b) PUSH_METHOD=git skips the guard" \
  bash -c "cd '$REPO' && PUSH_METHOD=git bash '$GUARD' --staged"

# =============================================================================
# (c) the exact Phase-6 `git add` -> guard --staged (BEFORE commit) sequence
#     catches an oversize staged file. Runs against a fresh index that is NOT
#     yet committed — the state the #2126 reorder guarantees.
# =============================================================================
git -C "$REPO" reset -q                          # clear the index from (a)
mk_file "$REPO/assignments.json" $((CEIL + 20000))   # a 2nd oversize state file
# Phase-6 shape: the final `git add` of the state files, then the guard, all
# BEFORE `git commit`.
git -C "$REPO" add assignments.json small.json
expect_rc 3 "(c) Phase-6 add->guard (pre-commit) catches oversize staged file" \
  bash -c "cd '$REPO' && PUSH_METHOD=mcp bash '$GUARD' --staged"

# =============================================================================
# (d) archive-routing (#1162/#2743): an oversize APPEND-ONLY ARCHIVE is the ONLY
#     oversize staged file -> exit 4 ("route via git push"), NOT the fail-closed
#     exit 3. This is the regression that wedged the pipeline: a legit archive
#     append must never abort the whole push.
# =============================================================================
git -C "$REPO" reset -q
BIGARCH="$REPO/assignments-archive.json" ; mk_file "$BIGARCH" $((CEIL + 200000))  # ~256 KiB archive
git -C "$REPO" add assignments-archive.json small.json
expect_rc 4 "(d) oversize archive-only staged set routes to git (exit 4)" \
  bash -c "cd '$REPO' && PUSH_METHOD=mcp bash '$GUARD' --staged"

# The 2nd known archive basename routes too (explicit-file mode, exit 4).
BIGARCH2="$REPO/action-list-archive.json" ; mk_file "$BIGARCH2" $((CEIL + 100000))
expect_rc 4 "(d) oversize action-list-archive arg routes to git (exit 4)" \
  bash -c "PUSH_METHOD=mcp bash '$GUARD' '$BIGARCH2'"

# =============================================================================
# (e) an oversize ACTIVE state file ALONGSIDE an oversize archive STILL trips
#     exit 3 — the #1014 corruption class fail-closes hard and dominates routing.
# =============================================================================
git -C "$REPO" reset -q
git -C "$REPO" add assignments-archive.json action-list.json    # archive(4) + active(3)
expect_rc 3 "(e) oversize active file dominates: archive+active staged trips exit 3" \
  bash -c "cd '$REPO' && PUSH_METHOD=mcp bash '$GUARD' --staged"

# =============================================================================
# Guardrail: a small-only staged set passes (exit 0).
# =============================================================================
git -C "$REPO" reset -q
git -C "$REPO" add small.json
expect_rc 0 "small-only staged set passes" \
  bash -c "cd '$REPO' && PUSH_METHOD=mcp bash '$GUARD' --staged"

# =============================================================================
# Guardrail: explicit-file mode still trips on an oversize arg (exit 3).
# =============================================================================
expect_rc 3 "explicit oversize file arg trips exit 3" \
  bash -c "PUSH_METHOD=mcp bash '$GUARD' '$BIG'"

# =============================================================================
# Regression witness (#2126): running the guard AFTER `git commit` sees an
# empty staged set (index == HEAD) and is a dead no-op (exit 0) even though the
# committed file is oversize. This is precisely why the post-commit placement
# was broken; the fix moves the guard BEFORE the commit.
# =============================================================================
git -C "$REPO" reset -q
git -C "$REPO" add action-list.json
git -C "$REPO" -c commit.gpgsign=false commit -qm "commit oversize file"
POST_OUT=$(cd "$REPO" && PUSH_METHOD=mcp bash "$GUARD" --staged 2>&1); POST_RC=$?
if [ "$POST_RC" = "0" ] && grep -q "no files to check" <<<"$POST_OUT"; then
  ok "(#2126 witness) post-commit --staged is an empty no-op — why the old placement was dead"
else
  fail "(#2126 witness) expected empty-staged no-op post-commit (rc=$POST_RC): $POST_OUT"
fi

echo
if [ "$fail_count" -eq 0 ]; then
  echo "==> mcp-push-size-guard smoke test PASSED"
else
  echo "==> mcp-push-size-guard smoke test FAILED ($fail_count)" >&2
  exit 1
fi
