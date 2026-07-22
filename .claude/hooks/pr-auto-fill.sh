#!/bin/bash
#
# pr-auto-fill.sh - PostToolUse hook to auto-fill PR descriptions
#
# Triggered after `gh pr create` commands
# Extracts epic/story/UC context from documentation
#
# Hook Event: PostToolUse (matcher: Bash)
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$(dirname "$SCRIPT_DIR")")"
TTS_SCRIPT="$SCRIPT_DIR/play-tts.sh"
EPICS_FILE="$PROJECT_DIR/_bmad-output/epics.md"
USE_CASES_FILE="$PROJECT_DIR/docs/use-cases.md"

# Read JSON input from stdin
INPUT=$(cat)

# Parse tool name and command
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // ""')
TOOL_INPUT=$(echo "$INPUT" | jq -r '.tool_input.command // ""')

# Only process Bash tool calls with gh pr create
if [[ "$TOOL_NAME" != "Bash" ]]; then
    exit 0
fi

if [[ ! "$TOOL_INPUT" =~ gh\ pr\ create ]]; then
    exit 0
fi

# Get current branch and extract epic number
BRANCH=$(git branch --show-current 2>/dev/null || echo "")
EPIC_NUM=""

# Try to extract epic number from branch name
# Patterns: feature/epic-5-voting, feature/EPIC-5, feature/epic-2a-orgs
if [[ "$BRANCH" =~ [Ee]pic[-_]?([0-9]+[AaBb]?) ]]; then
    EPIC_NUM="${BASH_REMATCH[1]}"
fi

if [[ -z "$EPIC_NUM" ]]; then
    # Announce that we couldn't detect epic
    if [[ -x "$TTS_SCRIPT" ]]; then
        "$TTS_SCRIPT" "PR created. No epic detected from branch name."
    fi
    exit 0
fi

# Normalize epic number (uppercase for letters)
EPIC_NUM=$(echo "$EPIC_NUM" | tr '[:lower:]' '[:upper:]')

# Wait for PR to be created
sleep 2

# Get PR number
PR_NUM=$(gh pr view --json number -q '.number' 2>/dev/null || echo "")

if [[ -z "$PR_NUM" ]]; then
    exit 0
fi

# Check if epics file exists
if [[ ! -f "$EPICS_FILE" ]]; then
    if [[ -x "$TTS_SCRIPT" ]]; then
        "$TTS_SCRIPT" "PR $PR_NUM created. Epics file not found."
    fi
    exit 0
fi

# Extract epic section from epics.md
# Look for ## Epic N: or ## Epic NA:
EPIC_SECTION=$(awk "/^## Epic $EPIC_NUM:/,/^## Epic [0-9]/" "$EPICS_FILE" | head -n -1)

if [[ -z "$EPIC_SECTION" ]]; then
    # Try with different formats
    EPIC_SECTION=$(awk "/^## Epic ${EPIC_NUM}[^0-9]/,/^## Epic [0-9]/" "$EPICS_FILE" | head -n -1)
fi

if [[ -z "$EPIC_SECTION" ]]; then
    if [[ -x "$TTS_SCRIPT" ]]; then
        "$TTS_SCRIPT" "PR $PR_NUM created. Epic $EPIC_NUM not found in documentation."
    fi
    exit 0
fi

# Extract key information
EPIC_TITLE=$(echo "$EPIC_SECTION" | grep -m1 "^## Epic" | sed 's/^## //')
EPIC_GOAL=$(echo "$EPIC_SECTION" | grep -A1 "^\*\*Goal:\*\*" | tail -1 | sed 's/^[[:space:]]*//')
FRS_COVERED=$(echo "$EPIC_SECTION" | grep -A1 "^\*\*FRs covered:\*\*" | tail -1 | sed 's/^[[:space:]]*//')

# Extract stories (look for ### Story patterns)
STORIES=$(echo "$EPIC_SECTION" | grep "^### Story" | sed 's/^### /- /')

# Count stories
STORY_COUNT=$(echo "$STORIES" | grep -c "^-" || echo "0")

# Get current PR body
CURRENT_BODY=$(gh pr view "$PR_NUM" --json body -q '.body' 2>/dev/null || echo "")

# Generate new PR body
NEW_BODY="## Summary
${EPIC_GOAL:-Implementation of $EPIC_TITLE}

## $EPIC_TITLE

**Goal:** ${EPIC_GOAL:-See epic documentation}

**FRs Covered:** ${FRS_COVERED:-See documentation}

## Stories Implemented ($STORY_COUNT stories)
$STORIES

## Changes
$(git diff main...HEAD --stat 2>/dev/null | tail -5)

---
*Auto-generated from \`_bmad-output/epics.md\`*"

# Update PR body
gh pr edit "$PR_NUM" --body "$NEW_BODY" 2>/dev/null

# Announce via TTS
if [[ -x "$TTS_SCRIPT" ]]; then
    "$TTS_SCRIPT" "PR $PR_NUM filled with Epic $EPIC_NUM context. $STORY_COUNT stories documented."
fi

# Output result
cat << EOF
{
    "pr_filled": true,
    "pr_number": $PR_NUM,
    "epic": "$EPIC_NUM",
    "stories_count": $STORY_COUNT
}
EOF

exit 0
