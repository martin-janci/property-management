#!/usr/bin/env bash
# install.sh — copy the ppt-project-management skill into ~/.claude/skills/
set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
TARGET_PARENT="$HOME/.claude/skills"
TARGET="$TARGET_PARENT/ppt-project-management"

VERIFY="$(dirname "$SCRIPT_DIR")/_verify-skill.sh"
if [[ -x "$VERIFY" ]]; then
  bash "$VERIFY" "$SCRIPT_DIR" || { echo "Refusing to install ppt-project-management: validation failed"; exit 1; }
fi

mkdir -p "$TARGET_PARENT"

if [[ -L "$TARGET" ]]; then
  echo "Found symlink at $TARGET — removing"
  rm "$TARGET"
elif [[ -d "$TARGET" ]]; then
  echo "Found existing directory at $TARGET — replacing"
  rm -rf "$TARGET"
fi

cp -r "$SCRIPT_DIR" "$TARGET"
rm -f "$TARGET/install.sh"

echo
echo "Installed ppt-project-management skill to: $TARGET"
ls -la "$TARGET"
