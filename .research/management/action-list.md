# PPT Action list

_Regenerated from action-list.json on 2026-11-08. 14 open · 12 in-progress · 0 done · 26 total._

| Priority | Owner | Action | Depends on | Status |
|---|---|---|---|---|
| high | pm-devops | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — still 5+ open backlog items structurally unclaimabl | — | open |
| high | pm-scrum-master | Unblock dispatcher pipeline — 3 approved PRs stuck on non-mechanical conflicts; triage Tier-1d retirement decision (gh-i | — | open |
| high | pm-scrum-master | Sweep remaining violations mutation handlers for the 403-vs-404 cross-tenant contract (#3187 revealed assign still retur | UNRESOLVED:pm-security | open |
| medium | pm-frontend | ppt-web RegistryPage.tsx delete/review mutation handlers (L102-142, L171-211) swallow failures with only console.error — | — | in-progress |
| medium | pm-tech-lead | Follow-up: realtor public profile page renders hardcoded Slovak UI on a 6-language portal (PR #3225) (Closes #3283) | — | in-progress |
| medium | pm-tech-lead | Follow-up: member-role escalation gate reimplements role parsing instead of reusing common::TenantRole::from_role_type ( | — | in-progress |
| medium | pm-tech-lead | Follow-up: fold IoT org-scope check into the insert on the hot add_reading path instead of a separate SELECT EXISTS (PR  | — | in-progress |
| medium | pm-tech-lead | Follow-up: consolidate per-file require_manager(rls) helpers onto the shared rls_authz::require_manager_rls (PR #3287) ( | — | in-progress |
| medium | pm-tech-lead | Follow-up: fault update/triage/status handlers manually guard org on an id-only query instead of using find_by_id_for_or | — | in-progress |
| medium | pm-tech-lead | Follow-up: add RLS-aware repo variants for vote sub-resource reads instead of pool-bypass + manual vote_in_org guard (PR | — | in-progress |
| medium | pm-tech-lead | Follow-up: cs.json/sk.json were reordered (~9k-line churn) instead of appended; enforce stable key order for messages/*. | — | in-progress |
| medium | pm-tech-lead | Follow-up: community feed shows every author as "Neighbor" (API returns ids only); formatMoney coerces Decimal via Numbe | — | in-progress |
| medium | pm-backend | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded | — | open |
| medium | pm-backend | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on ever | — | open |
| medium | pm-frontend | Adopt eslint-plugin-react-hooks + i18next/no-literal-string in frontend/apps/mobile (blocking, warn-only for one sprint) | UNRESOLVED:pm-qa | open |
| medium | pm-qa | Harden ppt-web rentals.connections.test.tsx confirmation-dialog findByRole flake (surfaced on PR #3356 verify under para | — | open |
| medium | pm-qa | Add integration test matrix for POST /api/v1/auth/password/change session-revocation (PR #3353): current session kept, o | — | open |
| low | pm-tech-lead | ppt-web registry feature (pet/vehicle forms + lists) ships hardcoded English placeholders in a 4-locale app — 0 useTrans | — | in-progress |
| low | pm-frontend | ppt-web RegistryPage.tsx (L249-250,275,302,317-318) hardcodes English (title, subtitle, Pets/Vehicles tabs, load-error)  | — | in-progress |
| low | pm-backend | reality-web shared-comparison loader casts the listings API body with an unchecked `as Promise<ListingSummary>` — no sha | — | in-progress |
| low | pm-backend | Shared Ktor HttpClient has no HttpTimeout - mobile API calls can hang indefinitely | — | open |
| low | pm-backend | Reality mobile MyListings + portfolio analytics silently cap at 100 listings (no pagination) | — | open |
| low | pm-backend | KMP realtor SSO error-body parsing is unguarded — crashes on non-JSON response | — | open |
| low | pm-security | SECURITY: ppt-web refresh-token persisted in localStorage — XSS-reachable | — | open |
| low | pm-tech-lead | InquiriesHandler splits ListingNotFound vs RealtorNotFound by error substring, but both collapse to the same 404 at the  | — | open |
| low | pm-qa | Screen-map drift: PR #3293 i18n'd reality-web realtor/[id] page without updating docs/screens/reality/realtor-detail.md | — | open |
