# pm-security — 2026-11-16

<sub>Generated: 2026-11-16T03:12:00Z — Phase 1.6 rotating role analysis</sub>

## Summary

No auth/JWT changes in the last 5 backend commits. The only security-relevant change is #3554, which added an owner check on the null-unit delegation path in `routes/delegations.rs`, and it needs a sibling-path audit. Epic 10A OAuth gates (#481, #482) are closed. The `gh --label security` query failed (GraphQL blocked in this environment), so merged security-label coverage is unverified.

## Findings

- #3554 (delegations.rs) closes an authorization gap on the null-unit path. Other handlers that accept an optional `unit_id` or null scope (delegations list/revoke/accept, neighbors) may share the same pattern. The one test added is a happy-path suite, so negative cross-owner coverage is unconfirmed.
- The open deep-link pending cross-user singleton (mobile-rn, score 2) can leak one user's pending deep-link or auth-flow target to the next user on a shared device after logout. It is a session-isolation issue and should be fixed before the next mobile release.
- The open KMP SSO error-body-unguarded item (score 2) risks crashes or leakage of server error text on malformed SSO responses. It is paired with #3560 (gdpr raw DB error leak, still in review), so error-sanitization should be checked across the SSO and GDPR routes.

## Risks

| Probability | Impact | Risk | Mitigation |
|-------------|--------|------|------------|
| medium | high | Further RLS or owner-check gaps remain in delegation-adjacent routes, enabling cross-tenant access | Sibling-path audit and negative authz tests |
| medium | medium | Raw DB or internal error leakage in API responses (#3560 pattern) elsewhere | Central error-mapping review of routes returning raw errors |
| low | medium | Mobile cross-user deep-link state on shared devices | Reset the singleton on logout or user switch |

## Decisions needed

- Treat #3560 (gdpr error leak) as a release blocker until it is merged — owner: `pm-security`
- Raise the deep-link and SSO backlog items from score 2 to this sprint's priority — owner: `pm-orchestrator`

## Open questions

- Is the `security` label applied to PRs? The `gh` query could not run, so please re-check via REST (`gh api`).
- Does the #3554 fix cover every delegation mutation endpoint, or only the one patched?

## Metrics

- security_prs_merged_seen: 3
- open_security_backlog: 2
- auth_jwt_changes: 0
- files_read: 1
