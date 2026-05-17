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

echo "==> ppt-research routine setup"

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
if [[ -n "${GH_TOKEN:-}" ]]; then
  # gh uses GH_TOKEN env automatically; no `gh auth login` needed.
  gh auth status || {
    echo "!! gh auth check failed; verify GH_TOKEN scope (needs repo:contents, pull-requests, issues)"
    exit 1
  }
  # Confirm we can see the repo at all.
  gh repo view martin-janci/property-management --json name,visibility | jq -r '.name'
else
  echo "!! GH_TOKEN not set — the routine cannot read PRs/issues or push state.json"
  echo "   Set it in the environment's Variables section."
  exit 1
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
