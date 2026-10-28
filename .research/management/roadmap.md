# PPT Roadmap — upkeep 2026-10-28

_Rebuilt from `coverage.json` (49/49 stories done across 13 epics) + current `action-list.json` + sprint-status.yaml. 1-day lag since 2026-10-27T01:07Z run._

## State of the project

- **Stories: 49/49 done** across 13 epics. No status flips this run; `last_checked` refreshed on epic-9 (1 story — TOTP 2FA Setup — stays `done`).
- Delta vs 2026-10-26 upkeep: **7 PRs merged** since the last routine run — sprint-status rollup fix (#3177), IoT WS reconnect test harden (#3178), reality-web price locale fix (#3179), community IDOR test cover (#3180), assign_violation org-membership (#3181), moderation assign/unassign 404 (#3182), violations mutation cross-org IDOR fix (#3183). All test-included; three tier-1 IDOR hardenings. **None map to a coverage story** (coverage is 100% done; these are code-review-driven fixes).
- Blockers: **#3187** still open (cross-tenant assign_violation 403 vs expected 404) — dispatcher opened draft PR #3188 pending human review. Prior #3168 (cargo-deny RUSTSEC) context carried.
- Open PRs this window: 4 draft dispatcher PRs (#3184, #3185, #3186, #3188 pending human review).
- Screen coverage: 0 orphan screens · 0 validation errors · 3 known missing UC links (UC-33.1/33.2/33.3 already queued elsewhere).

## Epic status (short)

- **Epic 6 — Announcements & Communication:** 6/6 done in coverage; sprint-status.yaml drift carried.
- **Epic 7A — Basic Document Management:** 5/5 done.
- **Epic 8A — Basic Notification Preferences:** 3/3 done.
- **Epic 10A — OAuth Provider Foundation:** 3/3 done.
- **Epic 10B — Platform Administration:** 7/7 done.
- **Epic 80 — Dispute Resolution:** 3/3 done in coverage (sprint-status drift carried).
- **Epic 81 — Reports:** 2/2 done.
- **Epic 82 — iOS Reality Portal (SwiftUI):** 5/5 done.
- **Epic 83 — Portal Integrations:** 3/3 done.
- **Epic 84 — Documents & e-signature:** 5/5 done.
- **Epic 85 — Build/CI configuration:** 2/2 done.
- **Epic 79 — API client integration:** 4/4 done.
- **Epic 9 — 2FA/TOTP:** 1/1 done. **Re-checked this run (cursor idx 12).** No merged PR in the 2026-10-26..2026-10-28 window touched 2FA/TOTP surfaces; `last_checked = 2026-10-28` stamped.

## Ranked plan

### merge-queue unblock (highest leverage)

- [high] Resolve **#3187** (cross-tenant assign_violation returns 403, expected 404) — draft PR #3188 awaiting human review; blocks backend CI on dev — owner: **pm-security**
- [high] Unblock mobile-native/KMP builds in cloud runner (**#2652**) — carried; 5+ KMP items structurally unclaimable — owner: **pm-devops**

### tier-1 security + authz posture (already shipping)

- [high] (carried, in-flight) Gate violations `add_evidence` sub-resource (**#3164**) — same shape as historic #2483 — owner: **pm-backend**
- [medium] Sweep remaining `add_*` sub-routes (announcements/attachments, documents/signatures, mediation/evidence) for the same ungated pattern — owner: **pm-backend**

### mobile RN quality debt (NEW this run — pm-frontend focus)

- [medium] Ship a VoteDetailScreen fix pass — rank-position UI for `ranked` questions + remove dead `?? 'English'` i18n fallback idiom — owner: **pm-frontend**
- [medium] Adopt `eslint-plugin-react-hooks` + `i18next-hardcoded-strings` in `frontend/apps/mobile` and gate CI on it (addresses carried pm-qa risk 2026-08-25) — owner: **pm-frontend**

### backend reliability (score 6-7)

- [medium] Diagnose + resolve **#3131** merge-pipeline deadlock — long-stale — owner: **pm-backend**
- [medium] Follow-up draft PR #3167 (unassign moderation 404) — owner: **pm-backend**

### refactor-with-security-overtones

- [medium] **#3006** — unify triplicated auth-header + MFA-retry in `@ppt/api-client` fetch primitives — owner: **pm-tech-lead**

### quality / RN lint (score 4-5, carried)

- [medium] Resolve mobile RN jest-expo version rot (**#2951**) — owner: **pm-devops**

### KMP code-review backlog (all blocked on #2652)

- CreateListingScreen onSubmit stub
- InquiriesResponse page_size vs reality-server `limit` mismatch
- Ktor HttpClient has no HttpTimeout
- getPortfolioAnalytics() 100-cap + unbounded fan-out

### carried follow-ups

- Portfolio/owner analytics drift (gh #3104)
- NewsScreen category badge i18n defaultValue fallback (gh #3116)
- Violations/appeal_number COUNT(*)+1 race (local repro welcome)
- screen-map drift for PR #2994 (ppt-web route refactor)
- Un-quarantine dispute KPIs test + window_start<=window_end validation
- Reconcile sprint-status.yaml drift vs coverage.json

### Screen-map drift

_Nothing new this run — carried: 3 missing UC links (UC-33.1/33.2/33.3), 1 ppt-web route refactor without screen-map update (PR #2994). Reality-web `[locale]/*` churn flagged for a Playwright-smoke roadmap item (pm-frontend / pm-qa)._

Buffer: **20/36 open** · candidates ranked but unqueued: ~0 (coverage is all `done` — refill depends on the dispatcher's external backlog scan, not this upkeep run). Buffer below floor by 16 but above half (18) — healthier than yesterday after three new pm-frontend items landed; a dispatcher backlog scan remains overdue to top up.
