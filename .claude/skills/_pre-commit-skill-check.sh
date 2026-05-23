#!/usr/bin/env bash
# _pre-commit-skill-check.sh — run as a git pre-commit hook to catch
# broken skill files BEFORE they land on a branch.
#
# Behaviour:
#   - If no staged files touch .claude/skills/ → exit 0 (no-op).
#   - Else: for each touched skill dir, run _verify-skill.sh.
#     If ANY fail → block the commit with a clear error.
#
# Install (adds to the existing pre-commit hook, does NOT replace it):
#   cat >> .git/hooks/pre-commit <<'HOOK'
#   bash .claude/skills/_pre-commit-skill-check.sh || exit $?
#   HOOK
#
# Or call standalone:
#   bash .claude/skills/_pre-commit-skill-check.sh

set -u

cd "$(git rev-parse --show-toplevel)" >/dev/null 2>&1 || exit 0

# Only consider staged additions / modifications (skip deletions).
STAGED=$(git diff --cached --name-only --diff-filter=ACM | grep -E '^\.claude/skills/[^/]+/(SKILL\.md|install\.sh|agents/[^/]+\.md)$' || true)

if [[ -z "$STAGED" ]]; then
  exit 0
fi

# Collect unique skill dirs from staged paths.
SKILL_DIRS=$(echo "$STAGED" | awk -F/ '{print ".claude/skills/" $3}' | sort -u)

VERIFY=".claude/skills/_verify-skill.sh"
if [[ ! -x "$VERIFY" ]]; then
  echo "WARN: $VERIFY missing or not executable; skipping skill checks" >&2
  exit 0
fi

FAILS=0
echo "Pre-commit: validating skill changes..."
for dir in $SKILL_DIRS; do
  if [[ ! -d "$dir" ]]; then
    continue
  fi
  if ! bash "$VERIFY" "$dir"; then
    FAILS=$((FAILS + 1))
  fi
done

if [[ $FAILS -gt 0 ]]; then
  echo "" >&2
  echo "Pre-commit blocked: $FAILS skill(s) failed validation." >&2
  echo "Fix the issues above (or run \`bash .claude/skills/_verify-skill.sh <dir>\` to retest)." >&2
  exit 1
fi

echo "Pre-commit: $(echo "$SKILL_DIRS" | wc -l) skill(s) validated · OK"
exit 0
