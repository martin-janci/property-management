#!/usr/bin/env bash
# Deterministic scope-drift check for the ppt-implement skill.
#
# Compares `git diff --name-only <base>..<head>` against the expected
# file-pathspec list for the implementer's owner_role
# (`scripts/owner-areas.json`).
#
# Exit codes:
#   0 = clean (all changed paths inside the owner's areas)
#   1 = unknown owner_role (cannot judge — log + continue)
#   2 = scope drift (changed paths outside the owner's areas — list to stdout)
#
# Usage:
#   scope-check.sh --owner <owner_role> [--base dev] [--head HEAD]
#
# Reads owner-areas.json from the same directory.

set -euo pipefail

OWNER=""
BASE="dev"
HEAD_REF="HEAD"

while [ $# -gt 0 ]; do
  case "$1" in
    --owner) OWNER="$2"; shift 2;;
    --base)  BASE="$2"; shift 2;;
    --head)  HEAD_REF="$2"; shift 2;;
    *) echo "unknown arg: $1" >&2; exit 64;;
  esac
done

if [ -z "$OWNER" ]; then
  echo "error: --owner required" >&2
  exit 64
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
AREAS_FILE="$SCRIPT_DIR/owner-areas.json"

if ! command -v jq >/dev/null 2>&1; then
  echo "error: jq is required" >&2
  exit 64
fi
if [ ! -f "$AREAS_FILE" ]; then
  echo "error: missing $AREAS_FILE" >&2
  exit 64
fi

# Lookup the owner's pattern list. `null` if the owner isn't mapped.
PATTERNS=$(jq -r --arg o "$OWNER" '.[$o] // empty | .[]?' "$AREAS_FILE")
if [ -z "$PATTERNS" ]; then
  echo "scope-check: unknown owner_role '$OWNER' (not in owner-areas.json)" >&2
  exit 1
fi

# Find the merge-base so the diff matches what GH PR diff would show.
# Fall back to plain <base>..<head> if no common ancestor (shallow clones).
if MB=$(git merge-base "$BASE" "$HEAD_REF" 2>/dev/null); then
  CHANGED=$(git diff --name-only "$MB..$HEAD_REF")
else
  CHANGED=$(git diff --name-only "$BASE..$HEAD_REF" 2>/dev/null || true)
fi

if [ -z "$CHANGED" ]; then
  exit 0
fi

# Convert pattern lines to a single egrep-style alternation. The patterns
# use git-pathspec syntax (`**`, `*`). Translate to regex:
#   **  -> .*
#   *   -> [^/]*
#   .   -> \.
#   /   -> /
# Anchor each pattern with ^...$ so we don't accidentally match a substring.
to_regex() {
  printf '%s' "$1" \
    | sed -E 's#\.#\\.#g' \
    | sed -E 's#\*\*#__GLOBSTAR__#g' \
    | sed -E 's#\*#[^/]*#g' \
    | sed -E 's#__GLOBSTAR__#.*#g'
}

ALT=""
while IFS= read -r pat; do
  [ -z "$pat" ] && continue
  rx=$(to_regex "$pat")
  if [ -z "$ALT" ]; then ALT="^${rx}$"; else ALT="${ALT}|^${rx}$"; fi
done <<< "$PATTERNS"

DRIFT=$(echo "$CHANGED" | grep -Ev "$ALT" || true)

if [ -z "$DRIFT" ]; then
  exit 0
fi

# Drift detected — print paths to stdout (one per line), exit 2.
echo "$DRIFT"
exit 2
