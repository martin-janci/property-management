#!/usr/bin/env bash
# Code-reuse pre-flight — grep the codebase for existing helpers whose names
# look similar to newly-added symbols in the implementer's diff.
#
# Advisory, not blocking. Stdout = WARN lines; exit code is always 0.
#
# Usage:
#   reuse-grep.sh --specialist <name> [--base dev] [--head HEAD]
#
# Specialist drives which symbol patterns + which search roots apply.

set -euo pipefail

SPECIALIST=""
BASE="dev"
HEAD_REF="HEAD"

while [ $# -gt 0 ]; do
  case "$1" in
    --specialist) SPECIALIST="$2"; shift 2;;
    --base)       BASE="$2"; shift 2;;
    --head)       HEAD_REF="$2"; shift 2;;
    *) echo "unknown arg: $1" >&2; exit 64;;
  esac
done

if [ -z "$SPECIALIST" ]; then
  echo "error: --specialist required" >&2
  exit 64
fi

# Pick the smallest possible diff range.
if MB=$(git merge-base "$BASE" "$HEAD_REF" 2>/dev/null); then
  DIFF=$(git diff --unified=0 "$MB..$HEAD_REF")
else
  DIFF=$(git diff --unified=0 "$BASE..$HEAD_REF" 2>/dev/null || true)
fi

# Get the added-line text (skip diff metadata).
ADDED=$(printf '%s\n' "$DIFF" | grep -E '^\+' | grep -vE '^\+\+\+' || true)

case "$SPECIALIST" in
  rust-backend|db-migration)
    # Patterns: new pub fn / struct / static / OnceLock / OnceCell that may
    # duplicate something already in api-core or auth crate.
    PATTERNS='(static\s+[A-Z_]+:\s*OnceLock|static\s+[A-Z_]+:\s*OnceCell|pub\s+fn\s+(validate_|verify_|build_|new_)[a-z_]+|pub\s+struct\s+[A-Z][A-Za-z]*Verifier)'
    SEARCH_ROOTS=(backend/crates/api-core/src backend/crates/auth/src backend/crates/db/src)
    ;;
  react-web|nextjs-web|react-native)
    PATTERNS='(export\s+(const|function)\s+(use[A-Z]|create[A-Z]|make[A-Z]))'
    SEARCH_ROOTS=(frontend/packages frontend/apps)
    ;;
  *)
    # No reuse heuristic for this specialist — exit clean.
    exit 0
    ;;
esac

NEW_SYMS=$(printf '%s\n' "$ADDED" \
  | grep -oE "$PATTERNS" 2>/dev/null \
  | grep -oE '[A-Za-z_][A-Za-z0-9_]+' \
  | sort -u || true)

if [ -z "$NEW_SYMS" ]; then
  exit 0
fi

# Skip generic keywords that always trip the pattern.
SKIP_RE='^(pub|fn|use|new|const|let|mut|struct|export|function|create|make|validate|verify|build|static|OnceLock|OnceCell|Verifier)$'

# For each new symbol, see if it already exists in the search roots
# (outside of the implementer's own diff).
if ! command -v rg >/dev/null 2>&1; then
  # No ripgrep -> fall back to grep -rn. Slower; still works.
  GREP_CMD="grep -rn --include=*.rs --include=*.ts --include=*.tsx"
else
  GREP_CMD="rg -n"
fi

EXISTING_ROOTS=()
for r in "${SEARCH_ROOTS[@]}"; do
  [ -d "$r" ] && EXISTING_ROOTS+=("$r")
done

# If no search root exists, exit clean.
if [ ${#EXISTING_ROOTS[@]} -eq 0 ]; then
  exit 0
fi

while IFS= read -r sym; do
  [ -z "$sym" ] && continue
  if echo "$sym" | grep -Eq "$SKIP_RE"; then continue; fi
  if hit=$($GREP_CMD -F "$sym" "${EXISTING_ROOTS[@]}" 2>/dev/null | head -1); then
    if [ -n "$hit" ]; then
      path=$(printf '%s' "$hit" | cut -d: -f1)
      echo "WARN: $sym looks like an existing helper at $path"
    fi
  fi
done <<< "$NEW_SYMS"

exit 0
