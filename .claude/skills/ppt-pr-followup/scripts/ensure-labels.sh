#!/usr/bin/env bash
# ensure-labels.sh — idempotently create the dispatcher's gating labels.
#
# Called once per dispatcher run (Phase 1 init). Safe to re-run: each
# `gh label create` is `|| true` so an existing label is a no-op.
#
# Adds:
#   needs-human-review — set by the dispatcher when the reviewer note matches
#                        a human-gate phrase. ppt-pr-merge refuses to merge
#                        a draft carrying this label even if approved + green.
#
# Usage: bash ensure-labels.sh [repo-slug]
#   default repo slug: martin-janci/property-management

set -u

REPO="${1:-martin-janci/property-management}"

gh label create needs-human-review \
  --repo "$REPO" \
  --color B60205 \
  --description "Draft PR is gated on a human-only review (macOS reviewer, domain expert, etc.). ppt-pr-merge refuses to land while this label is present." \
  2>/dev/null || true

# Future labels can be appended here in the same idempotent shape.
