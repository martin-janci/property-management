# Action List

<sub>Generated: 2026-11-09 06:45 UTC by pm Phase 1.6 (pm-scrum-master + pm-qa)</sub>

| Status | Owner | Priority | Action | Depends on |
|--------|-------|----------|--------|------------|
| open | pm-devops | high | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — still 5+ open backlog items structurally unclaimable in cloud; now compounded by #2966 so claimable=9/72 (buffer-low) |  |
| open | pm-backend | medium | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded |  |
| open | pm-backend | medium | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on every real /inquiries + /realtors/inquiries call |  |
| open | pm-backend | low | Shared Ktor HttpClient has no HttpTimeout - mobile API calls can hang indefinitely |  |
| open | pm-backend | low | Reality mobile MyListings + portfolio analytics silently cap at 100 listings (no pagination) |  |
| open | pm-frontend | medium | Adopt eslint-plugin-react-hooks + i18next/no-literal-string in frontend/apps/mobile (blocking, warn-only for one sprint) — closes the chronic class-of-defect behind the carried mobile-rn lint-gap risk (ranked-ballot + i18n fallback bugs still open 83 days later) | UNRESOLVED:pm-qa |
| open | pm-scrum-master | high | Sweep remaining violations mutation handlers for the 403-vs-404 cross-tenant contract (#3187 revealed assign still returns 403) — pattern audit across moderation.rs/aml_dsa.rs after PR #3183 fixed most but not assign | UNRESOLVED:pm-security |
| open | pm-backend | low | KMP realtor SSO error-body parsing is unguarded — crashes on non-JSON response |  |
| open | pm-security | low | SECURITY: ppt-web refresh-token persisted in localStorage — XSS-reachable |  |
| open | pm-scrum-master | high | Promote a non-mobile-native code-review vector from the next Phase 1.5 segment (ppt-web-core / reality-server oldest = 2026-11-04) to refill the planner buffer |  |
| open | pm-scrum-master | high | Triage #3357 — rebase the 3 blocked PRs OR adjust Tier-1d signal sourcing to stop emitting dead-end vectors |  |
| open | pm-scrum-master | medium | Open the diff for code-review-ppt-web-core-refresh-token-localstorage and upgrade signal confidence from medium to high (or drop) — currently blocks security fast-track at score=2+confidence=medium |  |
| open | pm-qa | medium | Add a dispatcher-self-test.sh test case that covers the quarantined-row recovery path so #3357-class stalls fail CI instead of silently queueing |  |
| open | pm-qa | medium | QA audit: non-mobile-native backlog items with confidence=medium need a one-shot verification step before Phase 3 can promote them |  |
