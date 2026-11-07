# Action list

<sub>Last regenerated: 2026-11-07T02:40:00Z by routine</sub>

| Priority | Owner | Action | Deadline | Depends on | Status |
|---|---|---|---|---|---|
| high | pm-devops | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — still 5+ open backlog items structurally unclaimable in cloud; now compounded by #2966 so claimable=9/72 (buffer-low) | — | — | open |
| high | pm-qa | Add a happy-path callback test for the Android SSO CSRF state fix (#2574) and a mobile layout-cache test (#2486) asserting the cache key includes tenant_id and is purged on logout | — | — | open |
| high | pm-qa | Add an integration test that exercises the real announcement fan-out RLS and targeting SQL (replacing the pure-Rust re-model #2484) — include a cross-tenant target_ids case asserting zero fan-out to foreign orgs | — | — | open |
| high | pm-qa | Add a failing-first test for portfolio_analytics cross-tenant IDOR (#2946) — seed two orgs and assert org A's metrics never include org B's properties | — | — | open |
| high | pm-qa | Write a cross-tenant regression test for violations add_evidence (#3164) that drives the real route as a user from another org and expects 403/404; cover dispute add_evidence (#2483/#2490) in the same sweep using the #3338-#3340 inquiry-idor pattern | — | — | open |
| high | pm-scrum-master | Run a full coverage scan (scan_kind=full) and plan a new sprint — current sprint 'Epic 6, 7A, 8A & 10A' is 4/4 done and 49/49 stories are done in coverage, so the candidate pipeline is exhausted | — | — | open |
| high | pm-scrum-master | Re-verify carried blockers #3187 (failing dev test), #3168 (cargo-deny advisories), #3131 (merge pipeline deadlock) against live gh state — project-state.md was dated 2026-10-28 | — | — | open |
| high | pm-scrum-master | Unblock the 3 stalled dispatcher claims (quarantined PR #3294 refresh-token-localstorage security, PR #3301 inquiry dead-FK classify review, PR #3333 screen-map drift review) — each to be reviewed and merged or released | — | — | open |
| high | pm-scrum-master | Sweep remaining violations mutation handlers for the 403-vs-404 cross-tenant contract (#3187 revealed assign still returns 403) — pattern audit across moderation.rs/aml_dsa.rs after PR #3183 fixed most but not assign | — | UNRESOLVED:pm-security | open |
| medium | pm-backend | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded | — | — | open |
| medium | pm-backend | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on every real /inquiries + /realtors/inquiries call | — | — | open |
| medium | pm-frontend | Adopt eslint-plugin-react-hooks + i18next/no-literal-string in frontend/apps/mobile (blocking, warn-only for one sprint) — closes the chronic class-of-defect behind the carried mobile-rn lint-gap risk (ranked-ballot + i18n fallback bugs still open 83 days later) | — | UNRESOLVED:pm-qa | open |
| medium | pm-qa | Land eslint-plugin-react-hooks and no-literal-string gates on frontend/apps/mobile as warn-only; add submit-wiring smoke tests for mobile-rn ranked-ballot and other fake-submit screens asserting the mock API is called with the expected payload | — | — | open |
| medium | pm-qa | Un-quarantine the dispute KPI test (#2575) and add window_start <= window_end negative cases (equal, inverted, missing bounds) plus a shape-assertion for the response | — | — | open |
| medium | pm-scrum-master | Re-triage the 72 backlog items to classify dep-poisoned vs mobile-native-gated; decide whether to unpoison, defer, or drop. Target honest-claimable >= 10 or document a dispatcher pause | — | — | open |
| medium | pm-scrum-master | Resolve the mobile-native gate rollup — close #2652 KMP cloud builds and #2951 RN jest-expo, or re-scope the gated items to be non-gated | — | — | open |
| low | pm-backend | Shared Ktor HttpClient has no HttpTimeout - mobile API calls can hang indefinitely | — | — | open |
| low | pm-backend | Reality mobile MyListings + portfolio analytics silently cap at 100 listings (no pagination) | — | — | open |
| low | pm-backend | KMP realtor SSO error-body parsing is unguarded — crashes on non-JSON response | — | — | open |
| low | pm-security | SECURITY: ppt-web refresh-token persisted in localStorage — XSS-reachable | — | — | open |
| low | pm-tech-lead | InquiriesHandler splits ListingNotFound vs RealtorNotFound by error substring, but both collapse to the same 404 at the route - dead, brittle classification | — | — | open |
| low | pm-scrum-master | Spot-check PR #3351 (registry i18n): confirm 6 locale files have key parity and the screen-map entries for registry screens were updated per the repo protocol; /screens validate passes | — | — | open |
| low | pm-qa | Screen-map drift: PR #3293 i18n'd reality-web realtor/[id] page without updating docs/screens/reality/realtor-detail.md | — | — | open |
| medium | pm-tech-lead | Follow-up: realtor public profile page renders hardcoded Slovak UI on a 6-language portal (PR #3225) (Closes #3283) | — | — | in-progress |
| medium | pm-tech-lead | Follow-up: member-role escalation gate reimplements role parsing instead of reusing common::TenantRole::from_role_type (PR #3291) (Closes #3304) | — | — | in-progress |
| medium | pm-tech-lead | Follow-up: fold IoT org-scope check into the insert on the hot add_reading path instead of a separate SELECT EXISTS (PR #3289) (Closes #3305) | — | — | in-progress |
| medium | pm-tech-lead | Follow-up: consolidate per-file require_manager(rls) helpers onto the shared rls_authz::require_manager_rls (PR #3287) (Closes #3306) | — | — | in-progress |
| medium | pm-tech-lead | Follow-up: fault update/triage/status handlers manually guard org on an id-only query instead of using find_by_id_for_org (PR #3286) (Closes #3307) | — | — | in-progress |
| medium | pm-tech-lead | Follow-up: add RLS-aware repo variants for vote sub-resource reads instead of pool-bypass + manual vote_in_org guard (PR #3281) (Closes #3308) | — | — | in-progress |
| medium | pm-tech-lead | Follow-up: cs.json/sk.json were reordered (~9k-line churn) instead of appended; enforce stable key order for messages/*.json (PR #3260) (Closes #3309) | — | — | in-progress |
| medium | pm-tech-lead | Follow-up: community feed shows every author as "Neighbor" (API returns ids only); formatMoney coerces Decimal via Number() (PR #3269) (Closes #3310) | — | — | in-progress |
| low | pm-tech-lead | ppt-web registry feature (pet/vehicle forms + lists) ships hardcoded English placeholders in a 4-locale app — 0 useTranslation on 4 files | — | — | in-progress |
| low | pm-backend | reality-web shared-comparison loader casts the listings API body with an unchecked `as Promise<ListingSummary>` — no shape validation, unlike the rest of reality-web's boundary guards (parseJsonBody / parseListingDetail) | — | — | in-progress |
