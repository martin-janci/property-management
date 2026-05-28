# pm-qa — QA / Test lens (2026-05-25)

_Daily rotation run. Read-only static analysis of sprint-status + merged PRs._

## Summary

The sprint carries an unmitigated regression-test gap on a shipped security fix: **PR #497 (inquiry IDOR `mark_as_read` scoping)** merged with the three acceptance-criteria TODOs all unchecked and **no test file** under `backend/servers/reality-server/tests/`. By contrast PR #493 (equipment IDOR) shipped *with* `equipment_cross_tenant_idor_tests.rs` — the precedent #497 should have followed. Separately, eight open test-hardening issues (#480–#487), two high-severity (#480 JWT in logs, #481 OAuth revoked-token bypass), gate six sprint stories from `done`.

## Next actions

| Priority | Action | Owner | DoD |
|---|---|---|---|
| high | Create `reality-server/tests/inquiry_idor_tests.rs`: B marks A's inquiry → 404 & `read_at` NULL; A marks own → 204 & set; idempotent re-mark → 204 | rust-backend | 3 scenarios pass in CI; PR references #497 |
| high | Close/defer #480 (JWT in logs) & #481 (OAuth revocation bypass) before 8a-3/10a-1/10a-3 promote | rust-backend | Issues closed or formally deferred; story gates updated |
| high | Verify `respond_to_inquiry` enforces realtor ownership at repo layer; add cross-realtor 4xx test if unproven | rust-backend | Ownership confirmed in SQL or test added |
| medium | Resolve Story 6-1 AC-1 ambiguity (notification clause vs deferred Task 4.3 / Epic 2B) | pm-feature | AC-1 amended; reviewer sign-off |
| medium | Track closure of #482/#485/#486 before 10a-2/7a-5/6-2/6-5 promote | react-web | Issues closed or deferred |
| medium | Add voice-device IDOR integration test (#483): cross-tenant list-commands → 403, not empty list | rust-backend | Test asserts 403; #483 closed |

## Risks

- **inquiry-idor-no-regression-test** (med prob / high impact): #497 fix has no regression coverage; a future refactor of the EXISTS-then-UPDATE pattern could silently reintroduce the IDOR.
- **oauth-revoked-token-bypass-open** (#481, high/high): revoked refresh tokens reusable; do not promote 10a-1/10a-3 while open.
- **jwt-token-in-ws-logs-open** (#480, high/high): JWT in WS query-param access logs; blocks 8a-3.
- **story-6-1-ac-mismatch** (med/med): AC-1 claims users notified but dispatch deferred to Epic 2B.
- **thb-no-owners** (med/med): #480–#487 have no assigned owners in sprint-status.

## Decisions needed

- Block sprint release on #497 IDOR test gap, or accept risk with a time-boxed test commitment? (owner: tech-lead)
- Formal deferral vs same-sprint resolution for #480/#481 before 10a-1/10a-3/8a-3 promote? (owner: pm-feature + tech-lead)
- Adopt a PR-merge policy requiring a test file for every security-fix PR (mirroring #493)? (owner: tech-lead)
