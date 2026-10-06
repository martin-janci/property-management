# PPT Roadmap — upkeep 2026-10-06

_Rebuilt from `coverage.json` (49/49 stories done across 13 epics) + current `action-list.json` + Phase 1 observation data. 3-day lag since 2026-10-03 run._

## State of the project

- **Stories: 49/49 done** across 13 epics. Composition unchanged since 2026-08-25 (last partial flipped there).
- Delta vs 2026-10-01 upkeep: **6 PRs merged**, 3 of them a repeating "silent-dropped-input / silent-failure" class on reality-web (#3019/#3021/#3022), 2 docs/screen-map reconciles (#3023/#3027), 1 dispatcher doc rule (#3018). Zero coverage status flips; `last_checked` refreshed on epic-83 + 84-3 + 10b-1.
- **New near-term lever: PR #3026 (vendored swagger-ui) is in draft and reviewer-approved** — merging it closes #2966 and immediately unblocks all 3 IDOR retries + ~9 claimable items. Highest-leverage single human action in the backlog this cycle.
- **#2652 KMP cloud build gate** still open; no proposed fix in this window.
- Screen coverage: 0 orphan screens · 0 validation errors · 3 known missing UC links (UC-33.1/33.2/33.3 carried).

## Epic status (short)

- **Epic 6 — Announcements & Communication:** 6/6 done.
- **Epic 7A — Basic Document Management:** 5/5 done.
- **Epic 8A — Basic Notification Preferences:** 3/3 done.
- **Epic 10A — OAuth Provider Foundation:** 3/3 done.
- **Epic 10B — Platform Administration:** 7/7 done. PR #3023 + #3027 reconciled screen-map docs with the shipped role-gate PRs (#2995, #3002). last_checked 2026-10-06.
- **Epic 80 — Dispute Resolution:** 3/3 done.
- **Epic 81 — Reports:** 2/2 done.
- **Epic 82 — iOS Reality Portal (SwiftUI):** 5/5 done.
- **Epic 83 — Portal Integrations:** 3/3 done. **Re-checked this run (cursor idx 8).** No merged PR in the 2026-10-03..10-06 window touched Airbnb / Booking / portal-webhook surfaces; `last_checked = 2026-10-06` stamped on all 3 stories.
- **Epic 84 — Documents & e-signature:** 5/5 done. 84-3 (price tracking) refreshed with #3021 evidence (reality-web PriceAlerts mark-read error toast).
- **Epic 85 — Build/CI configuration:** 2/2 done.
- **Epic 79 — API client integration:** 4/4 done.
- **Epic 9 — 2FA/TOTP:** 1/1 done.

## Ranked plan

### infra unblock (highest leverage, score 10)

- [high] **Human merge PR #3026** (vendored swagger-ui) — closes #2966 — gates every backend PR including all 3 IDOR retries — owner: **pm-devops** (approve+merge)
- [high] Unblock mobile-native/KMP builds in cloud runner (#2652) — 5+ KMP items structurally unclaimable — owner: **pm-devops**

### security tier-1 (score 8-9, all parked behind #2966)

- [high] Land #2946 fix (portfolio_analytics IDOR) — retry 2/2 draft open — owner: **pm-security**
- [high] Land #2944 fix (violation comments/evidence/payments IDOR + internal-notes leak) — retry 1/2 — owner: **pm-security**
- [high] Land #2945 fix (portfolio_properties IDOR) — retry 1/2 — owner: **pm-security**

### reviewer-starvation (score 5-6)

- [medium] Reviewer slot for stalled accounting trio **#2559** (PAY by Square) + **#2555** (invoice lifecycle) — 68d idle — owner: **pm-tech-lead**

### near-term human-merge queue (score 4-6)

- [medium] Human merge PR #3025 (jest-expo pin, Closes #2951) — owner: **pm-devops**
- [medium] Human merge PR #3028 (reality-server add-to-compare TOCTOU) — owner: **pm-tech-lead**
- [medium] Human merge PR #3020 (ppt-web workflow-automation toast i18n) — owner: **pm-frontend**
- [medium] Human merge PR #2987 (AUTHED_QUERY_KEY_ROOTS session-leak) — owner: **pm-backend**
- [medium] Human merge PR #2992 (reality-server dup URL validator) — owner: **pm-tech-lead**
- [medium] Human merge PR #2997 (RN meter-reading numeric validation) — owner: **pm-frontend**
- [medium] Close abandoned draft PR #3024 (superseded by #3027) — owner: **pm-scrum-master**
- [medium] Human reset / re-cut PR #2902 (contaminated branch) — owner: **pm-scrum-master**

### pm-data (new this run, score 4-6)

- [medium] Vitest data-contract-fuzz helper + pilot on reality-web /sell + /report (catches silent-dropped-input class demonstrated by #3017/#3016 fixes) — owner: **pm-qa**
- [low] Daily reviewer-note bucket counter over assignments.json — turns 2 months of dark ledger entries into a trend — owner: **pm-data**

### refactor-with-security-overtones (score 6)

- [medium] #3006 — unify triplicated auth-header + MFA-retry in @ppt/api-client fetch primitives — owner: **pm-tech-lead**

### quality / RN lint (score 4-5)

- [medium] Adopt eslint-plugin-react-hooks + no-hardcoded-strings in frontend/apps/mobile — owner: **pm-frontend**

### KMP code-review backlog (score 2-3, all blocked on #2652)

- InquiriesResponse page_size vs reality-server `limit` mismatch
- Ktor HttpClient has no HttpTimeout
- CancellationException swallowed in repos
- getPortfolioAnalytics() 100-cap + unbounded fan-out
- CreateListingScreen onSubmit stub
- SsoService untested

### carried refactors (score 2-3)

- api-server routes/integrations/webhook.rs repeated-churn (3 hotspot windows)
- reality-web FeaturedListings churn-hotspot
- ppt-web community-group detail / reality-web profile mock-data pages
- screen-map drift from PR #3013 + #3014 (reality)

**Buffer status:** carried-forward infra blockers unchanged. The near-term unblock set is a human-merge queue of 6+ approved draft PRs — all of which need review+merge, not more implementer work.
