#!/usr/bin/env bash
#
# Setup script for the cloud Claude Code routine that hosts the
# property-management research routine.
#
# Paste the BODY of this file into claude.ai/code/routines → environment
# settings → "Setup script". The script runs once per environment build and
# its output is cached for ~7 days, so keep it idempotent and fast.
#
# What this installs / verifies:
#   - GitHub CLI (`gh`) — not preinstalled in the default sandbox image
#   - jq + yq  — for JSON/YAML manipulation in the routine prompt
#   - git config so the routine can commit on `main`
#   - Smoke check that `gh auth status` works against the repo
#
# Environment variables the routine expects (set in claude.ai env vars):
#   GH_TOKEN         — fine-grained token with read access to PRs / issues /
#                      contents on martin-janci/property-management, and
#                      WRITE access to contents (for committing under
#                      .research/). Used by `gh` automatically.
#   GIT_AUTHOR_NAME  — defaults to "ppt-research-routine"
#   GIT_AUTHOR_EMAIL — defaults to "ppt-research-routine@martin-janci.dev"
#
# This script DOES NOT install anything for the implementation agent — the
# implementer runs locally or via the ppt-bridge-mcp (see README).

set -euo pipefail
# Verbose tracing only when explicitly requested. Avoids leaking token-shaped
# fragments into the cached log on every routine env build.
[[ "${DEBUG:-0}" == "1" ]] && set -x

echo "==> ppt-research routine setup"

# --- 0. GH_TOKEN sanity ----------------------------------------------------
# Fail fast: an empty / missing token guarantees the routine cannot read PRs
# or push state.json, and we'd rather break the env build than silently land
# on an unauthenticated `gh` later in the script.
if [[ -z "${GH_TOKEN:-}" ]]; then
  echo "!! GH_TOKEN is unset or empty — the routine cannot read PRs/issues or push state.json"
  echo "   Set it in the environment's Variables section as a fine-grained PAT with:"
  echo "     repo:contents (read+write), pull-requests (read), issues (read)"
  exit 1
fi

# --- 1. gh CLI -------------------------------------------------------------
# Pinned version. `releases/latest/download/<asset>` is a redirector that fills
# in whatever the current latest tag is — using a hardcoded filename like
# `gh_2.61.0_linux_amd64.tar.gz` against that path 404s once a newer release
# ships (the asset filename includes the current version). Pin the tag instead.
GH_CLI_VERSION="${GH_CLI_VERSION:-2.61.0}"
if ! command -v gh >/dev/null 2>&1; then
  echo "==> Installing gh CLI v${GH_CLI_VERSION}"
  curl -fsSL "https://github.com/cli/cli/releases/download/v${GH_CLI_VERSION}/gh_${GH_CLI_VERSION}_linux_amd64.tar.gz" \
    | tar -xz -C /tmp
  mv /tmp/gh_*/bin/gh /usr/local/bin/gh
  chmod +x /usr/local/bin/gh
fi
gh --version | head -1

# --- 2. jq + yq ------------------------------------------------------------
command -v jq >/dev/null 2>&1 || apt-get install -y --no-install-recommends jq
command -v yq >/dev/null 2>&1 || {
  curl -fsSL https://github.com/mikefarah/yq/releases/latest/download/yq_linux_amd64 \
    -o /usr/local/bin/yq && chmod +x /usr/local/bin/yq
}
jq --version
yq --version

# --- 3. git identity for the routine's commits ----------------------------
git config --global user.name  "${GIT_AUTHOR_NAME:-ppt-research-routine}"
git config --global user.email "${GIT_AUTHOR_EMAIL:-ppt-research-routine@martin-janci.dev}"
git config --global pull.ff only
git config --global init.defaultBranch main

# --- 4. gh auth + sanity ---------------------------------------------------
# gh uses GH_TOKEN env automatically; no `gh auth login` needed.
gh auth status || {
  echo "!! gh auth check failed; verify GH_TOKEN scope (needs repo:contents, pull-requests, issues)"
  exit 1
}

# Confirm we can see the repo at all (proves read access on contents).
gh repo view martin-janci/property-management --json name,visibility | jq -r '.name'

# Surface the token's actual scopes for diagnostics. Fine-grained PATs report
# scopes as `repo:status,repo_deployment,...` via the x-oauth-scopes response
# header; classic PATs report the umbrella `repo` scope. Both work, but if a
# user accidentally gives the routine a read-only token, the routine's first
# push will fail mysteriously — surface that here, not 30 lines into Phase 4.
SCOPES="$(gh api -i user 2>/dev/null | awk -F': ' 'tolower($1) == "x-oauth-scopes" { sub(/\r$/, "", $2); print $2 }' | head -1)"
# Strip whitespace so a header value of " " (which fine-grained PATs commonly
# return) is treated the same as a missing header — otherwise `[[ -n ]]` is
# true for whitespace-only and the assertion below false-flags a valid token.
SCOPES_TRIM="${SCOPES//[[:space:]]/}"
if [[ -n "$SCOPES_TRIM" ]]; then
  echo "==> token scopes: $SCOPES"
  # x-oauth-scopes only reports classic OAuth scope names (e.g. `repo`,
  # `public_repo`, `workflow`). Fine-grained PATs report empty/whitespace
  # here and enforce permissions server-side per-resource, so they fall
  # through to the else branch above.
  if ! echo "$SCOPES" | grep -qE '(^|, )(repo|public_repo)(,|$)'; then
    echo "!! token scopes don't include repo/public_repo — pushes to .research/ will fail"
    echo "   if this is a fine-grained PAT, ensure Contents = Read & Write is set on this repo"
    exit 1
  fi
else
  # No header (or whitespace-only) → fine-grained PAT or older gh; trust the
  # API call we already made and continue. Permissions are enforced per
  # resource on the server side for fine-grained PATs.
  echo "==> token scopes: (none reported — fine-grained PAT?)"
fi

# --- 5. seed .research/ if the routine was created before PR #266 merged ---
# (Defensive: if a fresh clone arrives before PR #266 lands, scaffold won't
# exist yet and the routine should noop on its first run.)
if [[ ! -d .research ]]; then
  echo "!! .research/ does not exist — the routine will exit on first run."
  echo "   Merge PR #266 first."
  exit 0
fi

echo "==> setup OK"
