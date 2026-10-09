# Action list

_Generated: 2026-10-09T04:30:00Z — regenerated from `action-list.json` (routine Phase 1.6 upkeep, pm-data rotation 2026-10-09)._

42 open items. Six new rows this run (pm-data: funnel-event schema, analytics-schema-foundations meta, screen-map instrumentationStatus field, analytics platform decision, support-data retention policy, mobile-event-parity audit). Top of list is still the infra unblocks (#2966 swagger-ui egress, #2652 KMP cloud, #2951 jest-expo) that gate the IDOR retries.

| ID | Priority | Owner | Action |
|---|---|---|---|
| `pm-data-analytics-schema-foundations-meta-2026-10-09` | high | pm-data | Meta: collapses the 6 pm-data items from 2026-07-23 (dispute add_evidence audit event, layout publish analytics, dispute-lifecycle KPIs, announcement fan-out... |
| `pm-data-funnel-event-schema-2026-10-09` | high | pm-data | Define a cross-platform funnel_step + funnel_fail event schema (publish-listing / submit-report / group-join / sync-schedule) covering reality-web + ppt-web ... |
| `pm-devops-unblock-mobile-native-cloud-builds` | high | pm-devops | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — still 5+ open backlog items structurally unclaimable in cloud; now compounded by #2966 s... |
| `gh-issue-2944-retry1` | high | pm-security | [CLOUD-BUILD-BLOCKED: backend api-server unbuildable in cloud runner (swagger-ui egress 403, #2966); tracked by open GitHub issue] Cross-tenant IDOR: violati... |
| `gh-issue-2945-retry1` | high | pm-security | [CLOUD-BUILD-BLOCKED: backend api-server unbuildable in cloud runner (swagger-ui egress 403, #2966); tracked by open GitHub issue] Cross-tenant IDOR (read+wr... |
| `gh-issue-2946-retry2` | high | pm-security | [CLOUD-BUILD-BLOCKED: backend api-server unbuildable in cloud runner (swagger-ui egress 403, #2966); tracked by open GitHub issue] Cross-tenant IDOR (read+wr... |
| `pm-data-analytics-platform-decision-2026-10-09` | high | pm-tech-lead | Decide analytics platform (bespoke pg aggregates vs Amplitude/PostHog/Segment) — carried from 2026-07-23 as an open decision; now blocks every emission wirin... |
| `code-review-mobile-native-kmp-inquiries-response-contract` | medium | pm-backend | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on every real /inquiries + /realtors/inquiri... |
| `code-review-ppt-web-core-authed-roots-session-leak` | medium | pm-backend | logout() AUTHED_QUERY_KEY_ROOTS allow-list omits mounted route groups; caches leak across users on shared workstation |
| `pm-data-mobile-event-parity-audit-2026-10-09` | medium | pm-data | Audit mobile-native + mobile-rn event-tracking parity with web — baseline measurement so we know the gap before new instrumentation lands; publish docs/data/... |
| `pm-data-screen-map-instrumentation-status-2026-10-09` | medium | pm-data | Add instrumentationStatus (none/partial/complete) to the screen-map frontmatter schema and backfill the 10 highest-trafficked screens; unlocks mechanical shi... |
| `pm-data-support-tooling-retention-policy-2026-10-09` | medium | pm-data | Publish retention policy for support_tooling_events + append-only audit tables (carried from 2026-05-28, 2026-07-23). Now blocks dispute/OAuth/layout audit-e... |
| `pm-scrum-master-stalled-accounting-pay-by-square-2559` | medium | pm-tech-lead | Stalled 64d: PR #2559 (PAY by Square accounting slice) needs reviewer slot — one of two non-dependabot >60d PRs; part of the accounting MVP trio that has bee... |
| `pm-scrum-master-stalled-invoice-lifecycle-2555` | medium | pm-tech-lead | Stalled 64d: PR #2555 (invoice lifecycle accounting slice) needs reviewer slot — companion to #2559; both are the aging accounting MVP trio tracked since 202... |
| `code-review-mobile-native-kmp-cancellation-swallowed` | low | pm-backend | mobile-native-kmp: shared repositories swallow CancellationException in catch(e: Exception), breaking coroutine cancellation and showing spurious errors |
| `code-review-mobile-native-kmp-create-listing-not-wired` | low | pm-backend | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded |
| `code-review-mobile-native-kmp-httpclient-no-timeout` | low | pm-backend | mobile-native-kmp shared Ktor HttpClient installs no HttpTimeout — every suspend API call can hang indefinitely on Android + iOS |
| `code-review-mobile-native-kmp-portfolio-analytics-caps-100` | low | pm-backend | mobile-native-kmp: getPortfolioAnalytics() truncates realtor portfolio at 100 listings — dashboard under-reports on large portfolios |
| `code-review-mobile-native-kmp-portfolio-analytics-unbounded-fanout-retry1` | low | pm-backend | mobile-native-kmp: getPortfolioAnalytics() fans out one analytics HTTP request per listing with no concurrency limit — up to 100 parallel GETs from a mobile ... |
| `code-review-mobile-rn-meter-reading-no-numeric-validation` | low | pm-backend | Mobile RN meter-reading input accepts non-numeric — silent write of garbage values |
| `code-review-ppt-web-core-documents-default-org-sentinel` | low | pm-backend | ppt-web documents routes fabricate a 'default-org' sentinel instead of gating when the user has no organizationId |
| `code-review-ppt-web-core-leases-hardcoded-eur-currency` | low | pm-backend | ppt-web lease mappers hardcode EUR for all monetary values, so PLN/HUF markets mislabel amounts |
| `code-review-ppt-web-core-rentals-mutation-silent` | low | pm-backend | ppt-web rentals: 4 mutations swallow non-auth errors (connection create, platform sync, guest check-in/out) |
| `code-review-reality-server-add-to-compare-toctou-max-cap` | low | pm-backend | reality-server add_to_compare TOCTOU on MAX_COMPARE_LISTINGS — two concurrent POSTs can exceed the 4-item cap |
| `code-review-reality-server-create-handlers-return-200-not-201` | low | pm-backend | reality-server create handlers return 200 OK but utoipa spec claims 201 Created — OpenAPI contract vs runtime divergence across 9 endpoints |
| `code-review-mobile-native-kmp-ssoservice-untested` | low | pm-qa | mobile-native-kmp: SsoService (deep-link token exchange, login, password reset, session restore) has zero direct tests |
| `screen-map-drift-pr-2994-ppt` | low | pm-qa | ppt-web route refactor (PR #2994) landed without touching docs/screens/ppt |
| `screen-map-drift-pr-3019-reality` | low | pm-qa | reality-web /report PR #3019 touched 2 routes with no docs/screens/reality update |
| `screen-map-drift-pr-3022-reality` | low | pm-qa | reality-web /sell PR #3022 touched 3 routes with no docs/screens/reality update |
| `screen-map-drift-pr-3033-reality` | low | pm-qa | reality-web /profile PR #3033 touched 3 routes with no docs/screens/reality update |
| `screen-map-drift-pr-3034-ppt` | low | pm-qa | ppt-web community-group-detail PR #3034 touched 2 routes with no docs/screens/ppt update |
| `screen-map-drift-pr-3036-ppt` | low | pm-qa | ppt-web route-group i18n PR #3036 touched 6 routes with no docs/screens/ppt update |
| `churn-hotspot-frontend/apps/ppt-web/src/features/workflow-automation/pages/CreateRulePage.tsx` | low | pm-tech-lead | ppt-web workflow-automation CreateRulePage.tsx — 4 commits in 14d (new hotspot) |
| `churn-hotspot-frontend/apps/ppt-web/src/features/workflow-automation/pages/TemplateLibraryPage.tsx` | low | pm-tech-lead | ppt-web workflow-automation TemplateLibraryPage.tsx — 3 commits in 14d (new hotspot) |
| `churn-hotspot-frontend/apps/ppt-web/src/routes/groups/community.tsx` | low | pm-tech-lead | ppt-web routes/groups/community.tsx recent churn — 3 commits in 14d (new hotspot) |
| `churn-hotspot-frontend/apps/ppt-web/src/routes/groups/core.tsx` | low | pm-tech-lead | ppt-web src/routes/groups/core.tsx — 4 commits in last 14d (new hotspot) |
| `churn-hotspot-frontend/apps/reality-web/src/app/[locale]/report/page.tsx` | low | pm-tech-lead | reality-web /report page recent churn — PR #3014 submit-wiring change |
| `churn-hotspot-frontend/apps/reality-web/src/app/[locale]/sell/page.tsx` | low | pm-tech-lead | reality-web /sell page recent churn — PR #3013 publish-wiring change |
| `churn-hotspot-frontend/apps/reality-web/src/components/home/FeaturedListings.tsx` | low | pm-tech-lead | reality-web FeaturedListings recent churn — PR #3015 error-surface fix |
| `churn-hotspot-frontend/apps/reality-web/src/components/import/SyncSchedule.tsx` | low | pm-tech-lead | reality-web components/import/SyncSchedule.tsx recent churn — 2 commits in 14d (new hotspot) |
| `churn-hotspot-frontend/apps/reality-web/src/lib/realtor-api.ts` | low | pm-tech-lead | reality-web lib/realtor-api.ts recent churn — 2 commits in 14d (new hotspot) |
| `code-review-reality-server-dup-url-validator` | low | pm-tech-lead | reality-server carries a duplicated URL-validation helper — merge into shared crate |
