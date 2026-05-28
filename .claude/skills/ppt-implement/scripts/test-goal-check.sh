#!/usr/bin/env bash
# Smoke test for goal-check.sh.
#
# Synthesises a tiny git repo + plan file + test:/fix: commit pair and
# asserts goal-check.sh exits 0. Then mutates the state to trigger each
# hard-fail path and asserts exit 1.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GOAL_CHECK="$SCRIPT_DIR/goal-check.sh"

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

cd "$TMP"
git init -q
git config user.email "test@example.com"
git config user.name  "test"
git commit --allow-empty -q -m "init"
git checkout -q -b dev
git commit --allow-empty -q -m "dev base"

SLUG="demo-fix-something"
mkdir -p .research/plans/_archive
cat > ".research/plans/$SLUG.md" <<EOF
# $SLUG

**Vector:** bug

## Hypothesis
Demo.

## Suggested approach
1. Step one.

## Test plan
- [ ] cargo test demo

## Out of scope
- \`backend/forbidden/\`
EOF

# Initial backlog with a row for SLUG.
mkdir -p .research
cat > .research/backlog.json <<EOF
{ "items": [ { "id": "$SLUG", "plan": "plans/$SLUG.md", "status": "ready" } ] }
EOF

git add -A
git commit -q -m "chore: seed plan + backlog"

# Branch the implementer would use.
git checkout -q -b "impl/$SLUG"
echo "fix code" > app.rs
git add app.rs
git commit -q -m "test(demo): regression for demo"
echo "more fix" > app2.rs
git add app2.rs
git commit -q -m "fix(demo): apply fix"

FAIL=0
ok()  { printf '  ok    %s\n' "$1"; }
bad() { printf '  FAIL  %s\n' "$1" >&2; FAIL=1; }

# Case 1 — happy path, skip IG7 (no justfile) and IG8 (no archive yet).
set +e
"$GOAL_CHECK" --slug "$SLUG" --base dev --skip IG7,IG8 >/tmp/gc.out 2>&1
rc=$?
set -e
if [ "$rc" = 0 ]; then ok "happy path passes (IG7/IG8 skipped)"
else bad "happy path expected exit 0, got $rc"; cat /tmp/gc.out >&2; fi

# Case 2 — wrong branch.
git checkout -q -b "wrong-branch-name"
set +e
"$GOAL_CHECK" --slug "$SLUG" --base dev --skip IG3,IG7,IG8 >/tmp/gc.out 2>&1
rc=$?
set -e
if [ "$rc" = 1 ]; then ok "wrong branch trips IG2 (exit 1)"
else bad "wrong branch expected exit 1, got $rc"; cat /tmp/gc.out >&2; fi
git checkout -q "impl/$SLUG"

# Case 3 — missing plan.
mv ".research/plans/$SLUG.md" ".research/plans/$SLUG.md.bak"
set +e
"$GOAL_CHECK" --slug "$SLUG" --base dev --skip IG3,IG7,IG8 >/tmp/gc.out 2>&1
rc=$?
set -e
if [ "$rc" = 1 ]; then ok "missing plan trips IG1 (exit 1)"
else bad "missing plan expected exit 1, got $rc"; cat /tmp/gc.out >&2; fi
mv ".research/plans/$SLUG.md.bak" ".research/plans/$SLUG.md"

# Case 4 — diff into Out-of-scope path trips IG6.
mkdir -p backend/forbidden
echo "drift" > backend/forbidden/oops.rs
git add backend/forbidden/oops.rs
git commit -q -m "feat: unwanted drift"
set +e
"$GOAL_CHECK" --slug "$SLUG" --base dev --skip IG3,IG7,IG8 >/tmp/gc.out 2>&1
rc=$?
set -e
if [ "$rc" = 1 ]; then ok "OOS path trips IG6 (exit 1)"
else bad "OOS drift expected exit 1, got $rc"; cat /tmp/gc.out >&2; fi
# Undo the drift commit so subsequent cases start clean.
git reset --hard HEAD~1 -q

# Case 5 — frontmatter-bearing plan still passes IG1.
cat > ".research/plans/$SLUG.md" <<EOF
---
slug: $SLUG
vector: bug
---
# $SLUG

## Hypothesis
Demo.

## Test plan
- [ ] cargo test demo
EOF
git add -A
git commit -q -m "chore: rewrite plan with frontmatter"
set +e
"$GOAL_CHECK" --slug "$SLUG" --base dev --skip IG3,IG7,IG8 >/tmp/gc.out 2>&1
rc=$?
set -e
if [ "$rc" = 0 ]; then ok "frontmatter-bearing plan passes IG1"
else bad "frontmatter plan expected exit 0, got $rc"; cat /tmp/gc.out >&2; fi

if [ "$FAIL" -eq 0 ]; then
  echo "==> test-goal-check: PASS"
  exit 0
else
  echo "==> test-goal-check: FAIL"
  exit 1
fi
