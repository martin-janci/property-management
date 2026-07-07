# pm-security — 2026-07-07

_Rotating role this run (pm_cursor idx 5). Static read; no compile/run._

## Summary

Epic 10A OAuth Provider stories (10a-1/10a-2/10a-3) are still `ready-for-dev` per `sprint-status.yaml`, gated by test_hardening_batch #481/#482/#487, but code inspection shows the #481 refresh-token revocation fix and #482 ProtectedRoute role fix already appear implemented and tested on `dev` — the tracker looks stale and should be re-verified before treating Epic 10A as blocked. Supply-chain gates (RUSTSEC crossbeam-epoch bump, quick-xml XXE 3-layer ban/CODEOWNERS/CI-assertion) worked as designed same-day, but the broader RUSTSEC ignore-list in `deny.toml` is ad-hoc reachability reasoning rather than a formal policy, and #2125's logout/favorite-toggle token race remains unverified for session-lifecycle leakage.

## Next actions

1. **[high]** Re-verify #481 (OAuth refresh-token revocation) against current `oauth.rs`/`services/oauth.rs` — code already filters `revoked_at IS NULL` and implements token-family reuse revocation; confirm `oauth_integration_tests.rs` exercises this, then close/defer the gate to unblock 10a-1/10a-3. — deps: rust-backend
2. **[medium]** Re-verify #482 (`ProtectedRoute tenants[0]` fallback) — `AuthContext.tsx` `deriveActiveRole()` and dedicated `ProtectedRoute.test.tsx` already implement/cover the highest-privilege fix; confirm coverage is sufficient and close the gate for 10a-2. — deps: react-web
3. **[high]** Confirm #487 MFA e2e rate-limit coverage is real and compiles (issue flagged both missing brute-force tests and a possible non-compiling nested `mod common`) before treating the 10a-1 auth-foundation gate as clear. — deps: rust-backend
4. **[medium]** Security-review #2125 (ListingDetail favorite-toggle rollback after logout, PR #2115) for token-lifecycle risk: confirm the toggle mutation doesn't fire with a stale captured token post-logout and that favorite UI state clears on logout rather than leaking prior-session state. — deps: react-web
5. **[low]** Formalize the `deny.toml` RUSTSEC ignore-list (rustls-webpki via AWS SDK/rustls 0.21, rsa Marvin timing via jsonwebtoken, lopdf stack overflow) into a tracked supply-chain policy doc with owners/timelines instead of only in-file comments. — deps: rust-backend
6. **[medium]** Sweep other RBAC checks (documents/announcements/notifications routes touched this sprint) for the JWT-claim-vs-DB-role mismatch pattern that #2120/#2107 fixed for outage mutations. — deps: rust-backend

## Risks

- **[med prob / med impact]** `sprint-status.yaml` test_hardening_batch may be stale — #481/#482 look already fixed in code but still gate Epic 10A as `ready-for-dev`, needlessly stalling the OAuth epic. Mitigation: run a verify pass against #470/#459 and their tests, then update the batch status explicitly.
- **[low prob / high impact]** If refresh-token replay handling is NOT actually covered by a passing regression test, revoked OAuth refresh tokens could be reusable (RFC 9700 violation) — a release blocker for the OAuth Provider epic. Mitigation: confirm `oauth_integration_tests.rs` asserts revoked-token replay triggers `revoke_token_family` and 401.
- **[med prob / high impact]** MFA brute-force/rate-limit protection unverified pending #487 — if the e2e suite doesn't compile/run in CI, credential-stuffing protection for the auth foundation is unconfirmed. Mitigation: fix/confirm `mfa_e2e_tests.rs` compiles and asserts lockout behavior before Epic 10A ships.
- **[low prob / med impact]** #2125 favorite-toggle-after-logout may indicate a broader pattern of components in reality-web holding stale token/auth references across the logout boundary. Mitigation: trace the actual hook and add a logout-boundary test.
- **[low prob / med impact]** Supply-chain ignore-list correctness depends on "not currently reachable" assumptions tied to large undone migrations (AWS SDK rustls 0.23, `jsonwebtoken` backend) with no committed date. Mitigation: track each ignored RUSTSEC ID with an owner and revisit if new code paths touch RSA or PDF parsing.

## Open questions

- Are #481 and #482 already closed by the code found in `oauth.rs` / `services/oauth.rs` / `AuthContext.tsx` + `ProtectedRoute.test.tsx`, or is `sprint-status.yaml` intentionally holding them open pending a formal verify pass?
- Does `mfa_e2e_tests.rs` currently compile and pass in CI (the nested `mod common` compile concern from #487), and is the rate-limit/lockout coverage found (3 hits) sufficient?
- Which component/hook actually owns the favorite-toggle mutation + logout race for #2125 — it wasn't in `ListingCard.tsx`/`ListingDetailContent.tsx` as expected from the issue title?
- Is there a committed owner/timeline for the AWS SDK rustls 0.23 migration and `jsonwebtoken` RSA-backend swap that the `deny.toml` ignore-list depends on staying non-reachable?
- Was the JWT-role-vs-DB-role mismatch pattern (#2107/#2120) unique to outage mutations, or does it also affect other Epic 6/7A/8A route authorization checks reviewed this sprint?

## Decisions needed

- Whether to re-verify and close #481/#482 today to unblock Epic 10A stories from `ready-for-dev`, or keep the gate open pending a dedicated QA pass — owner: pm-tech-lead
- Whether #2125's token/session-lifecycle angle needs a formal security sign-off before merge or can proceed as a UX-only fix — owner: pm-security
- Whether to invest in formalizing the supply-chain/SCA ignore-list into a tracked policy doc now, or defer given it's currently working via cargo-deny CI gates — owner: pm-tech-lead
