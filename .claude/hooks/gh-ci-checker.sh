#!/bin/bash
#
# gh-ci-checker.sh - PostToolUse hook for GitHub CI status
#
# Triggered after Bash commands, checks if it was a gh/git command
# and announces CI status via TTS.
#
# Hook Event: PostToolUse (matcher: Bash)
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

# Check if this is a relevant GitHub/git command
if [[ ! "$TOOL_INPUT" =~ ^(gh\ pr|gh\ run|git\ push) ]]; then
    exit 0
fi

# Wait a moment for GitHub to register the action
sleep 2

# Get current PR checks (suppress errors if no PR)
CHECKS=$(gh pr checks 2>/dev/null || echo "")

if [[ -z "$CHECKS" ]]; then
    # No PR associated with current branch
    exit 0
fi

# Count check statuses
PASS=$(echo "$CHECKS" | grep -c "pass" || echo "0")
FAIL=$(echo "$CHECKS" | grep -c "fail" || echo "0")
PENDING=$(echo "$CHECKS" | grep -c -E "(pending|queued|in_progress)" || echo "0")

# Announce via TTS if the script exists
if [[ -x "$TTS_SCRIPT" ]]; then
    if [[ "$PENDING" -gt 0 ]]; then
        "$TTS_SCRIPT" "CI running, $PENDING checks pending"
    elif [[ "$FAIL" -gt 0 ]]; then
        "$TTS_SCRIPT" "CI has $FAIL failures"
    elif [[ "$PASS" -gt 0 ]]; then
        "$TTS_SCRIPT" "All $PASS CI checks passed"
    fi
fi

# Output JSON for Claude (optional context)
cat << EOF
{
    "ci_status": {
        "passed": $PASS,
        "failed": $FAIL,
        "pending": $PENDING
    }
}
EOF

exit 0
