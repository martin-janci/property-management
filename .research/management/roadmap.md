# PPT Roadmap — upkeep 2026-11-08

_Rebuilt from `coverage.json` (49/49 stories done across 13 epics) + current `action-list.json` + sprint-status.yaml. 2-day lag since 2026-11-06 run; prior `upkeep` was 2026-10-26._

## State of the project

- **Stories: 49/49 done** across 13 epics. No status flips this run; `last_checked` refreshed on all `epic-10a` stories (cursor index 0 re-check — negative).
- Delta vs 2026-11-06: **7 PRs merged** — ppt-web registry cluster (#3351, #3354, #3355, #3359), reality-web shared-comparison body validation (#3350), reality-docs screen-map drift (#3358, re-land), api-server UC-57 session-revocation (#3353).
- **3 PRs closed unmerged** on 2026-11-08: #3301 (reality-server inquiry FK classification — NOT re-landed), #3333 (re-landed as #3358), #3356 (re-landed as #3359). Orphaned by a dev history rewrite.
- **New operator-action blocker: #3357** — dispatcher pipeline stalled on non-mechanical conflicts + Tier-1d signals dead-end. Visible as `buffer-low: claimable=6/72, backlog avail=0` on the fire-payload input.
- **Carried blockers resolved on GitHub** (all CLOSED this run, not previously reflected): #3187, #3168, #3164, #3131, #2652. project-state.md updated to drop them from the active list.
- **Churn cluster:** RegistryPage.tsx + ppt-web messages/*.json co-churn for 3 commits in 2 days. Expected during i18n rollout; recorded as a repeated-churn watch not a bug.

### Screen coverage

- 0 stories without screen-map · 0 orphan epics · 0 orphan screens · 0 missing UC links (per the last coverage.json scan; cheap upkeep didn't recompute).

## Ranked plan

### MVP (phase mvp)

- All stories in this phase are `done` per coverage.json. Nothing to rank.

### Follow-ups & non-gated backlog refill

- [high] **Re-land #3301** reality-server inquiry FK classification — owner: pm-backend — why: closed unmerged due to orphaned base; the fix (SQLSTATE 23503 over substring match) is sound.
- [high] **Resolve #3357 pipeline stall** — owner: pm-devops — why: dispatcher at 0 claimed for 10+ runs; needs operator action on orphaned branches + Tier-1d ingestion rewire.
- [high] **Seed non-gated web/reality-server vectors** — owner: pm-tech-lead + pm-security — why: fire payload reports `avail=0`; backlog cannot refill without a seeding sweep since coverage is 49/49.
- [high] **Create web + reality-server coverage-gap vectors** (shipped-without-sibling-test screens, reality-server contract edges) — owner: pm-frontend — why: pm-qa escalated this as the sustainable non-gated feed source.
- [high] **Deflake `rentals.connections.test.tsx`** — owner: pm-frontend — why: flake surfaced in #3356 body; erodes CI trust on the active registry area.
- [medium] **Reconcile 8a-3 sprint-status against test-hardening batch gate** — owner: pm-tech-lead — why: referenced gates (#480, #484) now CLOSED; sprint-status.yaml should match.
- [medium] **Define cloud-testable substitute for mobile-native-gated items** — owner: pm-tech-lead — why: 6/8 claimable items are KMP-gated; cloud CI can test API-contract slice even if device UI tests stay gated.
- [medium] **Review merged-PR assertion quality this cycle** (sample 2 of 7) — owner: pm-qa — why: 7 fast lands including the UC-57 session-revocation; verify revoked-token rejection across refresh/introspect/WS.

Buffer: 17/36 open · 8 candidates ranked but unqueued  ⚠ Buffer below half — consider running `/ppt-project-management scan` to refresh coverage with new gap signals once non-gated vectors are seeded.
