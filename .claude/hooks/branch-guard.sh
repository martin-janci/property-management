#!/bin/bash
#
# branch-guard.sh - PreToolUse hook to prevent dangerous git operations
#
# Blocks force pushes to main/master and warns about direct commits.
#
# Hook Event: PreToolUse (matcher: Bash)
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TTS_SCRIPT="$SCRIPT_DIR/play-tts.sh"

# Read JSON input from stdin
INPUT=$(cat)

# Parse tool name and command
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // ""')
TOOL_INPUT=$(echo "$INPUT" | jq -r '.tool_input.command // ""')

# Only process Bash tool calls
if [[ "$TOOL_NAME" != "Bash" ]]; then
    exit 0
fi

# Get current branch
BRANCH=$(git branch --show-current 2>/dev/null || echo "")

# Check for dangerous operations

# 1. Block force push to main/master
if [[ "$TOOL_INPUT" =~ git\ push.*--force ]] || [[ "$TOOL_INPUT" =~ git\ push.*-f ]]; then
    if [[ "$TOOL_INPUT" =~ (main|master) ]] || [[ "$BRANCH" == "main" ]] || [[ "$BRANCH" == "master" ]]; then
        if [[ -x "$TTS_SCRIPT" ]]; then
            "$TTS_SCRIPT" "Blocked! Force push to main is not allowed."
        fi
        cat << EOF
{
    "decision": "block",
    "reason": "Force push to main/master branch is blocked for safety. Use a feature branch and PR instead."
}
EOF
        exit 0
    fi
fi

# 2. Block hard reset on main/master
if [[ "$TOOL_INPUT" =~ git\ reset\ --hard ]]; then
    if [[ "$BRANCH" == "main" ]] || [[ "$BRANCH" == "master" ]]; then
        if [[ -x "$TTS_SCRIPT" ]]; then
            "$TTS_SCRIPT" "Blocked! Hard reset on main is dangerous."
        fi
        cat << EOF
{
    "decision": "block",
    "reason": "Hard reset on main/master branch is blocked. This operation is destructive and irreversible."
}
EOF
        exit 0
    fi
fi

# 3. Warn about direct commits to main (but allow)
if [[ "$TOOL_INPUT" =~ git\ commit ]] && [[ ! "$TOOL_INPUT" =~ --amend ]]; then
    if [[ "$BRANCH" == "main" ]] || [[ "$BRANCH" == "master" ]]; then
        if [[ -x "$TTS_SCRIPT" ]]; then
            "$TTS_SCRIPT" "Warning: Committing directly to main. Consider using a feature branch."
        fi
        # Allow but warn
        cat << EOF
{
    "warning": "Direct commit to main branch. Consider using feature branches for better collaboration."
}
EOF
    fi
fi

# 4. Block rebase of main
if [[ "$TOOL_INPUT" =~ git\ rebase ]]; then
    if [[ "$BRANCH" == "main" ]] || [[ "$BRANCH" == "master" ]]; then
        if [[ -x "$TTS_SCRIPT" ]]; then
            "$TTS_SCRIPT" "Blocked! Rebasing main is not allowed."
        fi
        cat << EOF
{
    "decision": "block",
    "reason": "Rebasing main/master branch is blocked. This can cause issues for other contributors."
}
EOF
        exit 0
    fi
fi

# Allow operation
exit 0
