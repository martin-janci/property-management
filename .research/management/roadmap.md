# PPT Roadmap — upkeep 2026-10-27

_Rebuilt from `coverage.json` (49/49 stories done across 13 epics) + current `action-list.json` (24 open) + sprint-status.yaml. Lag since 2026-10-27 01:07Z run: ~15h._

## State of the project

- **Stories: 49/49 done** across 13 epics. No status flips this run; `last_checked` refreshed on the single epic-9 story (`9-1-totp-2fa-setup`).
- Delta vs 2026-10-27 01:07Z: **1 PR merged** — #3183 code-review-api-handlers-violations-no-xtenant-idor-tests (narrow follow-up; no coverage story mapped).
- **Live delivery blocker:** 9-PR approved-review jam — the dispatcher payload (`claimable=6/72`) is almost entirely explained by this. Everything else is secondary until the jam clears.
- **New risk cluster:** `backend/servers/api-server/src/routes/violations.rs` surfaced 3 fresh authz findings in this run's Phase 1.5 code review on top of #3164 last week. Pattern suggests a route-wide sweep before further per-site patching.
- Screen coverage: 0 orphan screens · 0 orphan epics · 0 validation errors · 3 known missing UC links (UC-33.1/33.2/33.3, now explicitly queued as action-list rows).
- Open PRs this window: 26 total (5 touched); 9 in review-jam; 0 drafts in approved+stuck-draft state.

## Epic status (short)

- **Epic 6 — Announcements & Communication:** 6/6 done in coverage; sprint-status.yaml drift carried.
- **Epic 7A — Basic Document Management:** 5/5 done.
- **Epic 8A — Basic Notification Preferences:** 3/3 done.
- **Epic 10A — OAuth Provider Foundation:** 3/3 done.
- **Epic 10B — Platform Administration:** 7/7 done.
- **Epic 80 — Dispute Resolution:** 3/3 done in coverage (sprint-status still shows partial).
- **Epic 81 — Reports:** 2/2 done.
- **Epic 82 — iOS Reality Portal (SwiftUI):** 5/5 done.
- **Epic 83 — Portal Integrations:** 3/3 done.
- **Epic 84 — Documents & e-signature:** 5/5 done.
- **Epic 85 — Build/CI configuration:** 2/2 done.
- **Epic 79 — API client integration:** 4/4 done.
- **Epic 9 — 2FA/TOTP:** 1/1 done. **Re-checked this run (cursor idx 12).** No merged PR in the 2026-10-27 window touched 2FA / TOTP surfaces; `last_checked = 2026-10-27` stamped on the single story.

## Ranked plan

### merge-queue unblock (highest leverage, score 10)

- [high] **Clear the 9-PR approved-review jam** — merge #3024, #3075, #3108, #3119, #3171, #3184, #3185, #3186, #3188 after a last-minute sanity re-review — the dispatcher cannot drain until this clears — owner: **pm-tech-lead**
- [high] Unblock mobile-native/KMP builds in cloud runner (**#2652**) — 5+ KMP items structurally unclaimable — owner: **pm-devops** (carried)

### tier-1 security (score 8-9)

- [high] **Sweep violations.rs for role-gate / identity-check gaps** — Phase 1.5 surfaced 3 fresh findings (add_evidence role-gate asymmetry at :375, list_violations/list_appeals bulk leak at :295/:620, create_appeal missing violator-identity check at :584-600) on top of #3164 last week; one-shot audit beats per-site whack-a-mole — owner: **pm-backend**

### delivery process (score 7)

- [medium] **Decide auto-merge policy for Claude-Approvals PRs after a 24-48h cool-down** — the 9-PR jam suggests the human-merge step is the primary delivery bottleneck — owner: **pm-tech-lead**

### backend reliability (score 6-7)

- [medium] Diagnose + resolve #3131 merge-pipeline deadlock — long-stale; likely sqlx transaction scope or advisory-lock contention — owner: **pm-backend** (carried)
- [low] Un-quarantine the dispute KPIs test + add window_start<=window_end validation — carried from 2026-07-30 — owner: **pm-backend**

### quality / RN lint (score 4-5, carried)

- [medium] Resolve mobile RN jest-expo version rot (**#2951**) — owner: **pm-devops** (carried)

### KMP code-review backlog (score 2-3, all dep-blocked on #2652)

- CreateListingScreen onSubmit stub
- InquiriesResponse page_size vs reality-server `limit` mismatch
- Ktor HttpClient has no HttpTimeout
- getMyListings + portfolio analytics 100-cap + unbounded fan-out

### carried follow-ups (score 2-3)

- Portfolio/owner analytics drift (gh #3104, PR #3108 in review)
- NewsScreen category badge i18n defaultValue fallback (gh #3116, PR #3119 in review)
- Violations/appeal_number COUNT(*)+1 race (PR #3075 in review)
- Screen-map drift for PR #2994 (ppt-web route refactor, PR #3024 in review)
- Follow-up: violations add_evidence authz gap (#3164, PR #3171 in review)
- Churn hotspot: violations_role_authz_tests.rs (PR #3184 in review)
- ppt-web-core neighbors stale-token useMemo (PR #3185 in review)
- ppt-web-core useIotWebSocket.hasGivenUp dropped (PR #3186 in review)
- Failing-on-dev test gh-issue-3187 (PR #3188 in review)

### Screen-map drift

- [low] Link UC-33.1 / UC-33.2 / UC-33.3 to the correct screen-map `use-cases` arrays (3 gap-scan rows: `gap-screen-uc-33-{1,2,3}-link-to-screen-map` + one pm-frontend meta row `pm-frontend-link-missing-ucs-33`) — owner: **pm-frontend**
- [low] Reconcile ppt-web screen-map for PR #2994 route refactor — carried as `screen-map-drift-pr-2994-ppt`; PR #3024 is the resolution and is in the review-jam — owner: **pm-frontend / pm-qa**

Buffer: **24/36 open** · candidates ranked but unqueued: ~0 (coverage is all `done`; only 3 missing UC links were gap-scan-queuable). Below floor by 12 but ABOVE the half-full threshold (18) — not critical. The real delivery choke is the 9-PR review-jam, not the buffer depth.
