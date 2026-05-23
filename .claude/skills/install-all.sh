#!/usr/bin/env bash
# install-all.sh — install every PPT skill into ~/.claude/skills/
#
# For each skill directory under .claude/skills/ that has an install.sh:
#   1. Run _verify-skill.sh on it (fail-fast — skips broken skills)
#   2. Run its install.sh (copies into ~/.claude/skills/<name>/)
#
# Usage:
#   bash .claude/skills/install-all.sh           # install everything that passes verify
#   bash .claude/skills/install-all.sh --dry-run # show what would be installed
#   bash .claude/skills/install-all.sh ppt-implement ppt-pr-merge  # install only listed

set -u

cd "$(git rev-parse --show-toplevel)" >/dev/null 2>&1 || {
  echo "FATAL: not inside a git repo" >&2
  exit 2
}

DRY=0
FILTER=()
for arg in "$@"; do
  case "$arg" in
    --dry-run) DRY=1 ;;
    *)         FILTER+=("$arg") ;;
  esac
done

SKILLS_DIR=".claude/skills"
if [[ ! -d "$SKILLS_DIR" ]]; then
  echo "FATAL: $SKILLS_DIR not found" >&2
  exit 2
fi

VERIFY="$SKILLS_DIR/_verify-skill.sh"
if [[ ! -x "$VERIFY" ]]; then
  echo "FATAL: $VERIFY missing or not executable" >&2
  exit 2
fi

INSTALLED=0
SKIPPED=0
FAILED=0
FAIL_NAMES=()

for dir in "$SKILLS_DIR"/*/; do
  name="$(basename "$dir")"

  # Apply filter if given
  if [[ ${#FILTER[@]} -gt 0 ]]; then
    found=0
    for f in "${FILTER[@]}"; do
      [[ "$f" == "$name" ]] && found=1 && break
    done
    [[ $found -eq 0 ]] && continue
  fi

  # Must have an install.sh
  if [[ ! -f "$dir/install.sh" ]]; then
    echo "  -- skip $name (no install.sh)"
    SKIPPED=$((SKIPPED + 1))
    continue
  fi

  echo "  >> $name"

  # Validate first
  if ! bash "$VERIFY" "$dir" >/dev/null 2>&1; then
    # Re-run with output to surface the actual error
    bash "$VERIFY" "$dir" || true
    FAIL_NAMES+=("$name")
    FAILED=$((FAILED + 1))
    continue
  fi

  if [[ $DRY -eq 1 ]]; then
    echo "     (dry-run: would install)"
    INSTALLED=$((INSTALLED + 1))
    continue
  fi

  if bash "$dir/install.sh" >/dev/null 2>&1; then
    INSTALLED=$((INSTALLED + 1))
  else
    bash "$dir/install.sh" || true
    FAIL_NAMES+=("$name")
    FAILED=$((FAILED + 1))
  fi
done

echo
echo "Installed: $INSTALLED · Skipped: $SKIPPED · Failed: $FAILED"
if [[ $FAILED -gt 0 ]]; then
  echo "Failures: ${FAIL_NAMES[*]}"
  exit 1
fi
