# PPT Roadmap — upkeep 2026-10-09

_Rebuilt from `coverage.json` (49/49 stories done across 13 epics) + current `action-list.json` (42 open) + sprint-status.yaml. 1 routine run behind (previous: 2026-10-01). pm-data rotation this run (idx 6 → 7)._

## State of the project

- **Stories: 49/49 done** across 13 epics. No status flips this window.
- Delta vs 2026-10-01 upkeep: **6 PRs merged** (bugfix dispatcher wave 2026-10-08) — #3043 (SyncSchedule banner), #3044 (/sell Publish 405), #3045 (group-detail i18n), #3046 (/report sitemap + success id), #3047 (/profile next-intl locale), #3048 (community group join/leave error surface). All closed `gh-issue-30**` follow-ups. Zero coverage status flips (all surfaces already shipped); evidence lines appended to 4 adjacent stories (82-5, 83-1/2/3).
- **New analytics-debt signal (pm-data run):** the publish-405 defect (#3044) was invisible to metrics for an unknown duration — no `listing_publish_success` KPI exists anywhere. Same shape across reality-web /report submit, community-group join/leave, and SyncSchedule flows. Funnel-event schema is now the top data-track lever.
- **Infra blockers unchanged (carried):** #2966 utoipa-swagger-ui egress, #2652 KMP cloud build, #2951 jest-expo rot. All three still gate large chunks of the dispatcher queue.
- Screen coverage: 0 orphan screens · 0 validation errors · 3 known missing UC links (UC-33.1/33.2/33.3 still queued elsewhere) · **new field proposal:** `instrumentationStatus` (none/partial/complete).

## Epic status (short)

- **Epic 6 — Announcements & Communication:** 6/6 done in coverage; sprint-status drift (in-progress) carried.
- **Epic 7A — Basic Document Management:** 5/5 done.
- **Epic 8A — Basic Notification Preferences:** 3/3 done.
- **Epic 10A — OAuth Provider Foundation:** 3/3 done.
- **Epic 10B — Platform Administration:** 7/7 done.
- **Epic 80 — Dispute Resolution:** 3/3 done.
- **Epic 81 — Reports:** 2/2 done.
- **Epic 82 — iOS Reality Portal (SwiftUI):** 5/5 done. 82-5 evidence appended (PR #3047 /profile i18n adjacent to iOS inquiries/account story).
- **Epic 83 — Portal Integrations:** 3/3 done. **Re-checked this run (cursor idx 8).** `last_checked = 2026-10-09` stamped on all 3; PR #3043 SyncSchedule banner noted as adjacent realtor-import UX; no coverage flip.
- **Epic 84 — Documents & e-signature:** 5/5 done.
- **Epic 85 — Build/CI configuration:** 2/2 done.
- **Epic 79 — API client integration:** 4/4 done.
- **Epic 9 — 2FA/TOTP:** 1/1 done.

## Ranked plan

### analytics foundations (new this run, highest leverage non-infra, score 9)

- [high] **Define cross-platform `funnel_step` + `funnel_fail` schema** (publish-listing / submit-report / group-join / sync-schedule) across reality-web + ppt-web + mobile — would have surfaced #3044 publish-405 in minutes — owner: **pm-data** (depends: analytics platform decision)
- [high] **Land the `analytics-schema-foundations-2026-10-09` meta-task** that collapses the 6 open 2026-07-23 pm-data items into one dispatcher-claimable batch — owner: **pm-data**
- [high] **Analytics platform decision** (bespoke pg vs Amplitude/PostHog/Segment) — 78d open; now blocks every emission wire — owner: **pm-tech-lead**

### infra unblock (highest leverage, carried, score 10)

- [high] Unblock utoipa-swagger-ui egress for api-server cloud builds (**#2966**) — gates every backend PR, including all 3 IDOR retries — owner: **pm-devops**
- [high] Unblock mobile-native/KMP builds in cloud runner (**#2652**) — 5+ KMP items structurally unclaimable — owner: **pm-devops**

### security tier-1 (carried, score 8-9)

- [high] Land #2946 fix (portfolio_analytics IDOR) — retry 2/2 draft open as `gh-issue-2946-retry2` — blocked by #2966 — owner: **pm-security**
- [high] Land #2944 fix (violation comments/evidence/payments IDOR + internal-notes leak) — retry 1/2 — blocked by #2966 — owner: **pm-security**
- [high] Land #2945 fix (portfolio_properties IDOR) — retry 1/2 — blocked by #2966 — owner: **pm-security**

### refactor-with-security-overtones (carried, score 6)

- [medium] #3006 — unify triplicated auth-header + MFA-retry in @ppt/api-client fetch primitives (post-merge review of PR #3004) — owner: **pm-tech-lead**

### reviewer-starvation (score 5-6, aging)

- [medium] Reviewer slot for stalled accounting trio **#2559** (PAY by Square) and **#2555** (invoice lifecycle) — now 72d idle — owner: **pm-tech-lead**

### quality / RN lint (carried, score 4-5)

- [medium] Resolve mobile RN jest-expo version rot (**#2951**) — unblocks RN code-review items — owner: **pm-devops**
- [medium] Adopt eslint-plugin-react-hooks + no-hardcoded-strings in frontend/apps/mobile — owner: **pm-frontend**

### data / observability (new this run, score 4-5)

- [medium] Add `instrumentationStatus` to screen-map frontmatter schema + backfill top 10 trafficked screens — owner: **pm-data** (coord with pm-tech-lead on schema)
- [medium] Publish retention policy for support_tooling_events + append-only audit tables (carried 2026-05-28 → 2026-07-23 → now) — owner: **pm-data** (depends: pm-security GDPR classification)
- [medium] Mobile event-tracking parity audit (web vs mobile-native vs mobile-rn) baseline — owner: **pm-data**

### KMP code-review backlog (carried, score 2-3, all blocked on #2652)

- InquiriesResponse page_size vs reality-server `limit` mismatch
- Ktor HttpClient has no HttpTimeout
- CancellationException swallowed in repos
- getPortfolioAnalytics() 100-cap + unbounded fan-out
- CreateListingScreen onSubmit stub
- SsoService untested

### carried refactors (score 2-3)

- reality-server duplicated URL-validation helper — merge into shared crate
- api-server routes/integrations/webhook.rs repeated-churn (3 hotspot windows)
- logout() AUTHED_QUERY_KEY_ROOTS allow-list gap (session leak)

Buffer: **42 open / 36 floor** — above target (new +6 pm-data rows). No underflow warning. Top lever remains **#2966 unblock**; next-highest lever is the pm-data `funnel_step` schema which would have made #3044 publish-405 metric-visible.
