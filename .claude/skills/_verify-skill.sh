#!/usr/bin/env bash
# _verify-skill.sh — validate one skill directory's structure.
#
# Checks:
#   1. SKILL.md exists and starts with `---` frontmatter
#   2. Frontmatter has required fields: name, description, when_to_use, mode
#   3. `name` in frontmatter matches the directory basename
#   4. install.sh (if present) is executable and #!/usr/bin/env bash
#   5. If SKILL.md references `agents/<name>.md`, those files exist
#   6. No tabs in SKILL.md (markdown convention)
#
# Usage:
#   bash .claude/skills/_verify-skill.sh <path-to-skill-dir>
#
# Exit codes: 0 = OK, 1 = validation failure, 2 = bad invocation.

set -u

if [[ $# -lt 1 ]]; then
  echo "usage: $0 <skill-dir>" >&2
  exit 2
fi

SKILL_DIR="${1%/}"
SKILL_NAME="$(basename "$SKILL_DIR")"
SKILL_MD="$SKILL_DIR/SKILL.md"
FAILS=0

fail() {
  printf "  FAIL %s — %s\n" "$SKILL_NAME" "$1" >&2
  FAILS=$((FAILS + 1))
}

if [[ ! -d "$SKILL_DIR" ]]; then
  fail "directory does not exist: $SKILL_DIR"
  exit 1
fi

# --- 1. SKILL.md presence + frontmatter delimiter
if [[ ! -f "$SKILL_MD" ]]; then
  fail "SKILL.md missing"
  exit 1
fi

if ! head -1 "$SKILL_MD" | grep -qx -- '---'; then
  fail "SKILL.md must start with '---' frontmatter delimiter"
fi

# --- 2. Required frontmatter fields
FM=$(awk '/^---$/{c++; next} c==1{print}' "$SKILL_MD")
for field in name description when_to_use mode; do
  if ! printf '%s\n' "$FM" | grep -qE "^${field}:"; then
    fail "frontmatter missing required field: $field"
  fi
done

# --- 3. name matches directory
FM_NAME=$(printf '%s\n' "$FM" | awk -F': *' '/^name:/{print $2; exit}' | tr -d '[:space:]')
if [[ -n "$FM_NAME" && "$FM_NAME" != "$SKILL_NAME" ]]; then
  fail "frontmatter name '$FM_NAME' does not match directory basename '$SKILL_NAME'"
fi

# --- 4. install.sh sanity
if [[ -f "$SKILL_DIR/install.sh" ]]; then
  if [[ ! -x "$SKILL_DIR/install.sh" ]]; then
    fail "install.sh is not executable (chmod +x)"
  fi
  if ! head -1 "$SKILL_DIR/install.sh" | grep -qE '^#!/usr/bin/env (bash|sh)$|^#!/bin/(ba)?sh'; then
    fail "install.sh missing or wrong shebang"
  fi
fi

# --- 5. agents/*.md references in SKILL.md must exist on disk
# Look for patterns like `agents/<name>.md` mentioned in SKILL.md body.
REFERENCED_AGENTS=$(grep -oE 'agents/[a-z0-9_-]+\.md' "$SKILL_MD" 2>/dev/null | sort -u)
if [[ -n "$REFERENCED_AGENTS" ]]; then
  for ref in $REFERENCED_AGENTS; do
    if [[ ! -f "$SKILL_DIR/$ref" ]]; then
      fail "SKILL.md references $ref but file does not exist"
    fi
  done
fi

# --- 6. No tabs in SKILL.md (use spaces; matches existing skill style)
if grep -qP '\t' "$SKILL_MD"; then
  fail "SKILL.md contains tabs (use spaces only)"
fi

if [[ $FAILS -gt 0 ]]; then
  echo "  $SKILL_NAME: $FAILS validation error(s)" >&2
  exit 1
fi

echo "  OK $SKILL_NAME"
exit 0
