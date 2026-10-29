# PPT Roadmap — upkeep 2026-10-29

_Rebuilt from `coverage.json` (49/49 stories done across 13 epics) + current `action-list.json` + sprint-status.yaml. 16h since previous routine run (2026-10-28 10:54 UTC)._

## State of the project

- **Stories: 49/49 done** across 13 epics. No status flips this run; `last_checked = 2026-10-29` stamped on all 3 epic-10a stories (upkeep cursor idx 0 → 1).
- Delta vs 2026-10-28 upkeep: **0 PRs merged**. 4 open draft dispatcher PRs (#3190–#3193).
- **Backend CI remains red** on dev via #3187 (cross-tenant assign returns 403 vs expected 404). Blocks every backend PR's `test` check.
- **Merge pipeline deadlock** (#3131) still active: 11 approved auto-impl PRs stacked; dispatcher cannot self-approve.
- Screen coverage: 0 orphan screens · 0 validation errors · 3 known missing UC links (UC-33.1/33.2/33.3 already queued elsewhere).

## Epic status (short)

- **Epic 6 — Announcements & Communication:** 6/6 done.
- **Epic 7A — Basic Document Management:** 5/5 done.
- **Epic 8A — Basic Notification Preferences:** 3/3 done.
- **Epic 10A — OAuth Provider Foundation:** 3/3 done. **Re-checked this run (cursor idx 0).** No PR in the 16h window touched OAuth provider surfaces (none merged at all).
- **Epic 10B — Platform Administration:** 7/7 done.
- Epic 9 (TOTP), 79, 80, 81, 82, 83, 84, 85: all 3-7 stories per epic done.

## Ranked plan

### Phase 1 — Unblock merge pipeline (merge-queue infra)

- **[high]** Land #3187 fix (PR #3188 or operator-authored equivalent) → unred backend CI — owner: pm-security + pm-tech-lead — why: single-point-of-failure for every backend PR
- **[high]** Decide + act on #3131 unblock path (second-identity / admin-merge-lane / relaxed-protection) → drain 11-PR review queue — owner: pm-tech-lead + pm-devops — why: 24-day queue age at head; dispatcher cannot route around operator gate
- **[high]** Add review-queue aging alarm to routine brief — owner: pm-qa — why: makes IG3 freshness drift visible before merge
- **[medium]** Audit 11 in-review auto-impl PRs for IG3-contract drift — owner: pm-qa — why: 15–24d queue age likely invalidated regression evidence

### Phase 2 — Hygiene / parked backlog

- **[medium]** `pm-qa-add-evidence-authz-regression-test-3164` — add resident_cannot_add_evidence test (red on dev before fix) — owner: pm-qa + rust-backend
- **[medium]** `pm-qa-promote-review-queue-aging-risk-dec002` — formalize DEC-002 for merge-pipeline unblock — owner: pm-tech-lead
- **[low]** `pm-qa-test-shadow-debt-line-in-synthesis` — add test-shadow-debt line to pm-scrum-master synthesis — owner: pm-scrum-master
- **Mobile parked (5 items)** — all gated on `pm-devops-unblock-mobile-native-cloud-builds` (#2652)

Buffer: 41/36 open · 5 pm-qa high/medium items queued · 11 approved PRs un-mergeable upstream
