#!/usr/bin/env bash
# verify-all.sh — run every skill's smoke check; exit non-zero on any failure.
#
# Each smoke check is expected to complete in <30s. Total ~5 min worst case.
# Run from the repo root. Captures per-skill pass/fail/skip.
#
# Usage: ./.claude/skills/verify-all.sh [--quick]
#   --quick    swap long-running smokes (cargo / pnpm / gradle / npx-tsp)
#              for cheap file-presence checks. Still covers all skills;
#              each prints PASS / SKIP / FAIL — never silent.

set -u

QUICK=0
[[ "${1:-}" == "--quick" ]] && QUICK=1

cd "$(git rev-parse --show-toplevel)" >/dev/null 2>&1 || {
  echo "FATAL: not inside a git repo" >&2
  exit 2
}

# Pick a timeout binary — `timeout` on Linux, `gtimeout` from coreutils on macOS.
# If neither is present, run without a timeout cap (warn once).
if command -v timeout >/dev/null 2>&1; then
  TIMEOUT=timeout
elif command -v gtimeout >/dev/null 2>&1; then
  TIMEOUT=gtimeout
else
  echo "  (note: no timeout/gtimeout on PATH — running smoke checks without time cap)"
  TIMEOUT=""
fi

PASS=0
FAIL=0
SKIPPED=0
FAIL_NAMES=()

run() {
  local name="$1"
  local timeout_s="$2"
  shift 2
  printf "  %-22s ... " "$name"
  local out rc
  if [[ -n "$TIMEOUT" ]]; then
    out=$("$TIMEOUT" "${timeout_s}s" bash -c "$*" 2>&1)
  else
    out=$(bash -c "$*" 2>&1)
  fi
  rc=$?
  if [[ $rc -eq 0 ]]; then
    echo "PASS"
    PASS=$((PASS + 1))
  else
    echo "FAIL (exit $rc)"
    if [[ -n "$out" ]]; then
      sed 's/^/      | /' <<<"$out" | head -5
    fi
    FAIL=$((FAIL + 1))
    FAIL_NAMES+=("$name")
  fi
}

skip() {
  local name="$1"
  local reason="$2"
  printf "  %-22s ... SKIP (%s)\n" "$name" "$reason"
  SKIPPED=$((SKIPPED + 1))
}

echo "== .claude/skills smoke checks =="

run "ppt-research-flow"   10 'test -d .research/plans/_archive && test -f .research/implementer-prompt.md && echo ok'
run "ppt-bridge-mcp"      15 'curl -fsS https://p.rlt.sk/healthz >/dev/null'
if [[ $QUICK -eq 1 ]]; then
  # File-presence fallbacks: confirm the skill points at real workspace paths
  # without spinning up the toolchain (just / gh / cargo / pnpm / gradle /
  # npx-tsp / stack). Suitable for cold caches and offline CI runners.
  run "ppt-tests"         10 'test -f justfile && grep -qE "^test(-(backend|frontend|integration))?:" justfile'
  run "ppt-pr-create"     10 'test -d .github && test -f .github/workflows/ci.yml'
  run "ppt-rust-backend"  10 'test -f backend/Cargo.toml && test -d backend/crates'
  run "ppt-frontend"      10 'test -f frontend/pnpm-workspace.yaml || test -f frontend/package.json'
  run "ppt-mobile-native" 10 'test -f mobile-native/build.gradle.kts || test -f mobile-native/settings.gradle.kts'
  run "ppt-typespec"      10 'test -f docs/api/typespec/main.tsp'
  run "ppt-dev-stack"     10 'test -f .claude/skills/ppt-dev-stack/SKILL.md'
else
  run "ppt-tests"          10 'just --list 2>/dev/null | grep -qE "^\s+(test-backend|test-frontend|test-integration)\b"'
  run "ppt-pr-create"      10 'gh auth status >/dev/null 2>&1 && echo ok'
  run "ppt-rust-backend"  300 'cd backend && cargo check --workspace --message-format=short >/dev/null'
  run "ppt-frontend"       30 'cd frontend && pnpm -r list --depth -1 --json >/dev/null 2>&1'
  run "ppt-mobile-native"  60 'cd mobile-native && ./gradlew help -q >/dev/null 2>&1'
  run "ppt-typespec"       20 'cd docs/api/typespec && npx --no-install tsp --version >/dev/null 2>&1'
  run "ppt-dev-stack"      10 'stack list 2>/dev/null | grep -qE "(^|\s)pm-local(\s|$)"'
fi
run "ppt-db-migrations"   10 'test -d backend/crates/db/migrations && test -d backend/servers/deploy-server/migrations && test -f backend/crates/db/src/seed/runner.rs'
# ppt-deploy predates the research scaffold; we just verify the skill files
# are wired up. Live pmctl / onyx checks are out of scope for a generic harness.
run "ppt-deploy"          10 'test -f .claude/skills/ppt-deploy/SKILL.md && test -d .claude/skills/ppt-deploy/commands && test -d .claude/skills/ppt-deploy/references'

echo
echo "== summary =="
echo "passed: $PASS"
echo "skipped: $SKIPPED"
echo "failed: $FAIL"
if [[ $FAIL -gt 0 ]]; then
  echo "failed skills: ${FAIL_NAMES[*]}"
  exit 1
fi
exit 0
