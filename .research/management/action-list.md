# Action list

_Generated: 2026-07-07T10:30:00Z_

| ID | Priority | Owner | Action | Status | Deps |
|----|----------|-------|--------|--------|------|
| `pm-security-confirm-487-mfa-e2e-rate-limit-coverage-` | high | pm-security | Confirm #487 MFA e2e rate-limit coverage is real and compiles before treating the 10a-1 auth-foundation gate as clear | open | - |
| `pm-security-re-verify-481-oauth-refresh-token-revoca` | high | pm-security | Re-verify #481 (OAuth refresh-token revocation) against current oauth.rs/services/oauth.rs — code already filters revoked_at IS NULL and implements token-family reuse revocation; confirm oauth_integra | open | - |
| `pm-security-re-verify-482-protectedroute-tenants-0-f` | medium | pm-security | Re-verify #482 (ProtectedRoute tenants[0] fallback) — AuthContext.tsx deriveActiveRole() and dedicated ProtectedRoute.test.tsx already implement/cover the highest-privilege fix; confirm coverage is su | open | - |
| `pm-security-security-review-2125-listingdetail-favor` | medium | pm-security | Security-review #2125 (ListingDetail favorite-toggle rollback after logout, PR #2115) for token-lifecycle risk | open | - |
| `pm-security-sweep-other-rbac-checks-documents-announ` | medium | pm-security | Sweep other RBAC checks (documents/announcements/notifications routes) for the JWT-claim-vs-DB-role mismatch pattern that #2120/#2107 fixed | open | - |
| `83-3-portal-webhooks` | low | pm-backend | Coverage gap [phase4]: Real Estate Portal Webhooks — verify and finish to done. Gaps: Implemented and labeled under Epic 105 'Portal Syndication / Story 105.4' in code; functionally covers this story' | open |  |
| `churn-hotspot-backend-crates-db-src-repositories-reality-portal-rs` | low | pm-backend | Churn hotspot: backend/crates/db/src/repositories/reality_portal.rs (+59/−28 in PR #1297 PAP-142 IDOR scoping) | open |  |
| `pm-security-formalize-the-deny-toml-rustsec-ignore-l` | low | pm-security | Formalize the deny.toml RUSTSEC ignore-list into a tracked supply-chain policy doc with owners/timelines | open | - |
| `gh-issue-2141` | high | pm-security | cargo-deny advisories FAILED on dev: RUSTSEC-2026-0204 (crossbeam-epoch) (Closes #2141) | in-progress |  |
| `79-3-error-handling-toasts` | medium | pm-frontend | Coverage gap [mvp]: Error Handling and Toast Notifications — verify and finish to done. Gaps: no screen-map (orphan epic) | in-progress |  |
| `80-2-dispute-filing-flow` | medium | pm-frontend | Coverage gap [mvp]: Dispute Filing Flow — verify and finish to done. Gaps: Redesigned 5-step wizard (redesignStatus: in-progress) not shipped — only single-page form is live; Localized `disputes.draft | in-progress |  |
| `gh-issue-2076` | medium | pm-tech-lead | Follow-up: add DB-round-trip regression test pinning lease.rs units-join drift (PR #2060) (Closes #2076) | in-progress |  |
| `gh-issue-2121` | medium | pm-tech-lead | Follow-up: add non-manager deny test for outage mutations (PR #2120) (Closes #2121) | in-progress |  |
| `gh-issue-2122` | medium | pm-tech-lead | Follow-up: get_accounting_metrics positional tuple still allows same-type transposition (PR #2117) (Closes #2122) | in-progress |  |
| `gh-issue-2123` | medium | pm-tech-lead | Follow-up: enlarge tie group in include_system pagination test (PR #2116) (Closes #2123) | in-progress |  |
| `gh-issue-2124` | medium | pm-tech-lead | Follow-up: enum-sync ALL completeness is not compiler-enforced (PR #2113) (Closes #2124) | in-progress |  |
| `gh-issue-2125` | medium | pm-tech-lead | Follow-up: test ListingDetail updateAuth() + guard favorite-toggle rollback after logout (PR #2115) (Closes #2125) | in-progress |  |
| `gh-issue-2127` | medium | pm-tech-lead | Follow-up: deny.toml CODEOWNERS gate is advisory-only; add CI ban-presence check (PR #2111) (Closes #2127) | in-progress |  |
| `gh-issue-2128` | medium | pm-tech-lead | Follow-up: exec-bit gate only blocks if wired as a required status check on dev (PR #2119) (Closes #2128) | in-progress |  |
| `gh-issue-2129` | medium | pm-tech-lead | Follow-up: mobile RN screen wiring — type drift + misleading Leases/Forms status + polish (PR #2118) (Closes #2129) | in-progress |  |
| `churn-hotspot-backend-crates-db-tests-form-rls-repo-tests-rs` | low | pm-backend | Churn hotspot: backend/crates/db/tests/form_rls_repo_tests.rs touched 2x since 2026-06-12 (window 2026-06-12→2026-06-13) | in-progress |  |
| `churn-hotspot-backend-crates-integrations-src-booking-rs` | low | pm-backend | Churn hotspot: backend/crates/integrations/src/booking.rs (+404/−29 in PR #1294 Booking.com OTA retry) | in-progress |  |
| `code-review-api-core-osrng-expect` | low | pm-tech-lead | crypto.rs:127 SysRng.try_fill_bytes(...).expect() panics if OS CSPRNG errors during integration-credential encrypt | in-progress |  |
| `refactor-churn-hotspot-mobile-announcements-test` | low | pm-tech-lead | Churn hotspot: AnnouncementsScreen.test.ts — 4 PRs this run, instability proxy | in-progress |  |
| `test-gap-hotfix-no-test-pr-959-reality-listings-pagination` | low | pm-qa | Reality-server listings pagination clamp (PR #959) shipped without a regression test for limit=-1 | in-progress |  |
