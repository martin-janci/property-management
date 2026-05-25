#!/usr/bin/env bash
# Unit test for scope-check.sh.
#
# Stages synthetic file lists via a temporary git repo and asserts the
# expected exit codes + drift lists.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCOPE_CHECK="$SCRIPT_DIR/scope-check.sh"

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

cd "$TMP"
git init -q
git config user.email "test@example.com"
git config user.name  "test"
git commit --allow-empty -q -m "init"
git checkout -q -b dev
git commit --allow-empty -q -m "dev base"

# Mirror the script's owner-areas.json into the temp repo's lookup path.
mkdir -p "$TMP/scripts"
cp "$SCRIPT_DIR/owner-areas.json" "$TMP/scripts/owner-areas.json"

FAIL=0
ok()   { printf '  ok    %s\n' "$1"; }
bad()  { printf '  FAIL  %s\n' "$1" >&2; FAIL=1; }

run_case() {
  local name="$1"; shift
  local owner="$1"; shift
  local expect_exit="$1"; shift
  local files=("$@")
  local branch="case-$(echo "$name" | tr ' /' '-' | tr '[:upper:]' '[:lower:]')"

  git checkout -q dev
  git checkout -q -b "$branch"
  for f in "${files[@]}"; do
    mkdir -p "$(dirname "$f")"
    echo "test" > "$f"
    git add "$f"
  done
  git commit -q -m "$name"

  set +e
  out=$(bash "$SCOPE_CHECK" --owner "$owner" --base dev --head HEAD 2>/dev/null)
  code=$?
  set -e

  if [ "$code" = "$expect_exit" ]; then
    ok "$name (exit=$code as expected)"
  else
    bad "$name (exit=$code, expected $expect_exit; out=$out)"
  fi
}

echo "==> scope-check unit tests"
# Override the script's lookup path so it finds our copied owner-areas.json.
# scope-check.sh resolves it via dirname of its own path — so we copy the
# real script into place too.
cp "$SCRIPT_DIR/scope-check.sh"  "$TMP/scripts/scope-check.sh"
cp "$SCRIPT_DIR/owner-areas.json" "$TMP/scripts/owner-areas.json"
SCOPE_CHECK="$TMP/scripts/scope-check.sh"

run_case "pm-backend stays in backend"          pm-backend  0  backend/servers/api-server/src/foo.rs
run_case "pm-backend drifts to frontend"        pm-backend  2  backend/servers/api-server/src/foo.rs frontend/apps/ppt-web/src/bar.tsx
run_case "pm-frontend stays in ppt-web"         pm-frontend 0  frontend/apps/ppt-web/src/x.tsx frontend/packages/api-client/src/y.ts
run_case "pm-frontend drifts to backend"        pm-frontend 2  frontend/apps/ppt-web/src/x.tsx backend/servers/api-server/src/h.rs
run_case "pm-security touches auth + mfa"       pm-security 0  backend/crates/auth/src/jwt.rs backend/servers/api-server/src/handlers/mfa/totp.rs
run_case "pm-security drifts to documents"      pm-security 2  backend/servers/api-server/src/handlers/documents/list.rs
run_case "unknown owner returns 1"              not-a-role  1  backend/foo.rs
run_case "generic accepts everything"           generic     0  backend/x.rs frontend/y.ts docs/z.md

echo
if [ "$FAIL" = "0" ]; then
  echo "==> scope-check: PASS"
  exit 0
else
  echo "==> scope-check: FAIL" >&2
  exit 1
fi
