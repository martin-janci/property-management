#!/usr/bin/env bash
# Deterministic IG1–IG8 goal checks for the ppt-implement skill.
#
# Mirrors the inline commands documented in .research/implementer-prompt.md
# under "Implementation goals (IG1–IG8)" so an implementer can run one
# command before pushing instead of grepping the prompt for snippets.
#
# Each check prints a T-style line (`ok` / `WARN` / `FAIL`) so the output
# is grep-friendly and parseable in CI. Hard-fails set exit=1; advisory
# checks (where a human PR-body decision is required) emit WARN and never
# set exit≠0 by themselves.
#
# Usage:
#   goal-check.sh --slug <slug> [--base dev]
#
# Required:
#   --slug    plan slug (without `.md`) — also used to derive branch
#
# Optional:
#   --base    base branch for diffs (default: dev)
#   --skip    comma-separated list of IGs to skip (e.g. IG3,IG7 — useful
#             when a plan's vector legitimately doesn't require them)
#
# Exit codes:
#   0 = all hard checks passed (warnings allowed)
#   1 = at least one hard check failed (incl. a missing plan file, which is
#       an IG1 FAIL — not a usage error)
#   64 = bad usage (missing/empty flag args, jq absent)
#
# Goal coverage:
#   IG1 plan exists                                 HARD
#   IG2 branch named impl/<slug>                    HARD
#   IG3 ≥1 test commit + ≥1 fix commit on branch    HARD (skippable)
#   IG4 cross-reference in PR body                  WARN-only (PR not local)
#   IG5 plan's Test plan commands exist             WARN-only (parses prose)
#   IG6 no diff in "Out of scope" paths             HARD when OOS declared
#   IG7 `just verify` succeeds (impact-scoped gate) HARD (skippable)
#   IG8 archive-move + backlog flip in this PR      HARD (skippable pre-final-commit)

set -euo pipefail

SLUG=""
BASE="dev"
SKIP_LIST=""

# Under `set -u`, a bare `--slug` (no value) would otherwise abort with an
# unbound-variable error and exit 1. Validate the value is present so the
# script emits a consistent usage error (exit 64) instead.
require_val() {
  # $1 = flag name, $2 = remaining arg count ($#)
  if [ "$2" -lt 2 ]; then
    echo "error: $1 requires a value" >&2
    exit 64
  fi
}

while [ $# -gt 0 ]; do
  case "$1" in
    --slug) require_val "$1" "$#"; SLUG="$2"; shift 2;;
    --base) require_val "$1" "$#"; BASE="$2"; shift 2;;
    --skip) require_val "$1" "$#"; SKIP_LIST="$2"; shift 2;;
    -h|--help)
      sed -n '2,40p' "$0"
      exit 0
      ;;
    *) echo "unknown arg: $1" >&2; exit 64;;
  esac
done

if [ -z "$SLUG" ]; then
  echo "error: --slug required" >&2
  exit 64
fi
if ! command -v jq >/dev/null 2>&1; then
  echo "error: jq is required" >&2
  exit 64
fi

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
PLAN_PATH="$REPO_ROOT/.research/plans/$SLUG.md"
ARCHIVE_PATH="$REPO_ROOT/.research/plans/_archive/$SLUG.md"
EXPECTED_BRANCH="impl/$SLUG"

skipped() {
  case ",$SKIP_LIST," in
    *",$1,"*) return 0;;
    *) return 1;;
  esac
}

fail_count=0
warn_count=0
ok()   { printf '  %s  %s\n' "ok"   "$*"; }
warn() { printf '  %s %s\n' "WARN" "$*"; warn_count=$((warn_count+1)); }
fail() { printf '  %s %s\n' "FAIL" "$*"; fail_count=$((fail_count+1)); }
skip() { printf '  %s %s\n' "skip" "$*"; }

echo "==> ppt-implement goal-check  slug=$SLUG  base=$BASE"
echo "    plan:   $PLAN_PATH"
echo "    repo:   $REPO_ROOT"
[ -n "$SKIP_LIST" ] && echo "    skip:   $SKIP_LIST"
echo

# -----------------------------------------------------------------------
# IG1 — Plan exists and starts with `# <slug>`
# -----------------------------------------------------------------------
echo "IG1  plan exists and is well-formed"
if skipped IG1; then skip "IG1 skipped via --skip"
elif [ ! -f "$PLAN_PATH" ]; then
  if [ -f "$ARCHIVE_PATH" ]; then
    warn "plan already archived at plans/_archive/$SLUG.md — assuming post-IG8 state"
  else
    fail "plan file not found: $PLAN_PATH"
  fi
else
  # Skip optional YAML frontmatter (--- … ---) before checking for H1.
  first_h1="$(awk '
    NR==1 && $0=="---" { in_fm=1; next }
    in_fm && $0=="---" { in_fm=0; next }
    in_fm { next }
    /^# / { print; exit }
  ' "$PLAN_PATH")"
  if [ -z "$first_h1" ]; then
    fail "plan has no markdown H1 after optional frontmatter"
  elif [ "$first_h1" = "# $SLUG" ]; then
    ok "first H1 matches slug: '$first_h1'"
  else
    fail "first H1 is '$first_h1' — expected '# $SLUG' (per implementer-prompt.md IG1)"
  fi
fi
echo

# -----------------------------------------------------------------------
# IG2 — Branch named impl/<slug>
# -----------------------------------------------------------------------
echo "IG2  branch named impl/$SLUG"
if skipped IG2; then skip "IG2 skipped via --skip"
else
  current_branch="$(git -C "$REPO_ROOT" branch --show-current 2>/dev/null || echo '')"
  if [ "$current_branch" = "$EXPECTED_BRANCH" ]; then
    ok "current branch matches: $current_branch"
  else
    fail "current branch '$current_branch' != expected '$EXPECTED_BRANCH'"
  fi
fi
echo

# -----------------------------------------------------------------------
# IG3 — TDD discipline: at least one test: commit AND at least one fix: commit
# This is a proxy for "test fails on main, passes after fix". The script
# cannot rerun the test against a different SHA without checking out — that
# is the implementer's job. But the *shape* (two commits, conventional
# subjects) is verifiable here.
# -----------------------------------------------------------------------
echo "IG3  TDD evidence — separate test: and fix: commits on branch"
if skipped IG3; then skip "IG3 skipped via --skip (vector probably doesn't require IG3)"
else
  # Range: BASE..HEAD on this branch.
  range="$BASE..HEAD"
  if ! git -C "$REPO_ROOT" rev-parse "$BASE" >/dev/null 2>&1; then
    fail "base '$BASE' not found locally — cannot verify this HARD check. Run: git fetch origin $BASE (or pass --skip IG3 if the vector truly doesn't need it)"
  else
    test_commits="$(git -C "$REPO_ROOT" log "$range" --pretty=%s 2>/dev/null | grep -cE '^test(\([^)]+\))?:' || true)"
    fix_commits="$(git -C "$REPO_ROOT" log "$range" --pretty=%s 2>/dev/null | grep -cE '^(fix|feat|perf|refactor)(\([^)]+\))?:' || true)"
    if [ "$test_commits" -ge 1 ] && [ "$fix_commits" -ge 1 ]; then
      ok "found $test_commits test: commit(s) and $fix_commits fix/feat/perf/refactor commit(s) on $range"
    elif [ "$test_commits" -ge 1 ]; then
      warn "found test: commits but no fix/feat/perf/refactor commit on $range — IG3 partially satisfied"
    else
      fail "no test: commit on $range — IG3 requires a failing-on-main test"
    fi
  fi
fi
echo

# -----------------------------------------------------------------------
# IG4 — "Suggested approach" steps addressed in PR body
# Cannot verify locally; emit WARN with guidance.
# -----------------------------------------------------------------------
echo "IG4  suggested-approach cross-reference (PR body)"
if skipped IG4; then skip "IG4 skipped via --skip"
elif [ ! -f "$PLAN_PATH" ]; then
  skip "plan absent — already archived?"
else
  step_count="$(awk '/^## Suggested approach/{f=1;next} f && /^## /{f=0} f' "$PLAN_PATH" | grep -cE '^[0-9]+\.' || true)"
  if [ "$step_count" -gt 0 ]; then
    warn "$step_count Suggested-approach step(s) in plan — confirm each is addressed or skipped in the PR body"
  else
    ok "no numbered Suggested-approach steps"
  fi
fi
echo

# -----------------------------------------------------------------------
# IG5 — Test plan commands exist in the plan
# Cannot run them generically (free-text). Emit WARN if Test plan section
# is empty.
# -----------------------------------------------------------------------
echo "IG5  Test plan section non-empty"
if skipped IG5; then skip "IG5 skipped via --skip"
elif [ ! -f "$PLAN_PATH" ]; then
  skip "plan absent — already archived?"
else
  tp_lines="$(awk '/^## Test plan/{f=1;next} f && /^## /{f=0} f' "$PLAN_PATH" | grep -cE '^- \[' || true)"
  if [ "$tp_lines" -ge 1 ]; then
    ok "$tp_lines Test-plan checklist item(s) declared — quote each command→exit-code in the PR body"
  else
    warn "Test plan section appears empty — add at least one verifiable command"
  fi
fi
echo

# -----------------------------------------------------------------------
# IG6 — No diff in "Out of scope" paths
# Parses bullet paths under `## Out of scope` and greps the diff.
# -----------------------------------------------------------------------
echo "IG6  diff respects 'Out of scope' paths"
if skipped IG6; then skip "IG6 skipped via --skip"
elif [ ! -f "$PLAN_PATH" ]; then
  skip "plan absent — already archived?"
else
  oos_paths="$(awk '/^## Out of scope/{f=1;next} f && /^## /{f=0} f' "$PLAN_PATH" \
    | grep -oE '`[^`]+`' | tr -d '`' \
    | grep -vE '^\s*$' || true)"
  if [ -z "$oos_paths" ]; then
    ok "no Out-of-scope paths declared"
  else
    if ! git -C "$REPO_ROOT" rev-parse "$BASE" >/dev/null 2>&1; then
      fail "base '$BASE' not found — cannot verify OOS paths (HARD). Run: git fetch origin $BASE (or --skip IG6)"
    else
      changed="$(git -C "$REPO_ROOT" diff --name-only "$BASE"..HEAD)"
      breach=""
      while IFS= read -r p; do
        [ -z "$p" ] && continue
        # Glob-style match: treat each declared OOS path as a literal
        # prefix OR a glob. Use grep -E with the path made regex-safe.
        re="$(printf '%s' "$p" | sed -e 's/[][\\.+*?(){}|^$]/\\&/g' -e 's/\\\*/.*/g')"
        hits="$(printf '%s\n' "$changed" | grep -E "^$re" || true)"
        if [ -n "$hits" ]; then
          breach="$breach\n  - $p :: $(printf '%s' "$hits" | tr '\n' ' ')"
        fi
      done <<< "$oos_paths"
      if [ -z "$breach" ]; then
        ok "no changed paths under declared OOS prefixes"
      else
        # %b expands the \n escapes in $breach WITHOUT treating path content
        # (which may contain %) as a printf format string.
        fail "diff touches Out-of-scope paths:$(printf '%b' "$breach")"
      fi
    fi
  fi
fi
echo

# -----------------------------------------------------------------------
# IG7 — just check + just test
# These are slow; gated by --skip IG7 for fast pre-push smoke runs.
# -----------------------------------------------------------------------
echo "IG7  just verify (deterministic impact-scoped gate)"
if skipped IG7; then skip "IG7 skipped via --skip (run before pushing)"
elif [ ! -x "$REPO_ROOT/scripts/verify-impact.sh" ]; then
  warn "scripts/verify-impact.sh not found/executable — skipping IG7"
else
  if (cd "$REPO_ROOT" && ./scripts/verify-impact.sh --base "$BASE") >/tmp/goal-check.verify.log 2>&1; then
    ok "just verify passed — $(grep -m1 '^VERIFY OK ' /tmp/goal-check.verify.log || echo 'VERIFY OK line missing?')"
  else
    fail "just verify failed (see /tmp/goal-check.verify.log): $(grep -m1 '^VERIFY FAIL: ' /tmp/goal-check.verify.log || true)"
  fi
fi
echo

# -----------------------------------------------------------------------
# IG8 — archive move + backlog flip in this PR
# Pass when, in the BASE..HEAD diff, plans/<slug>.md was renamed to
# plans/_archive/<slug>.md AND backlog.json has the row for SLUG with
# status="done". Pre-final-commit, this check is expected to fail — that's
# why it's skippable.
# -----------------------------------------------------------------------
echo "IG8  archive move + backlog status=done in this PR"
if skipped IG8; then skip "IG8 skipped via --skip (pre-final-commit)"
else
  if ! git -C "$REPO_ROOT" rev-parse "$BASE" >/dev/null 2>&1; then
    fail "base '$BASE' not found — cannot verify archive move (HARD). Run: git fetch origin $BASE (or --skip IG8 pre-final-commit)"
  else
    renames="$(git -C "$REPO_ROOT" log "$BASE..HEAD" --diff-filter=R --name-status 2>/dev/null \
      | awk -v slug=".research/plans/$SLUG.md" -v arch=".research/plans/_archive/$SLUG.md" '
        $1 ~ /^R[0-9]+/ && $2 == slug && $3 == arch { print }')"
    if [ -n "$renames" ]; then
      ok "archive rename present in branch history"
    else
      fail "no rename of plans/$SLUG.md → plans/_archive/$SLUG.md on $BASE..HEAD"
    fi

    backlog="$REPO_ROOT/.research/backlog.json"
    if [ -f "$backlog" ]; then
      status="$(jq -r --arg slug "$SLUG" '.items[]? | select(.plan=="plans/\($slug).md" or .id==$slug) | .status' "$backlog" 2>/dev/null | head -1)"
      if [ "$status" = "done" ]; then
        ok "backlog.json row for $SLUG: status=done"
      elif [ -z "$status" ]; then
        warn "no backlog.json row references plans/$SLUG.md or id=$SLUG"
      else
        fail "backlog.json row for $SLUG: status=$status (expected done)"
      fi
    else
      warn "backlog.json not found at $backlog"
    fi
  fi
fi
echo

# -----------------------------------------------------------------------
# Summary
# -----------------------------------------------------------------------
if [ "$fail_count" -eq 0 ]; then
  echo "==> goal-check: PASS  (warnings=$warn_count)"
  exit 0
else
  echo "==> goal-check: FAIL  (fails=$fail_count warnings=$warn_count)"
  exit 1
fi
