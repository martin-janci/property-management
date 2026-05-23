#!/usr/bin/env bash
# install.sh — copy the ppt-dev-team skill into ~/.claude/skills/
set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
TARGET_PARENT="$HOME/.claude/skills"
TARGET="$TARGET_PARENT/ppt-dev-team"

VERIFY="$(dirname "$SCRIPT_DIR")/_verify-skill.sh"
if [[ -x "$VERIFY" ]]; then
  bash "$VERIFY" "$SCRIPT_DIR" || { echo "Refusing to install ppt-dev-team: validation failed"; exit 1; }
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
echo "Installed ppt-dev-team skill to: $TARGET"
ls -la "$TARGET"
