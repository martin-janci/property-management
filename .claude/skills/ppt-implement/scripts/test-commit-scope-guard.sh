#!/usr/bin/env bash
# Fixture tests for commit-scope-guard.sh — exercise the allow-list
# matching logic against the PR #496 scenario (dispatcher stop hook
# auto-committing parallel-agent work into an ADR-008 doc edit).
#
# Each fixture sets up a fresh ephemeral git repo, stages a known set
# of paths, then invokes the guard with a specific --allow pattern
# list (or --owner) and asserts the expected exit code.
#
# Run from anywhere — the script `cd`s into a tempdir for each case.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GUARD="$SCRIPT_DIR/commit-scope-guard.sh"

if [ ! -x "$GUARD" ]; then
  echo "missing or non-executable: $GUARD" >&2
  exit 1
fi

failures=0

run_case() {
  local label="$1"; shift
  local expected_exit="$1"; shift
  local files_csv="$1"; shift   # comma-separated list of paths to touch + stage
  # Remaining args are passed verbatim to the guard.

  local tmp
  tmp=$(mktemp -d)
  (
    cd "$tmp"
    git init -q
    git config user.email t@t
    git config user.name t
    git commit --allow-empty -q -m init

    IFS=',' read -r -a paths <<< "$files_csv"
    for p in "${paths[@]}"; do
      mkdir -p "$(dirname "$p")"
      : > "$p"
      git add "$p"
    done

    set +e
    "$GUARD" "$@" >/dev/null 2>&1
    local rc=$?
    set -e
    if [ "$rc" -ne "$expected_exit" ]; then
      printf 'FAIL: %s — expected exit=%s got=%s\n' "$label" "$expected_exit" "$rc" >&2
      exit 1
    fi
    printf 'ok:   %s (exit=%s)\n' "$label" "$rc"
  ) || failures=$((failures + 1))
  rm -rf "$tmp"
}

# PR #496 fixture (the regression we're guarding against): an ADR doc
# edit branch with the dispatcher's own management files also touched,
# plus parallel agents' mobile env-config + iOS xcscheme changes that
# don't belong on this branch.
PR496_FILES="docs/architecture.md,.research/management/action-list.json,frontend/apps/mobile/app.config.ts,mobile-native/iosApp/iosApp/Resources/Info.plist"

# Case 1: ADR-edit scope — only allow architecture doc + dispatcher
# management dir. Expect REFUSE (exit 2) because mobile + iOS files
# are out of scope.
run_case \
  "PR #496 ADR-edit scope refuses parallel-agent files" \
  2 \
  "$PR496_FILES" \
  --allow 'docs/architecture.md' \
  --allow '.research/management/**'

# Case 2: Same files, but the allow-list is broad enough to cover
# everything → ACCEPT (exit 0). This simulates an explicit, declared
# multi-area task (rare; usually wrong, but legal).
run_case \
  "broad allow-list accepts all staged paths" \
  0 \
  "$PR496_FILES" \
  --allow 'docs/**' \
  --allow '.research/**' \
  --allow 'frontend/**' \
  --allow 'mobile-native/**'

# Case 3: In-scope only — pure ADR edit, no parallel work bled in.
run_case \
  "pure ADR-edit accepts" \
  0 \
  "docs/architecture.md,.research/management/action-list.json" \
  --allow 'docs/architecture.md' \
  --allow '.research/management/**'

# Case 4: Empty staged set is always a no-op pass.
run_case \
  "empty staged set passes" \
  0 \
  "" \
  --allow 'docs/architecture.md'

# Case 5: Unknown owner_role fails closed (exit 1).
run_case \
  "unknown owner_role fails closed" \
  1 \
  "docs/architecture.md" \
  --owner "totally-fake-role-name"

if [ "$failures" -gt 0 ]; then
  printf '\n%s case(s) failed\n' "$failures" >&2
  exit 1
fi
printf '\nAll commit-scope-guard fixtures passed.\n'
