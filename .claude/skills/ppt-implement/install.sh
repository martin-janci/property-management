#!/usr/bin/env bash
# install.sh — copy the ppt-implement skill into ~/.claude/skills/
#
# Usage:  bash .claude/skills/ppt-implement/install.sh
#
# Copy semantics (not symlink) — local edits in ~/.claude/skills/ppt-implement/
# do NOT propagate back to the git repo. Re-run this script after pulling
# updates to refresh the local copy.

set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
TARGET_PARENT="$HOME/.claude/skills"
TARGET="$TARGET_PARENT/ppt-implement"

mkdir -p "$TARGET_PARENT"

if [[ -L "$TARGET" ]]; then
  echo "Found symlink at $TARGET — removing"
  rm "$TARGET"
elif [[ -d "$TARGET" ]]; then
  echo "Found existing directory at $TARGET — replacing"
  rm -rf "$TARGET"
fi

cp -r "$SCRIPT_DIR" "$TARGET"

# Strip the install script from the installed copy — it's only meaningful in the repo.
rm -f "$TARGET/install.sh"

echo
echo "Installed ppt-implement skill to: $TARGET"
echo
echo "Contents:"
ls -la "$TARGET"
echo
echo "To verify it loads, run: claude --list-skills | grep ppt-implement"
