# PPT Roadmap — upkeep 2026-10-14

_Rebuilt from `coverage.json` (49/49 stories done across 13 epics) + current `action-list.json` + sprint-status.yaml. 6-day lag since 2026-10-08 run. pm-data rotation slot._

## State of the project

- **Stories: 49/49 done** across 13 epics. No status flips this window; the window landed quality + bugfix work on top of the already-done feature set.
- Delta vs 2026-10-01 upkeep: **26 PRs merged** — the broadest observation window in weeks. Themes: reality-web /sell + /report + /profile funnel hardening (#3044 /#3046 /#3056 /#3058 /#3064 /#3066 /#3067); ppt-web community/groups/documents/rentals mutation-error surfacing (#3045 /#3048 /#3049 /#3050); screen-map drift reconciliations (#3053 /#3054 /#3055 /#3058 /#3059); churn-hotspot test coverage + small refactors (#3060..#3063 /#3065 /#3068); auth-fetch triplication closed by #3071 (resolves carried #3006); dispatcher ENOSPC fix #3080 (closes #3073).
- Screen coverage: 0 orphan screens · 0 validation errors · 3 known missing UC links (UC-33.1/33.2/33.3 already queued elsewhere).
- Open PRs: 17 — 7 security retries drafts (`needs-human-review`) gated on reviewer bandwidth + the #3076 ammonia bump that blocks backend CI; 8 dependabot bulk; 2 older stalled.
- **New role focus**: pm-data flagged a live data-health incident on portfolio-analytics (#3078) and complete absence of event tracking on the funnels PRs #3044..#3067 just hardened.

## Epic status (short)

- **Epic 6 — Announcements & Communication:** 6/6 done in coverage (sprint-status still shows in-progress — drift).
- **Epic 7A — Basic Document Management:** 5/5 done. PR #3049 (gate documents routes on organizationId) is defense-in-depth on shipped surface.
- **Epic 8A — Basic Notification Preferences:** 3/3 done.
- **Epic 10A — OAuth Provider Foundation:** 3/3 done.
- **Epic 10B — Platform Administration:** 7/7 done.
- **Epic 80 — Dispute Resolution:** 3/3 done.
- **Epic 81 — Reports:** 2/2 done. Last checked 2026-08-31.
- **Epic 82 — iOS Reality Portal (SwiftUI):** 5/5 done. Last checked 2026-10-01.
- **Epic 83 — Portal Integrations:** 3/3 done. **Re-checked this run (cursor idx 8).** No merged PR in the 2026-10-08..10-14 window touched airbnb/booking/portal-webhooks surfaces; `last_checked = 2026-10-14` stamped on all 3 stories.
- **Epic 84 — Documents & e-signature:** 5/5 done.
- **Epic 85 — Build/CI configuration:** 2/2 done.
- **Epic 79 — API client integration:** 4/4 done. PR #3071 (folded last auth-header copy into shared bearerAuthorization) closes risk #3006.
- **Epic 9 — 2FA/TOTP:** 1/1 done.

## Ranked plan

### Phase MVP — infra unblock (highest leverage, score 10)

- [high] **Land PR #3077** (`ammonia` → 4.1.7, closes RUSTSEC-2021-0157) — gates every backend-touching PR today. Owner: **pm-security / pm-devops**.
- [high] Unblock utoipa-swagger-ui egress for api-server cloud builds (**#2966**) — still open, still gating. Owner: **pm-devops**.
- [high] Unblock mobile-native/KMP builds in cloud runner (**#2652**) — 7 KMP items unclaimable. Owner: **pm-devops**.

### Phase MVP — security tier-1 (score 8-9)

- [high] Land **PR #3082** (SSRF IPv4-compatible IPv6 bypass, `gh-issue-3079`) — static validator misses `::127.0.0.1`-style literals. Owner: **pm-security**.
- [high] Land **PR #3081** (portfolio-analytics property-metrics 500 + IDOR regression tests, `gh-issue-3078`) — live data-health incident + untracked schema drift. Owner: **pm-tech-lead**.
- [high] Land **PR #3070** (ViolationRepository::list_notifications org scoping, `gh-issue-3069`) — same IDOR class as #2944. Owner: **pm-tech-lead**.
- [medium] Land **PR #3072** (violations missing-role-authz — any org member could issue fines and approve own appeal). Owner: **pm-backend**.
- [low] Land **PR #3074** (record_payment atomic) + **PR #3075** (violation/appeal numbering race). Owner: **pm-backend**.

### Phase MVP — analytics / data-health (new rotation; score 7)

- [high] **Instrument reality-web /sell wizard funnel** — 5 events via existing trackEvent bus. Owner: **pm-data** → pm-frontend.
- [high] **Instrument reality-web /report submission flow** — 4 events with returned id + error class. Owner: **pm-data** → pm-frontend.
- [high] **Classify #3081 as data-health incident** + add portfolio-metrics 5xx-rate alert + schema-drift CI gate. Owner: **pm-data** (post-#3081).
- [medium] **Canonical currency metric** after #3051 — publish `docs/analytics/metrics.md`; dedupe revenue dashboards. Owner: **pm-data**.
- [medium] **Mutation-error funnel event helper** wired at the 3 sites #3048/#3049/#3050 just surfaced. Owner: **pm-data** → pm-frontend.
- [medium] **Minimum-analytics DoD** + PR-template checklist so no new story ships without instrumentation. Owner: **pm-data**.

### Phase MVP — reviewer-starvation (score 5-6)

- [medium] Reviewer slot for stalled accounting trio **#2559** (PAY by Square) and **#2555** (invoice lifecycle) — 78d idle. Owner: **pm-tech-lead**.
- [medium] Reviewer bandwidth for the 7 `needs-human-review` security drafts. Owner: **pm-tech-lead**.

### Phase MVP — quality / RN lint (score 4-5)

- [medium] Resolve mobile RN jest-expo version rot (**#2951**). Owner: **pm-devops**.
- [medium] Adopt eslint-plugin-react-hooks + no-hardcoded-strings in frontend/apps/mobile. Owner: **pm-frontend**.

### Phase MVP — KMP code-review backlog (score 2-3, all blocked on #2652)

- InquiriesResponse page_size vs reality-server `limit` mismatch
- Ktor HttpClient has no HttpTimeout
- CancellationException swallowed in repos
- getPortfolioAnalytics() 100-cap + unbounded fan-out
- CreateListingScreen onSubmit stub (NotImplementedError)
- SsoService zero direct tests

### Screen-map drift

- [low] `screen-map-drift-pr-2994-ppt` — ppt-web route refactor landed without touching docs/screens/ppt. Owner: **pm-qa**.

Buffer: 23/36 open · 0 candidates ranked but unqueued (coverage is 49/49 done, so gap-scan yields no new candidates; buffer refills from role rotation + dispatcher-issue-ingest).
