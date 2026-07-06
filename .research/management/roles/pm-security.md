# pm-security — 2026-07-06

_Rotating role output for the daily PPT research routine. Rendered from the pm-security agent JSON._

## Summary

Wins this sprint: quick-xml XXE ban is now a hard cargo-deny gate with CODEOWNERS protection (#2096 + #2111), and the accounting-export honesty invariant is enforced by construction (#2099) — both close real CVE/compliance exposure. Top open items: PR **#1797** (OCR auth + rental-guest-PII gate) is a **13-day stale draft on a security-critical surface**, test-hardening batch item **#481** (OAuth refresh-token revocation bypass) still blocks 10a-1/10a-3, and **#2107**'s fabricated-JWT test bypass masks real auth-role drift — none of these should ship to prod unresolved.

## Next actions

| priority | action | dependency | done-when |
|---|---|---|---|
| high | Promote PR #1797 out of draft, get it reviewed and merged (closes #1772 unauthenticated-OCR, #1766 guest-PII exposure) | rust-backend | PR #1797 merged to dev; #1772 and #1766 closed with regression tests |
| high | Fix issue #2107: outages happy-path tests must exercise real login/authz instead of a fabricated JWT with mismatched role | rust-backend | Test suite derives JWT via real login flow; DB-role/JWT-role parity asserted |
| high | Resolve test-hardening #481 (refresh-token revocation bypass) before promoting 10a-1/10a-3 to done | rust-backend | `revoked_at IS NULL` check restored in refresh-token query + regression test; story_gate cleared |
| high | Complete 79-2 auth-flow sign-off (SSO/JWT/cookie) — currently the top pending pm-security action per coverage.json | none | `auth.rs` / `sso.rs` reviewed end-to-end for token TTL, cookie Path/scope, session revocation; sign-off recorded in coverage.json |
| medium | Add PKCE + refresh-rotation + introspection tests for OAuth Provider (10a-1/10a-2/10a-3) | rust-backend | `pm-security-oauth-10a-untested-security-contract` risk closed with test coverage |
| medium | Assess Dependabot #2018 (`aes-gcm` 0.10.3 → 0.11.0 major bump) for breaking changes on any auth/crypto call sites | rust-backend | Impact note posted on PR #2018; merge or defer decision made |

## Risks

| id | risk | prob | impact | mitigation |
|---|---|---|---|---|
| pr-1797-stale-security-draft | PR #1797 (OCR auth + guest PII gate) remains an unmerged draft for 13+ days on a known-critical surface | high | high | Prioritize review/merge this sprint; escalate if blocked |
| oauth-refresh-revocation-bypass | Test-hardening #481: OAuth refresh-token revocation query bypass — revoked tokens may still be reusable (RFC 9700 violation) | medium | high | Restore `revoked_at IS NULL` predicate + regression test before 10a promotion |
| fabricated-jwt-test-bypass | #2107 fabricated-JWT test bypass could mask real authz breakage in outages happy-path tests | medium | medium | Rework tests to use real login/authz path |
| oauth-10a-no-security-tests | OAuth Provider (10a) ships without PKCE/refresh-rotation/introspection test coverage | medium | high | Land dedicated security-contract tests before epic-10a promotion to done |
| audit-hash-debug-format-p1-04 | Debug-format audit-hash issue (P1-04) still open — potential sensitive-data leakage via Debug logging on an audit-hash surface | low | medium | Confirm status and close or explicitly defer with owner |

## Open questions

- Is PR #1797's auth fix already partially present on dev — does #1797 now only cover the remaining rental-guest-PII gate?
- What is the status of test-hardening items #480 (WS auth token logged in query param) and #487 (MFA rate-limit test gap) relative to epic-10a promotion?
- Is the cookie Path/scope hardening residual from a prior sprint closed, or still pending as part of 79-2 sign-off?
- Has Dependabot #2018 (aes-gcm major bump) been triaged for any direct use in token/session encryption paths?
- What is the current state/owner of Booking.com OAuth atomic credential swap on re-connect risk?

## Decisions needed

- Whether epic-10a (OAuth Provider) can proceed toward done without closing #481/#487 test-hardening gates — owner: rust-backend / pm-security.
- Whether PR #1797 should be split (OCR auth fix vs. guest-PII gate) to unblock the already-complete portion sooner — owner: rust-backend.
